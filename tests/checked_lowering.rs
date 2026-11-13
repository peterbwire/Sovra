//! Checked lowering consumes retained evidence and preserves the legacy IR contract.

use sovra::compiler::{
    checked::{
        CheckError, CheckedUnit, EvidenceError, EvidenceLimits, FlowEvidenceLimits,
        LoweringEvidenceLimits, SemanticEvidenceLimits,
    },
    interpreter,
    ir::{self, CheckedLoweringError, Instruction, IrProgram, Literal},
    parser::Parser,
    resources::LoweringLimits,
    semantic::SemanticAnalyzer,
};

fn structural() -> EvidenceLimits {
    EvidenceLimits {
        max_nodes: 32768,
        max_source_bytes: 65536,
    }
}
fn types() -> SemanticEvidenceLimits {
    SemanticEvidenceLimits {
        max_facts: 32768,
        max_types: 32768,
        max_name_bytes: 2 * 1024 * 1024,
    }
}
fn flow() -> FlowEvidenceLimits {
    FlowEvidenceLimits {
        max_declarations: 32768,
        max_uses: 32768,
        max_coercions: 32768,
        max_name_bytes: 2 * 1024 * 1024,
    }
}
fn contexts() -> LoweringEvidenceLimits {
    LoweringEvidenceLimits {
        max_function_results: 4096,
        max_contexts: 65536,
    }
}
fn snapshot(source: &str) -> CheckedUnit {
    CheckedUnit::from_source_for_lowering(source, structural(), types(), flow(), contexts())
        .unwrap()
}

#[test]
fn checked_lowering_preserves_exact_ir_and_execution_on_portable_programs() {
    let cases = [
        include_str!("../examples/hello-world/main.svr"),
        include_str!("../examples/functions/main.svr"),
        include_str!("../examples/numbers/main.svr"),
        include_str!("../examples/strings/main.svr"),
        include_str!("../examples/records/main.svr"),
        include_str!("../examples/modules/main.svr"),
        include_str!("../examples/scoped-types/main.svr"),
        include_str!("../examples/library-foundations/main.svr"),
        "type Real = Float; struct Point { x: Real } fn f(x: Real) -> Real { return x; } fn g() -> Real { return 1.0; } fn main() { let mut x: Real = 1; x = 2; let p = Point { x: 3.0 }; print(f(p.x)); print(g()); let mut a = [[0.0]]; let b = [[1]]; a = b; a[0] = [2]; print(a[0][0]); let mixed = [[2.0], [1]]; print(mixed[1][0]); }",
        "fn x() -> Int { return 1; } fn main() { let mut x = 2; if true { let x = x + 1; print(x); } else { let x = 4; print(x); } while false { let x = x + 2; print(x); } x = x + 1; let x = x + 2; print(x); print(x()); if false && true { print(0); } if true || false { print(1); } }",
        "type N = Float; mod m { type N = Int; struct P { x: Float } export fn f(x: N) -> Float { let p = P { x: 2 }; return p.x + x; } } fn main() { print(m::f(1)); let a = [[], [1.0]]; let mut b = [[0.0]]; b = a; print(b[1][0]); }",
    ];
    for (case, source) in cases.iter().enumerate() {
        let parsed = Parser::new().parse_source(source).unwrap();
        let typed = SemanticAnalyzer::new().analyze(&parsed).unwrap();
        let old = ir::lower_with_limits(&typed, LoweringLimits::DEFAULT).unwrap();
        let unit = snapshot(source);
        let new = ir::lower_checked(&unit, LoweringLimits::DEFAULT).unwrap();
        assert_eq!(new, old, "exact IR case {case}");
        let executed = interpreter::run(&new);
        assert!(executed.is_ok(), "executable case {case}");
        assert_eq!(executed, interpreter::run(&old), "execution case {case}");
        // Emission helpers are fresh per run, never mutable state in the snapshot.
        let clone = unit.clone();
        assert_eq!(
            ir::lower_checked(&clone, LoweringLimits::DEFAULT).unwrap(),
            new
        );
    }
}

fn payload(program: &IrProgram) -> usize {
    program
        .functions
        .iter()
        .map(|function| {
            function.name.len()
                + function.parameters.iter().map(String::len).sum::<usize>()
                + function
                    .instructions
                    .iter()
                    .map(|instruction| match instruction {
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
                        _ => 0,
                    })
                    .sum::<usize>()
        })
        .sum()
}

#[test]
fn emission_limits_match_legacy_at_every_instruction_and_string_boundary() {
    let source = "type Real = Float; struct P { x: Real } fn f(x: Real) -> Real { return x; } fn main() { let x = 1; if true { let x = x + 1; print(\"λ😀\"); print(f(x)); } let p = P { x: 1.0 }; print(p.x); }";
    let unit = snapshot(source);
    let typed = SemanticAnalyzer::new()
        .analyze(&Parser::new().parse_source(source).unwrap())
        .unwrap();
    let complete = ir::lower_checked(&unit, LoweringLimits::DEFAULT).unwrap();
    let instructions = complete
        .functions
        .iter()
        .map(|f| f.instructions.len())
        .sum::<usize>();
    let bytes = payload(&complete);
    let exact = LoweringLimits {
        max_instructions: instructions,
        max_string_bytes: bytes,
    };
    assert_eq!(ir::lower_checked(&unit, exact).unwrap(), complete);
    for budget in (0..instructions)
        .map(|max_instructions| LoweringLimits {
            max_instructions,
            max_string_bytes: bytes,
        })
        .chain((0..bytes).map(|max_string_bytes| LoweringLimits {
            max_instructions: instructions,
            max_string_bytes,
        }))
    {
        let old = ir::lower_with_limits(&typed, budget).unwrap_err();
        assert_eq!(
            ir::lower_checked(&unit, budget).unwrap_err(),
            CheckedLoweringError::Emission(old)
        );
    }
    assert_eq!(ir::lower_checked(&unit, exact).unwrap(), complete);
}

#[test]
fn lowering_metadata_is_explicit_atomic_and_preserves_diagnostic_precedence() {
    let source = "struct P { x: Float } fn f() -> Float { return 1.0; } fn main() { let p = P { x: 2.0 }; print(f()); }";
    let ordinary = CheckedUnit::from_source(source, structural()).unwrap();
    let typed = CheckedUnit::from_source_with_evidence(source, structural(), types()).unwrap();
    let full_flow =
        CheckedUnit::from_source_with_flow_evidence(source, structural(), types(), flow()).unwrap();
    for unit in [ordinary, typed, full_flow] {
        assert_eq!(
            ir::lower_checked(&unit, LoweringLimits::DEFAULT).unwrap_err(),
            CheckedLoweringError::EvidenceUnavailable
        );
    }
    let unit = snapshot(source);
    let metadata = unit.semantic_evidence().unwrap().lowering().unwrap();
    let exact = LoweringEvidenceLimits {
        max_function_results: metadata.function_results().len(),
        max_contexts: metadata.contexts().len(),
    };
    assert_eq!(exact.max_function_results, 2);
    assert_eq!(exact.max_contexts, 3); // Identity return, field and initializer contexts.
    assert!(
        CheckedUnit::from_source_for_lowering(source, structural(), types(), flow(), exact).is_ok()
    );
    for limits in [
        LoweringEvidenceLimits {
            max_function_results: exact.max_function_results - 1,
            ..exact
        },
        LoweringEvidenceLimits {
            max_contexts: exact.max_contexts - 1,
            ..exact
        },
        LoweringEvidenceLimits {
            max_function_results: 0,
            ..exact
        },
        LoweringEvidenceLimits {
            max_contexts: 0,
            ..exact
        },
    ] {
        assert_eq!(
            CheckedUnit::from_source_for_lowering(source, structural(), types(), flow(), limits)
                .unwrap_err(),
            EvidenceError::Limit
        );
    }
    let invalid = "fn main() { let p = 1; print(missing); }";
    let errors = SemanticAnalyzer::new()
        .analyze(&Parser::new().parse_source(invalid).unwrap())
        .unwrap_err();
    assert_eq!(
        CheckedUnit::from_source_for_lowering(
            invalid,
            structural(),
            types(),
            flow(),
            LoweringEvidenceLimits {
                max_function_results: 0,
                max_contexts: 0
            }
        )
        .unwrap_err(),
        EvidenceError::Check(CheckError::Diagnostics(errors))
    );
    let zero_types = SemanticEvidenceLimits {
        max_facts: 0,
        max_types: 0,
        max_name_bytes: 0,
    };
    assert!(
        CheckedUnit::from_source_with_evidence("fn main() {}", structural(), zero_types).is_ok()
    );
    assert_eq!(
        CheckedUnit::from_source_for_lowering(
            "fn main() {}",
            structural(),
            zero_types,
            flow(),
            contexts()
        )
        .unwrap_err(),
        EvidenceError::Limit
    ); // Retained default Unit result needs a type slot.
    let empty = CheckedUnit::from_source_for_lowering(
        "",
        structural(),
        zero_types,
        flow(),
        LoweringEvidenceLimits {
            max_function_results: 0,
            max_contexts: 0,
        },
    )
    .unwrap();
    assert!(ir::lower_checked(
        &empty,
        LoweringLimits {
            max_instructions: 0,
            max_string_bytes: 0
        }
    )
    .unwrap()
    .functions
    .is_empty());
}

#[test]
fn caller_ast_equal_spans_do_not_merge_checked_binding_identities() {
    fn immutable_thread_safe<T: Send + Sync>() {}
    immutable_thread_safe::<CheckedUnit>();
    let mut ast = Parser::new()
        .parse_source("fn main() { let x = 1; if true { let x = x + 1; print(x); } print(x); }")
        .unwrap();
    let span = ast.functions[0].span;
    if let sovra::compiler::ast::Statement::Let {
        value,
        span: statement_span,
        ..
    } = &mut ast.functions[0].body[0]
    {
        *statement_span = span;
        value.span = span;
    }
    let unit =
        CheckedUnit::from_program_for_lowering(&ast, structural(), types(), flow(), contexts())
            .unwrap();
    assert!(unit.source().is_none());
    let old = ir::lower_program(&ast).unwrap();
    assert_eq!(
        ir::lower_checked(&unit, LoweringLimits::DEFAULT).unwrap(),
        old
    );
    // Caller ASTs may represent the compatibility alias as a one-part path.
    let mut alias = Parser::new()
        .parse_source("fn main() { print(1); }")
        .unwrap();
    let sovra::compiler::ast::Statement::Expression(call) = &mut alias.functions[0].body[0] else {
        unreachable!()
    };
    let sovra::compiler::ast::ExpressionKind::Call { callee, .. } = &mut call.kind else {
        unreachable!()
    };
    callee.kind = sovra::compiler::ast::ExpressionKind::QualifiedName {
        path: vec!["print".into()],
    };
    let unit =
        CheckedUnit::from_program_for_lowering(&alias, structural(), types(), flow(), contexts())
            .unwrap();
    assert_eq!(
        ir::lower_checked(&unit, LoweringLimits::DEFAULT).unwrap(),
        ir::lower_program(&alias).unwrap()
    );
}
