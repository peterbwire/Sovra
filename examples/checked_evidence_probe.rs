//! Opt-in benchmark probe; never executes input programs or changes compiler defaults.
use sovra::compiler::{
    checked::{
        CheckedUnit, EvidenceLimits, FlowEvidenceLimits, LoweringEvidenceLimits,
        SemanticEvidenceLimits,
    },
    ir,
    parser::Parser,
    project::packages,
    resources::{LoweringLimits, PackageCompilationLimits},
    semantic::SemanticAnalyzer,
};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 4, "mode source|package path");
    assert!(matches!(
        args[1].as_str(),
        "legacy" | "checked" | "candidate"
    ));
    let checked = args[1] != "legacy";
    let candidate = args[1] == "candidate";
    let package = args[2] == "package";
    let nodes = if candidate {
        if package {
            1_250_000
        } else {
            250_000
        }
    } else {
        1_000_000
    };
    let names = if candidate && package {
        64 * 1024 * 1024
    } else {
        16 * 1024 * 1024
    };
    // Probe-only finite allowances, not proposed production defaults.
    let semantic = SemanticEvidenceLimits {
        max_facts: nodes,
        max_types: if candidate {
            if package {
                262_144
            } else {
                65_536
            }
        } else {
            1_000_000
        },
        max_name_bytes: names,
    };
    let flow = FlowEvidenceLimits {
        max_declarations: nodes,
        max_uses: nodes,
        max_coercions: nodes,
        max_name_bytes: names,
    };
    let lowering = LoweringEvidenceLimits {
        max_function_results: nodes,
        max_contexts: if candidate && package {
            5_000_000
        } else {
            1_000_000
        },
    };
    let program = match args[2].as_str() {
        "source" => {
            let source = std::fs::read_to_string(&args[3]).unwrap();
            let parsed = Parser::new().parse_source(&source).unwrap();
            if checked {
                let unit = CheckedUnit::from_program_for_lowering(
                    &parsed,
                    EvidenceLimits {
                        max_nodes: nodes,
                        max_source_bytes: 0,
                    },
                    semantic,
                    flow,
                    lowering,
                )
                .unwrap();
                let evidence = unit.semantic_evidence().unwrap();
                eprintln!("nodes={} facts={} types={} nominal_bytes={} declarations={} uses={} coercions={} declaration_bytes={} results={} contexts={}", unit.nodes().len(), evidence.facts().len(), evidence.types().len(), evidence.name_bytes(), evidence.flow().unwrap().declarations().len(), evidence.flow().unwrap().uses().len(), evidence.flow().unwrap().coercions().len(), evidence.flow().unwrap().name_bytes(), evidence.lowering().unwrap().function_results().len(), evidence.lowering().unwrap().contexts().len());
                ir::lower_checked(&unit, LoweringLimits::DEFAULT).unwrap()
            } else {
                let unit = SemanticAnalyzer::new().analyze(&parsed).unwrap();
                ir::lower_with_limits(&unit, LoweringLimits::DEFAULT).unwrap()
            }
        }
        "package" => {
            if checked {
                let limits = packages::PackageEvidenceLimits {
                    max_nodes: nodes,
                    semantic,
                    flow,
                    lowering,
                    max_imports: if candidate { 250_000 } else { 1_000_000 },
                    max_import_bytes: if candidate {
                        64 * 1024 * 1024
                    } else {
                        16 * 1024 * 1024
                    },
                };
                let result = packages::compile_with_checked_evidence(
                    std::path::Path::new(&args[3]),
                    PackageCompilationLimits::DEFAULT,
                    limits,
                )
                .unwrap();
                eprintln!(
                    "units={} initial={limits:?} remaining={:?}",
                    result.units, result.remaining
                );
                result.ir
            } else {
                packages::compile_with_legacy_lowering(
                    std::path::Path::new(&args[3]),
                    PackageCompilationLimits::DEFAULT,
                )
                .unwrap()
            }
        }
        _ => panic!("unknown input kind"),
    };
    println!("{program:?}");
}
