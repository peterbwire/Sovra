//! Import-aware checked package compilation preserves linking and cumulative limits.

use sovra::compiler::{
    checked::{FlowEvidenceLimits, LoweringEvidenceLimits, SemanticEvidenceLimits},
    interpreter,
    ir::{Instruction, IrProgram, Literal},
    project::packages::{
        compile_with_checked_evidence, compile_with_legacy_lowering, CheckedPackageError,
        PackageEvidenceLimits,
    },
    resources::PackageCompilationLimits,
};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

const LIBRARY: &str = "type Private = Float; mod api { export struct Item { value: Private } export fn make(value: Private) -> Item { return Item { value: value }; } export fn read(item: Item) -> Private { return item.value; } export fn widen(value: Private) -> Private { return value; } fn secret() {} } fn main() { print(\"dependency entry must not run\"); }";
const ROOT: &str = "use a::api; use b::api; type Private = Int; fn main() { let mut values = [0.0]; let value = a::api::widen(1); values[0] = value; let item = a::api::make(3); print(b::api::read(item)); print(values[0]); print(value / 2); }";
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "svr-checked-packages-{}-{}",
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
        std::fs::write(root.join("app/main.svr"), ROOT).unwrap();
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
fn evidence() -> PackageEvidenceLimits {
    PackageEvidenceLimits {
        max_nodes: 100000,
        semantic: SemanticEvidenceLimits {
            max_facts: 100000,
            max_types: 100000,
            max_name_bytes: 16 * 1024 * 1024,
        },
        flow: FlowEvidenceLimits {
            max_declarations: 100000,
            max_uses: 100000,
            max_coercions: 100000,
            max_name_bytes: 16 * 1024 * 1024,
        },
        lowering: LoweringEvidenceLimits {
            max_function_results: 100000,
            max_contexts: 100000,
        },
        max_imports: 10000,
        max_import_bytes: 16 * 1024 * 1024,
    }
}
fn consumed(a: PackageEvidenceLimits, b: PackageEvidenceLimits) -> PackageEvidenceLimits {
    PackageEvidenceLimits {
        max_nodes: a.max_nodes - b.max_nodes,
        semantic: SemanticEvidenceLimits {
            max_facts: a.semantic.max_facts - b.semantic.max_facts,
            max_types: a.semantic.max_types - b.semantic.max_types,
            max_name_bytes: a.semantic.max_name_bytes - b.semantic.max_name_bytes,
        },
        flow: FlowEvidenceLimits {
            max_declarations: a.flow.max_declarations - b.flow.max_declarations,
            max_uses: a.flow.max_uses - b.flow.max_uses,
            max_coercions: a.flow.max_coercions - b.flow.max_coercions,
            max_name_bytes: a.flow.max_name_bytes - b.flow.max_name_bytes,
        },
        lowering: LoweringEvidenceLimits {
            max_function_results: a.lowering.max_function_results - b.lowering.max_function_results,
            max_contexts: a.lowering.max_contexts - b.lowering.max_contexts,
        },
        max_imports: a.max_imports - b.max_imports,
        max_import_bytes: a.max_import_bytes - b.max_import_bytes,
    }
}

#[test]
fn checked_packages_preserve_imported_alias_record_identity_ir_and_relocation() {
    let fixture = Fixture::new();
    let old =
        compile_with_legacy_lowering(fixture.app(), PackageCompilationLimits::DEFAULT).unwrap();
    let new =
        compile_with_checked_evidence(fixture.app(), PackageCompilationLimits::DEFAULT, evidence())
            .unwrap();
    assert_eq!(new.ir, old);
    assert_eq!(new.units, 2); // Two aliases to the same canonical provider compile once.
    assert_eq!(evidence().max_imports - new.remaining.max_imports, 6);
    assert_eq!(interpreter::run(&new.ir).unwrap(), ["3", "1", "0.5"]);
    // Execute JavaScript generated from the checked package IR, not a legacy CLI build.
    use std::io::Write as _;
    use std::process::{Command, Stdio};
    let mut node = Command::new("node")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Node.js is required for backend execution tests");
    node.stdin
        .take()
        .unwrap()
        .write_all(sovra::compiler::backend::render_javascript(&new.ir).as_bytes())
        .unwrap();
    let output = node.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8(output.stdout)
            .unwrap()
            .replace("\r\n", "\n"),
        "3\n1\n0.5\n"
    );
    let relocated = Fixture::new();
    let other = compile_with_checked_evidence(
        relocated.app(),
        PackageCompilationLimits::DEFAULT,
        evidence(),
    )
    .unwrap();
    assert_eq!(other.ir, new.ir);
    assert_eq!(other.remaining, new.remaining);
    let example = compile_with_checked_evidence(
        "examples/local-packages/app",
        PackageCompilationLimits::DEFAULT,
        evidence(),
    )
    .unwrap();
    assert_eq!(
        example.ir,
        compile_with_legacy_lowering(
            "examples/local-packages/app",
            PackageCompilationLimits::DEFAULT
        )
        .unwrap()
    );
    std::fs::write(
        fixture.0.join("lib/main.svr"),
        LIBRARY.replace("type Private = Float", "type Private = Int"),
    )
    .unwrap();
    let refreshed =
        compile_with_checked_evidence(fixture.app(), PackageCompilationLimits::DEFAULT, evidence())
            .unwrap();
    assert_eq!(
        refreshed.ir,
        compile_with_legacy_lowering(fixture.app(), PackageCompilationLimits::DEFAULT).unwrap()
    );
    assert_ne!(refreshed.ir, new.ir);
    assert_eq!(interpreter::run(&refreshed.ir).unwrap(), ["3", "1", "0"]);
}

#[test]
fn evidence_is_cumulative_exact_and_atomic_across_units_and_alias_bindings() {
    let fixture = Fixture::new();
    let measured =
        compile_with_checked_evidence(fixture.app(), PackageCompilationLimits::DEFAULT, evidence())
            .unwrap();
    let exact = consumed(evidence(), measured.remaining);
    let result =
        compile_with_checked_evidence(fixture.app(), PackageCompilationLimits::DEFAULT, exact)
            .unwrap();
    assert_eq!(result.ir, measured.ir);
    let mut limits = Vec::new();
    macro_rules! less { ($($field:ident).+) => {{ let mut copy = exact; assert!(copy.$($field).+ > 0); copy.$($field).+ -= 1; limits.push(copy); }} }
    less!(max_nodes);
    less!(semantic.max_facts);
    less!(semantic.max_types);
    less!(semantic.max_name_bytes);
    less!(flow.max_declarations);
    less!(flow.max_uses);
    less!(flow.max_coercions);
    less!(flow.max_name_bytes);
    less!(lowering.max_function_results);
    less!(lowering.max_contexts);
    less!(max_imports);
    less!(max_import_bytes);
    for limit in limits {
        assert!(
            matches!(
                compile_with_checked_evidence(
                    fixture.app(),
                    PackageCompilationLimits::DEFAULT,
                    limit
                ),
                Err(CheckedPackageError::Evidence { .. })
            ),
            "{limit:?}"
        );
    }
    assert!(matches!(
        compile_with_checked_evidence(
            fixture.app(),
            PackageCompilationLimits::DEFAULT,
            PackageEvidenceLimits {
                max_imports: 0,
                ..evidence()
            }
        ),
        Err(CheckedPackageError::Evidence { .. })
    ));
    // Every failed attempt leaves source and subsequent successful compilation intact.
    assert_eq!(
        std::fs::read_to_string(fixture.0.join("app/main.svr")).unwrap(),
        ROOT
    );
    assert_eq!(
        compile_with_checked_evidence(fixture.app(), PackageCompilationLimits::DEFAULT, exact)
            .unwrap()
            .ir,
        measured.ir
    );
}

fn strings(ir: &IrProgram) -> usize {
    ir.functions
        .iter()
        .map(|f| {
            f.name.len()
                + f.parameters.iter().map(String::len).sum::<usize>()
                + f.instructions
                    .iter()
                    .map(|i| match i {
                        Instruction::LoadLiteral(
                            Literal::Integer(s) | Literal::Float(s) | Literal::String(s),
                        )
                        | Instruction::LoadName(s)
                        | Instruction::StoreName(s)
                        | Instruction::Binary(s)
                        | Instruction::LoadField(s) => s.len(),
                        Instruction::Call { name, .. } => name.len(),
                        Instruction::MakeStruct { type_name, fields } => {
                            type_name.len() + fields.iter().map(String::len).sum::<usize>()
                        }
                        _ => 0,
                    })
                    .sum::<usize>()
        })
        .sum()
}
#[test]
fn original_graph_interface_and_linked_ir_limits_keep_exact_diagnostics() {
    let fixture = Fixture::new();
    let original = PackageCompilationLimits::DEFAULT;
    let ir = compile_with_legacy_lowering(fixture.app(), original).unwrap();
    let instructions = ir
        .functions
        .iter()
        .map(|f| f.instructions.len())
        .sum::<usize>();
    let mut limits = vec![
        PackageCompilationLimits {
            max_ast_nodes: 0,
            ..original
        },
        PackageCompilationLimits {
            max_interface_nodes: 0,
            ..original
        },
    ];
    for value in 0..instructions {
        let mut limit = original;
        limit.lowering.max_instructions = value;
        limits.push(limit);
    }
    for value in 0..strings(&ir) {
        let mut limit = original;
        limit.lowering.max_string_bytes = value;
        limits.push(limit);
    }
    for limit in limits {
        let expected = compile_with_legacy_lowering(fixture.app(), limit).unwrap_err();
        assert_eq!(
            compile_with_checked_evidence(fixture.app(), limit, evidence()).unwrap_err(),
            CheckedPackageError::Diagnostics(expected)
        );
    }
}

#[test]
fn foreign_nominal_records_and_private_calls_keep_existing_errors() {
    let fixture = Fixture::new();
    std::fs::create_dir_all(fixture.0.join("other")).unwrap();
    std::fs::write(
        fixture.0.join("other/sovra.toml"),
        "[project]\nname = \"other\"\nentry = \"main.svr\"\n",
    )
    .unwrap();
    std::fs::write(fixture.0.join("other/main.svr"), LIBRARY).unwrap();
    let manifest = fixture.0.join("app/sovra.toml");
    let text = std::fs::read_to_string(&manifest).unwrap().replace(
        "[dependencies.b]\npath = \"../lib\"",
        "[dependencies.b]\npath = \"../other\"",
    );
    std::fs::write(&manifest, text).unwrap();
    for (source, code) in [
        (
            "use a::api; use b::api; fn main() { print(b::api::read(a::api::make(1))); }",
            "E3007",
        ),
        ("use a::api; fn main() { a::api::secret(); }", "E3004"),
    ] {
        std::fs::write(fixture.0.join("app/main.svr"), source).unwrap();
        let expected =
            compile_with_legacy_lowering(fixture.app(), PackageCompilationLimits::DEFAULT)
                .unwrap_err();
        assert!(expected
            .items
            .iter()
            .any(|diagnostic| diagnostic.code == code));
        assert_eq!(
            compile_with_checked_evidence(
                fixture.app(),
                PackageCompilationLimits::DEFAULT,
                evidence()
            )
            .unwrap_err(),
            CheckedPackageError::Diagnostics(expected.clone())
        );
        // Exhausted import metadata must not hide an ordinary semantic failure.
        assert_eq!(
            compile_with_checked_evidence(
                fixture.app(),
                PackageCompilationLimits::DEFAULT,
                PackageEvidenceLimits {
                    max_imports: 0,
                    ..evidence()
                }
            )
            .unwrap_err(),
            CheckedPackageError::Diagnostics(expected)
        );
    }
}

#[test]
fn transitive_signature_records_keep_the_original_owner_across_relay_units() {
    let fixture = Fixture::new();
    std::fs::create_dir_all(fixture.0.join("leaf")).unwrap();
    std::fs::write(
        fixture.0.join("leaf/sovra.toml"),
        "[project]\nname = \"leaf\"\nentry = \"main.svr\"\n",
    )
    .unwrap();
    std::fs::write(fixture.0.join("leaf/main.svr"), LIBRARY).unwrap();
    std::fs::write(fixture.0.join("lib/sovra.toml"), "[project]\nname = \"relay\"\nentry = \"main.svr\"\n[dependencies.leaf]\npath = \"../leaf\"\n").unwrap();
    std::fs::write(fixture.0.join("lib/main.svr"), "use leaf::api; mod api { export fn make(value: Float) -> leaf::api::Item { return leaf::api::make(value); } export fn read(item: leaf::api::Item) -> Float { return leaf::api::read(item); } export fn widen(value: Float) -> Float { return leaf::api::widen(value); } }").unwrap();
    let old =
        compile_with_legacy_lowering(fixture.app(), PackageCompilationLimits::DEFAULT).unwrap();
    let new =
        compile_with_checked_evidence(fixture.app(), PackageCompilationLimits::DEFAULT, evidence())
            .unwrap();
    assert_eq!(new.units, 3);
    assert_eq!(new.ir, old);
    assert_eq!(interpreter::run(&new.ir).unwrap(), ["3", "1", "0.5"]);
    assert!(new.ir.functions.iter().flat_map(|f| &f.instructions).any(|instruction| matches!(instruction, Instruction::MakeStruct { type_name, .. } if type_name == "@package2::type::api::Item")));
}

#[test]
fn ordinary_package_entry_points_use_the_active_checked_profile() {
    use sovra::compiler::project::packages;
    let fixture = Fixture::new();
    let reference =
        packages::compile_with_legacy_lowering(fixture.app(), PackageCompilationLimits::DEFAULT)
            .unwrap();
    assert_eq!(packages::compile(fixture.app()).unwrap(), reference);
    assert_eq!(
        packages::compile_with_compilation_limits(fixture.app(), PackageCompilationLimits::DEFAULT)
            .unwrap(),
        reference
    );
    let explicit = packages::compile_with_checked_evidence(
        fixture.app(),
        PackageCompilationLimits::DEFAULT,
        packages::PackageEvidenceLimits::DEFAULT,
    )
    .unwrap();
    assert_eq!(explicit.ir, reference);
    assert_eq!(explicit.units, 2);
    assert_eq!(
        packages::compile_with_compilation_limits(
            fixture.app(),
            PackageCompilationLimits {
                lowering: sovra::compiler::resources::LoweringLimits {
                    max_instructions: 0,
                    max_string_bytes: PackageCompilationLimits::DEFAULT.lowering.max_string_bytes
                },
                ..PackageCompilationLimits::DEFAULT
            }
        )
        .unwrap_err()
        .items[0]
            .code,
        "E3021"
    );
}
