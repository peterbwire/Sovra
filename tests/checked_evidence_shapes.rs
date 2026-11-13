//! Evidence quotas must independently bound shallow syntax with large retained shapes.
use sovra::compiler::{
    checked::{
        CheckError, CheckedUnit, EvidenceError, EvidenceLimits, FlowEvidenceLimits,
        LoweringEvidenceLimits, SemanticEvidenceLimits,
    },
    ir,
    parser::Parser,
    resources::LoweringLimits,
    semantic::SemanticAnalyzer,
};

#[test]
fn adversarial_evidence_shapes_have_exact_independent_limits_and_retry_parity() {
    let deep = format!(
        "fn main() {{ let a0 = 1; {} }}",
        (1..=192)
            .map(|i| format!("let a{i} = [a{}];", i - 1))
            .collect::<String>()
    );
    let names = format!(
        "fn main() {{ {} }}",
        (0..64)
            .map(|i| format!("let local_{i}_{} = {i};", "x".repeat(1000)))
            .collect::<String>()
    );
    let nominals = format!(
        "{} fn main() {{ {} }}",
        (0..128)
            .map(|i| format!("struct T{i} {{ value: Float }}"))
            .collect::<String>(),
        (0..128)
            .map(|i| format!("let v{i} = T{i} {{ value: {i} }};"))
            .collect::<String>()
    );
    for source in [&deep, &names, &nominals] {
        let parsed = Parser::new().parse_source(source).unwrap();
        let generous = (
            EvidenceLimits {
                max_nodes: 100_000,
                max_source_bytes: 0,
            },
            SemanticEvidenceLimits {
                max_facts: 100_000,
                max_types: 100_000,
                max_name_bytes: 1_000_000,
            },
            FlowEvidenceLimits {
                max_declarations: 100_000,
                max_uses: 100_000,
                max_coercions: 100_000,
                max_name_bytes: 1_000_000,
            },
            LoweringEvidenceLimits {
                max_function_results: 100_000,
                max_contexts: 100_000,
            },
        );
        let build = |limits: (
            EvidenceLimits,
            SemanticEvidenceLimits,
            FlowEvidenceLimits,
            LoweringEvidenceLimits,
        )| {
            CheckedUnit::from_program_for_lowering(&parsed, limits.0, limits.1, limits.2, limits.3)
        };
        let unit = build(generous).unwrap();
        let evidence = unit.semantic_evidence().unwrap();
        let flow = evidence.flow().unwrap();
        let lowering = evidence.lowering().unwrap();
        let exact = (
            EvidenceLimits {
                max_nodes: unit.nodes().len(),
                max_source_bytes: 0,
            },
            SemanticEvidenceLimits {
                max_facts: evidence.facts().len(),
                max_types: evidence.types().len(),
                max_name_bytes: evidence.name_bytes(),
            },
            FlowEvidenceLimits {
                max_declarations: flow.declarations().len(),
                max_uses: flow.uses().len(),
                max_coercions: flow.coercions().len(),
                max_name_bytes: flow.name_bytes(),
            },
            LoweringEvidenceLimits {
                max_function_results: lowering.function_results().len(),
                max_contexts: lowering.contexts().len(),
            },
        );
        if source == &deep {
            assert!(
                evidence.types().len() > 192,
                "inferred depth exceeds syntax nesting"
            );
        }
        if source == &names {
            assert!(
                flow.name_bytes() > 64_000,
                "names need their own byte budget"
            );
        }
        if source == &nominals {
            assert!(evidence.name_bytes() > 0);
            assert!(flow.coercions().len() >= 128);
        }
        let expected = ir::lower_with_limits(
            &SemanticAnalyzer::new().analyze(&parsed).unwrap(),
            LoweringLimits::DEFAULT,
        )
        .unwrap();
        for field in 0..10 {
            let mut limited = exact;
            let counter = match field {
                0 => &mut limited.0.max_nodes,
                1 => &mut limited.1.max_facts,
                2 => &mut limited.1.max_types,
                3 => &mut limited.1.max_name_bytes,
                4 => &mut limited.2.max_declarations,
                5 => &mut limited.2.max_uses,
                6 => &mut limited.2.max_coercions,
                7 => &mut limited.2.max_name_bytes,
                8 => &mut limited.3.max_function_results,
                _ => &mut limited.3.max_contexts,
            };
            if *counter == 0 {
                continue;
            }
            *counter -= 1;
            let failure = build(limited).unwrap_err();
            if field == 0 {
                assert_eq!(failure, EvidenceError::Check(CheckError::NodeLimit));
            } else {
                assert_eq!(failure, EvidenceError::Limit, "quota {field}");
            }
            // No partially published state contaminates a fresh exact-budget retry.
            let retry = build(exact).unwrap();
            assert_eq!(
                ir::lower_checked(&retry, LoweringLimits::DEFAULT).unwrap(),
                expected
            );
        }
    }
}
