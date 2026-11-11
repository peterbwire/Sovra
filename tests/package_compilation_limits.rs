//! Cumulative compilation quotas without executing untrusted package input.

use sovra::compiler::{
    ir::{Instruction, IrProgram, Literal},
    parser::Parser,
    project::packages::{
        compile, compile_with_compilation_limits, compile_with_graph_limits,
        compile_with_source_limit,
    },
    resources::{
        check_ast_budget, LoweringLimits, PackageCompilationLimits, PackageGraphLimits,
        DEFAULT_PACKAGE_SOURCE_BYTES,
    },
};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

const LIBRARY: &str = r#"
mod api {
    export struct Item { value: Int }
    export fn make(value: Int) -> Item { return Item { value: value }; }
    export fn read(item: Item) -> Int { return item.value; }
}
"#;
const BODY: &str = "fn main() { print(b::api::read(a::api::make(7))); }";

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "svr-package-quotas-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(root.join("app")).unwrap();
        std::fs::create_dir_all(root.join("lib")).unwrap();
        std::fs::write(root.join("app/sovra.toml"), "[project]\nname = \"app\"\nentry = \"main.svr\"\n[dependencies.a]\npath = \"../lib\"\n[dependencies.b]\npath = \"../lib\"\n").unwrap();
        std::fs::write(
            root.join("lib/sovra.toml"),
            "[project]\nname = \"lib\"\nentry = \"main.svr\"\n",
        )
        .unwrap();
        std::fs::write(root.join("lib/main.svr"), LIBRARY).unwrap();
        std::fs::write(
            root.join("app/main.svr"),
            format!("use a::api;\nuse b::api;\n{BODY}"),
        )
        .unwrap();
        Self(root)
    }
    fn app(&self) -> PathBuf {
        self.0.join("app")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn unlimited() -> PackageCompilationLimits {
    PackageCompilationLimits {
        max_ast_nodes: usize::MAX,
        max_interface_nodes: usize::MAX,
        lowering: LoweringLimits {
            max_instructions: usize::MAX,
            max_string_bytes: usize::MAX,
        },
    }
}

fn payload(ir: &IrProgram) -> (usize, usize) {
    let mut instructions = 0;
    let mut bytes = 0;
    for function in &ir.functions {
        bytes += function.name.len() + function.parameters.iter().map(String::len).sum::<usize>();
        for instruction in &function.instructions {
            instructions += 1;
            bytes += match instruction {
                Instruction::LoadLiteral(
                    Literal::Integer(text) | Literal::Float(text) | Literal::String(text),
                )
                | Instruction::LoadName(text)
                | Instruction::StoreName(text)
                | Instruction::Binary(text)
                | Instruction::LoadField(text) => text.len(),
                Instruction::Call { name, .. } => name.len(),
                Instruction::MakeStruct { type_name, fields } => {
                    type_name.len() + fields.iter().map(String::len).sum::<usize>()
                }
                Instruction::LoadLiteral(Literal::Boolean(_))
                | Instruction::StoreIndex
                | Instruction::WidenFloat
                | Instruction::WidenFloatArray { .. }
                | Instruction::MakeArray { .. }
                | Instruction::Index
                | Instruction::Jump { .. }
                | Instruction::JumpIfFalse { .. }
                | Instruction::Return
                | Instruction::Pop => 0,
            };
        }
    }
    (instructions, bytes)
}

#[test]
fn cumulative_ast_counts_canonical_sources_once_and_locates_exhaustion() {
    let fixture = Fixture::new();
    let count = [LIBRARY, BODY]
        .iter()
        .map(|source| {
            check_ast_budget(&Parser::new().parse_source(source).unwrap(), usize::MAX).unwrap()
        })
        .sum();
    let limits = PackageCompilationLimits {
        max_ast_nodes: count,
        ..unlimited()
    };
    assert_eq!(
        compile_with_compilation_limits(fixture.app(), limits).unwrap(),
        compile(fixture.app()).unwrap()
    );
    let errors = compile_with_compilation_limits(
        fixture.app(),
        PackageCompilationLimits {
            max_ast_nodes: count - 1,
            ..limits
        },
    )
    .unwrap_err();
    assert_eq!(errors.items[0].code, "E3020");
    assert!(errors.items[0]
        .source_file
        .as_ref()
        .unwrap()
        .ends_with("main.svr"));
    assert!(errors.items[0]
        .message
        .contains(&count.saturating_sub(1).to_string()));
}

#[test]
fn repeated_aliases_charge_interface_copies_before_semantic_expansion() {
    let fixture = Fixture::new();
    // Library: exported record+field (2), two signatures+parameters (4).
    // Consumer: two signature sets (8), one attached record (2), two aliases
    // (2), and its exposed copy of the attached record (2): total 20.
    let limits = PackageCompilationLimits {
        max_interface_nodes: 20,
        ..unlimited()
    };
    assert!(compile_with_compilation_limits(fixture.app(), limits).is_ok());
    for max_interface_nodes in 0..20 {
        let errors = compile_with_compilation_limits(
            fixture.app(),
            PackageCompilationLimits {
                max_interface_nodes,
                ..limits
            },
        )
        .unwrap_err();
        assert_eq!(errors.items[0].code, "E3023");
        assert!(errors.items[0].source_file.is_some());
        let diagnostic = &errors.items[0];
        let source = std::fs::read_to_string(diagnostic.source_file.as_ref().unwrap()).unwrap();
        assert!(diagnostic.span.start <= diagnostic.span.end);
        assert!(diagnostic.span.end <= source.len(), "{diagnostic:?}");
        assert!(source.is_char_boundary(diagnostic.span.start));
        assert!(source.is_char_boundary(diagnostic.span.end));
        if max_interface_nodes >= 18 {
            assert_eq!(
                &source[diagnostic.span.start..diagnostic.span.end],
                "use a::api;"
            );
        }
    }
    assert!(compile_with_compilation_limits(fixture.app(), limits).is_ok());
}

#[test]
fn final_linked_payload_is_charged_across_packages_at_every_boundary() {
    let fixture = Fixture::new();
    let expected = compile(fixture.app()).unwrap();
    let (instructions, strings) = payload(&expected);
    assert_eq!(expected.functions.len(), 3);
    assert!(expected
        .functions
        .iter()
        .any(|function| function.name.starts_with("@package1::")));
    let limits = PackageCompilationLimits {
        lowering: LoweringLimits {
            max_instructions: instructions,
            max_string_bytes: strings,
        },
        ..unlimited()
    };
    assert_eq!(
        compile_with_compilation_limits(fixture.app(), limits).unwrap(),
        expected
    );
    for max_instructions in 0..instructions {
        let errors = compile_with_compilation_limits(
            fixture.app(),
            PackageCompilationLimits {
                lowering: LoweringLimits {
                    max_instructions,
                    ..limits.lowering
                },
                ..limits
            },
        )
        .unwrap_err();
        assert_eq!(errors.items[0].code, "E3021");
        assert!(errors.items[0].source_file.is_some());
    }
    for max_string_bytes in 0..strings {
        let errors = compile_with_compilation_limits(
            fixture.app(),
            PackageCompilationLimits {
                lowering: LoweringLimits {
                    max_string_bytes,
                    ..limits.lowering
                },
                ..limits
            },
        )
        .unwrap_err();
        assert_eq!(errors.items[0].code, "E3022");
    }
    assert_eq!(sovra::compiler::interpreter::run(&expected).unwrap(), ["7"]);
    // A failure in another invocation must not consume a later budget.
    assert_eq!(
        compile_with_compilation_limits(fixture.app(), limits).unwrap(),
        expected
    );
}

#[test]
fn checked_package_quota_path_still_rejects_invalid_bodies() {
    let fixture = Fixture::new();
    std::fs::write(
        fixture.0.join("lib/main.svr"),
        "mod api { export fn make() { print(missing); } }",
    )
    .unwrap();
    let errors = compile_with_compilation_limits(fixture.app(), unlimited()).unwrap_err();
    assert_eq!(errors.items[0].code, "E3001");
}

#[test]
fn zero_instruction_package_still_charges_entry_name_and_no_unused_dependencies() {
    let fixture = Fixture::new();
    std::fs::write(
        fixture.0.join("app/sovra.toml"),
        "[project]\nname = \"app\"\nentry = \"main.svr\"\n",
    )
    .unwrap();
    std::fs::write(fixture.0.join("app/main.svr"), "fn main() {}").unwrap();
    let limits = PackageCompilationLimits {
        max_ast_nodes: 1,
        max_interface_nodes: 0,
        lowering: LoweringLimits {
            max_instructions: 0,
            max_string_bytes: 4,
        },
    };
    let ir = compile_with_compilation_limits(fixture.app(), limits).unwrap();
    assert_eq!(payload(&ir), (0, 4));
    let errors = compile_with_compilation_limits(
        fixture.app(),
        PackageCompilationLimits {
            lowering: LoweringLimits {
                max_string_bytes: 3,
                ..limits.lowering
            },
            ..limits
        },
    )
    .unwrap_err();
    assert_eq!(errors.items[0].code, "E3022");
    let errors = compile_with_compilation_limits(
        fixture.app(),
        PackageCompilationLimits {
            max_ast_nodes: 0,
            ..limits
        },
    )
    .unwrap_err();
    assert_eq!(errors.items[0].code, "E3020");
}

#[test]
fn approved_interface_default_is_exact_and_explicit_overrides_still_work() {
    let fixture = Fixture::new();
    let defaults = PackageCompilationLimits::DEFAULT;
    assert_eq!(defaults.max_ast_nodes, 1_000_000);
    assert_eq!(defaults.max_interface_nodes, 250_000);
    assert_eq!(defaults.lowering.max_instructions, 4_000_000);
    assert_eq!(defaults.lowering.max_string_bytes, 256 * 1024 * 1024);
    // A function plus 99 parameters is 100 interface nodes per materialization.
    // Export validation (100) + 2,499 imports (249,900) exactly meets the default.
    let parameters = (0..99)
        .map(|index| format!("p{index}: Int"))
        .collect::<Vec<_>>()
        .join(", ");
    std::fs::write(
        fixture.0.join("lib/main.svr"),
        format!("mod api {{ export fn f({parameters}) {{}} }}"),
    )
    .unwrap();
    let exact = format!(
        "{} fn main() {{ print(42); }}",
        "use a::api;\n".repeat(2499)
    );
    std::fs::write(fixture.0.join("app/main.svr"), &exact).unwrap();
    assert!(compile(fixture.app()).is_ok());
    assert!(compile_with_graph_limits(fixture.app(), PackageGraphLimits::DEFAULT).is_ok());
    assert!(compile_with_source_limit(fixture.app(), DEFAULT_PACKAGE_SOURCE_BYTES).is_ok());
    std::fs::write(
        fixture.0.join("app/main.svr"),
        format!("use a::api;\n{exact}"),
    )
    .unwrap();
    for result in [
        compile(fixture.app()),
        compile_with_graph_limits(fixture.app(), PackageGraphLimits::DEFAULT),
        compile_with_source_limit(fixture.app(), DEFAULT_PACKAGE_SOURCE_BYTES),
    ] {
        assert_eq!(result.unwrap_err().items[0].code, "E3023");
    }
    let ir = compile_with_compilation_limits(
        fixture.app(),
        PackageCompilationLimits {
            max_interface_nodes: defaults.max_interface_nodes + 100,
            ..defaults
        },
    )
    .unwrap();
    assert_eq!(sovra::compiler::interpreter::run(&ir).unwrap(), ["42"]);
}

#[test]
fn approved_ast_default_counts_the_whole_graph() {
    let fixture = Fixture::new();
    // Two canonical units: (1 + 2*250,000) + (1 + 2*249,999) = 1,000,000.
    std::fs::write(
        fixture.0.join("app/main.svr"),
        format!("fn main() {{ {} }}", "1;".repeat(250_000)),
    )
    .unwrap();
    std::fs::write(
        fixture.0.join("lib/main.svr"),
        format!("fn helper() {{ {} }}", "1;".repeat(249_999)),
    )
    .unwrap();
    assert!(compile(fixture.app()).is_ok());
    std::fs::write(
        fixture.0.join("lib/main.svr"),
        format!("fn helper() {{ {} }}", "1;".repeat(250_000)),
    )
    .unwrap();
    let errors = compile(fixture.app()).unwrap_err();
    assert_eq!(errors.items[0].code, "E3020");
    assert!(errors.items[0].message.contains("1000000"));
}
