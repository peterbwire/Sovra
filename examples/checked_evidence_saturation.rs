//! Manual candidate-profile saturation probe. Not run by ordinary compiler commands.
use sovra::compiler::{
    checked::{
        CheckError, CheckedUnit, EvidenceError, EvidenceLimits, FlowEvidenceLimits,
        LoweringEvidenceLimits, SemanticEvidenceLimits,
    },
    ir,
    parser::Parser,
    resources::LoweringLimits,
};
use std::fmt::Write;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert!(
        args.len() == 3 || args.len() == 4,
        "nodes|types exact|overflow [--lower]"
    );
    let lower = args.len() == 4;
    if lower {
        assert_eq!(args[3], "--lower");
    }
    let overflow = match args[2].as_str() {
        "exact" => false,
        "overflow" => true,
        _ => panic!("unknown boundary"),
    };
    let mut source = String::new();
    match args[1].as_str() {
        "nodes" => {
            source.push_str("fn helper() {} fn main() {");
            for _ in 0..(124_999 + usize::from(overflow)) {
                source.push_str("1;");
            }
            source.push('}');
        }
        "types" => {
            let count = 65_535 + usize::from(overflow);
            for i in 0..count {
                write!(source, "struct T{i} {{}}").unwrap();
            }
            source.push_str("fn f(");
            for i in 0..count {
                if i > 0 {
                    source.push(',');
                }
                write!(source, "p{i}: T{i}").unwrap();
            }
            source.push_str(") {} fn main() {}");
        }
        _ => panic!("unknown shape"),
    }
    let parsed = Parser::new().parse_source(&source).unwrap();
    let result = CheckedUnit::from_program_for_lowering(
        &parsed,
        EvidenceLimits {
            max_nodes: 250_000,
            max_source_bytes: 0,
        },
        SemanticEvidenceLimits {
            max_facts: 250_000,
            max_types: 65_536,
            max_name_bytes: 16 * 1024 * 1024,
        },
        FlowEvidenceLimits {
            max_declarations: 250_000,
            max_uses: 250_000,
            max_coercions: 250_000,
            max_name_bytes: 16 * 1024 * 1024,
        },
        LoweringEvidenceLimits {
            max_function_results: 250_000,
            max_contexts: 1_000_000,
        },
    );
    if overflow {
        let error = result.unwrap_err();
        assert_eq!(
            error,
            if args[1] == "nodes" {
                EvidenceError::Check(CheckError::NodeLimit)
            } else {
                EvidenceError::Limit
            }
        );
        let diagnostic = error.into_diagnostics();
        assert_eq!(diagnostic.items[0].code, "E3025");
        println!("expected_exhaustion shape={} code=E3025", args[1]);
    } else {
        let unit = result.unwrap();
        let evidence = unit.semantic_evidence().unwrap();
        match args[1].as_str() {
            "nodes" => assert_eq!(unit.nodes().len(), 250_000),
            "types" => assert_eq!(evidence.types().len(), 65_536),
            _ => unreachable!(),
        }
        if lower {
            let program = ir::lower_checked(&unit, LoweringLimits::DEFAULT).unwrap();
            println!(
                "lowered functions={} instructions={}",
                program.functions.len(),
                program
                    .functions
                    .iter()
                    .map(|f| f.instructions.len())
                    .sum::<usize>()
            );
        }
        println!(
            "accepted shape={} nodes={} types={}",
            args[1],
            unit.nodes().len(),
            evidence.types().len()
        );
    }
}
