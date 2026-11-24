//! Manual aggregate graph/IR boundary probe; never executes package code.
use sovra::compiler::{
    checked::{FlowEvidenceLimits, LoweringEvidenceLimits, SemanticEvidenceLimits},
    ir::{Instruction, Literal},
    project::packages::{self, PackageEvidenceLimits},
    resources::PackageCompilationLimits,
};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(
        args.len(),
        6,
        "legacy|checked root instructions string-bytes expected-code|ok"
    );
    assert!(matches!(
        args[1].as_str(),
        "legacy" | "checked" | "checked-tight"
    ));
    let mut compilation = PackageCompilationLimits::DEFAULT;
    compilation.lowering.max_instructions = args[3].parse().unwrap();
    compilation.lowering.max_string_bytes = args[4].parse().unwrap();
    let result = if args[1] == "checked" {
        packages::compile_with_compilation_limits(&args[2], compilation)
    } else if args[1] == "checked-tight" {
        packages::compile_with_checked_evidence(
            &args[2],
            compilation,
            PackageEvidenceLimits {
                max_nodes: if args[1] == "checked-tight" {
                    999_999
                } else {
                    1_250_000
                },
                semantic: SemanticEvidenceLimits {
                    max_facts: 1_250_000,
                    max_types: 262_144,
                    max_name_bytes: 64 * 1024 * 1024,
                },
                flow: FlowEvidenceLimits {
                    max_declarations: 1_250_000,
                    max_uses: 1_250_000,
                    max_coercions: 1_250_000,
                    max_name_bytes: 64 * 1024 * 1024,
                },
                lowering: LoweringEvidenceLimits {
                    max_function_results: 1_250_000,
                    max_contexts: 5_000_000,
                },
                max_imports: 250_000,
                max_import_bytes: 64 * 1024 * 1024,
            },
        )
        .map(|r| r.ir)
        .map_err(|e| e.into_diagnostics())
    } else {
        packages::compile_with_legacy_lowering(&args[2], compilation)
    };
    match result {
        Err(errors) => {
            assert_ne!(args[5], "ok", "unexpected failure: {errors:?}");
            assert_eq!(errors.items.len(), 1);
            assert_eq!(errors.items[0].code, args[5]);
            // Identical located diagnostics must survive the checked path.
            println!("{errors:?}");
        }
        Ok(ir) => {
            assert_eq!(args[5], "ok", "expected rejection");
            let mut count = 0usize;
            let mut bytes = 0usize;
            for function in &ir.functions {
                bytes += function.name.len()
                    + function.parameters.iter().map(String::len).sum::<usize>();
                for instruction in &function.instructions {
                    count += 1;
                    bytes += match instruction {
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
                    };
                }
            }
            eprintln!("{{\"instructions\":{count},\"string_bytes\":{bytes}}}");
            println!("{ir:?}");
        }
    }
}
