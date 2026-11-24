//! Opt-in facts must come from the existing checker, not another inference pass.

use sovra::compiler::checked::{
    CheckError, CheckedUnit, EvidenceError, EvidenceLimits, FactKind, RecordedType,
    SemanticEvidenceLimits,
};
use sovra::compiler::{ir, parser::Parser, semantic::SemanticAnalyzer};

fn structural() -> EvidenceLimits {
    EvidenceLimits {
        max_nodes: 4096,
        max_source_bytes: 16384,
    }
}

fn generous() -> SemanticEvidenceLimits {
    SemanticEvidenceLimits {
        max_facts: 4096,
        max_types: 512,
        max_name_bytes: 4096,
    }
}

fn at(unit: &CheckedUnit, start: usize, kind: FactKind) -> &RecordedType {
    let evidence = unit.semantic_evidence().unwrap();
    let fact = evidence
        .facts()
        .iter()
        .find(|fact| fact.kind == kind && unit.nodes()[fact.node.ordinal()].span.start == start)
        .expect("checked fact at the selected structural node");
    &evidence.types()[fact.ty.ordinal()]
}

#[test]
fn recorded_types_preserve_initializer_binding_shadowing_and_canonical_alias_meaning() {
    let source = "// λ😀\r\ntype Real = Float; struct Point { x: Real } \
        fn widen(value: Real) -> Real { return value; } \
        fn main() { let mut x: Real = 1; \
        if true { let x = \"inner\"; print(x); } print(x); \
        let p = Point { x: 2 }; let y = widen(3); }";
    let unit = CheckedUnit::from_source_with_evidence(source, structural(), generous()).unwrap();
    assert_eq!(
        at(
            &unit,
            source.find("value: Real").unwrap(),
            FactKind::Parameter
        ),
        &RecordedType::Float
    );
    assert_eq!(
        at(
            &unit,
            source.find("let mut x").unwrap(),
            FactKind::Local { mutable: true }
        ),
        &RecordedType::Float
    );
    assert_eq!(
        at(&unit, source.find("1;").unwrap(), FactKind::Expression),
        &RecordedType::Int
    );
    assert_eq!(
        at(
            &unit,
            source.find("let x =").unwrap(),
            FactKind::Local { mutable: false }
        ),
        &RecordedType::String
    );
    let inner = source.find("print(x)").unwrap() + "print(".len();
    let outer = source.rfind("print(x)").unwrap() + "print(".len();
    assert_eq!(
        at(&unit, inner, FactKind::Expression),
        &RecordedType::String
    );
    assert_eq!(at(&unit, outer, FactKind::Expression), &RecordedType::Float);
    assert_eq!(
        at(
            &unit,
            source.find("let y").unwrap(),
            FactKind::Local { mutable: false }
        ),
        &RecordedType::Float
    );
    assert!(
        matches!(at(&unit, source.find("let p").unwrap(), FactKind::Local { mutable: false }), RecordedType::Named(name) if name.as_ref() == "Point")
    );
    // Retaining facts must not alter the normalized AST, structural IDs or IR.
    let plain = CheckedUnit::from_source(source, structural()).unwrap();
    assert_eq!(plain.nodes(), unit.nodes());
    assert_eq!(plain.program(), unit.program());
    assert!(plain.semantic_evidence().is_none());
    let typed = SemanticAnalyzer::new()
        .analyze(&Parser::new().parse_source(source).unwrap())
        .unwrap();
    let detached = sovra::compiler::semantic::TypedProgram {
        program: unit.program().clone(),
    };
    assert_eq!(ir::lower(&typed), ir::lower(&detached));
    // IDs remain usable after cloning/dropping the original snapshot.
    let clone = unit.clone();
    drop(unit);
    assert_eq!(
        at(&clone, outer, FactKind::Expression),
        &RecordedType::Float
    );
}

#[test]
fn nested_arrays_preserve_holes_and_intern_types_without_duplicate_name_payload() {
    let source = "struct Item { x: Int } fn main() { let empty = []; \
        let nested = [[], [1]]; let a = Item { x: 1 }; let b = Item { x: 2 }; }";
    let unit = CheckedUnit::from_source_with_evidence(source, structural(), generous()).unwrap();
    let evidence = unit.semantic_evidence().unwrap();
    let RecordedType::Array(element) = at(
        &unit,
        source.find("let empty").unwrap(),
        FactKind::Local { mutable: false },
    ) else {
        panic!("array type")
    };
    assert_eq!(evidence.types()[element.ordinal()], RecordedType::Unknown);
    let RecordedType::Array(element) = at(
        &unit,
        source.find("let nested").unwrap(),
        FactKind::Local { mutable: false },
    ) else {
        panic!("nested array type")
    };
    let RecordedType::Array(leaf) = evidence.types()[element.ordinal()] else {
        panic!("array element")
    };
    assert_eq!(evidence.types()[leaf.ordinal()], RecordedType::Int);
    assert_eq!(evidence.name_bytes(), "Item".len());
    assert_eq!(
        evidence
            .types()
            .iter()
            .filter(|ty| matches!(ty, RecordedType::Named(_)))
            .count(),
        1
    );
    for fact in evidence.facts() {
        assert!(fact.node.ordinal() < unit.nodes().len());
        assert!(fact.ty.ordinal() < evidence.types().len());
    }
    let ids: std::collections::HashSet<_> = evidence.facts().iter().map(|fact| fact.node).collect();
    assert_eq!(ids.len(), evidence.facts().len());
    for (index, ty) in evidence.types().iter().enumerate() {
        if let RecordedType::Array(element) = ty {
            assert!(element.ordinal() < index);
        }
    }
}

#[test]
fn module_aliases_and_builtin_result_types_follow_existing_resolution() {
    let source = "type Real = Int; mod m { type Real = Float; struct Point { x: Real } \
        fn value(v: Real) -> Real { let p = Point { x: 1 }; return p.x; } } \
        fn main() { let size = std::len(\"λ😀\"); }";
    let unit = CheckedUnit::from_source_with_evidence(source, structural(), generous()).unwrap();
    assert_eq!(
        at(&unit, source.find("v: Real").unwrap(), FactKind::Parameter),
        &RecordedType::Float
    );
    assert!(
        matches!(at(&unit, source.find("let p").unwrap(), FactKind::Local { mutable: false }), RecordedType::Named(name) if name.as_ref() == "m::Point")
    );
    // Receiver p and field p.x share a start byte; select the full field node.
    let evidence = unit.semantic_evidence().unwrap();
    let start = source.find("p.x").unwrap();
    let field = evidence
        .facts()
        .iter()
        .find(|fact| {
            let node = &unit.nodes()[fact.node.ordinal()];
            fact.kind == FactKind::Expression
                && node.span.start == start
                && node.span.end == start + 3
        })
        .unwrap();
    assert_eq!(evidence.types()[field.ty.ordinal()], RecordedType::Float);
    assert_eq!(
        at(
            &unit,
            source.find("let size").unwrap(),
            FactKind::Local { mutable: false }
        ),
        &RecordedType::Int
    );
    let head = source.find("std::len").unwrap();
    let evidence = unit.semantic_evidence().unwrap();
    // The call has an Int result, but its qualified function head is not a
    // first-class value type; absence must not be converted into Unknown/Any.
    let matches: Vec<_> = evidence
        .facts()
        .iter()
        .filter(|fact| unit.nodes()[fact.node.ordinal()].span.start == head)
        .collect();
    assert_eq!(matches.len(), 1);
    assert_eq!(evidence.types()[matches[0].ty.ordinal()], RecordedType::Int);
}

#[test]
fn exact_overflow_and_zero_semantic_allowances_return_no_partial_snapshot() {
    let source = "struct Item { x: Int } fn main() { let a = Item { x: 1 }; }";
    let observed =
        CheckedUnit::from_source_with_evidence(source, structural(), generous()).unwrap();
    let facts = observed.semantic_evidence().unwrap();
    let exact = SemanticEvidenceLimits {
        max_facts: facts.facts().len(),
        max_types: facts.types().len(),
        max_name_bytes: facts.name_bytes(),
    };
    assert!(CheckedUnit::from_source_with_evidence(source, structural(), exact).is_ok());
    for limit in [
        SemanticEvidenceLimits {
            max_facts: exact.max_facts - 1,
            ..exact
        },
        SemanticEvidenceLimits {
            max_types: exact.max_types - 1,
            ..exact
        },
        SemanticEvidenceLimits {
            max_name_bytes: exact.max_name_bytes - 1,
            ..exact
        },
        SemanticEvidenceLimits {
            max_facts: 0,
            ..exact
        },
        SemanticEvidenceLimits {
            max_types: 0,
            ..exact
        },
        SemanticEvidenceLimits {
            max_name_bytes: 0,
            ..exact
        },
    ] {
        assert_eq!(
            CheckedUnit::from_source_with_evidence(source, structural(), limit).unwrap_err(),
            EvidenceError::Limit
        );
    }
    let zero = SemanticEvidenceLimits {
        max_facts: 0,
        max_types: 0,
        max_name_bytes: 0,
    };
    let empty = CheckedUnit::from_source_with_evidence("fn main() {}", structural(), zero).unwrap();
    assert!(empty.semantic_evidence().unwrap().facts().is_empty());
    assert!(empty.semantic_evidence().unwrap().types().is_empty());
}

#[test]
fn failed_sources_keep_diagnostics_even_after_evidence_exhaustion() {
    for source in [
        "fn main() { let x = 1; print(missing); }",
        "type A = B; type B = A;",
        "fn f(x) {}",
    ] {
        let expected = Parser::new()
            .parse_source(source)
            .and_then(|ast| SemanticAnalyzer::new().analyze(&ast))
            .unwrap_err();
        for limits in [
            generous(),
            SemanticEvidenceLimits {
                max_facts: 0,
                max_types: 0,
                max_name_bytes: 0,
            },
        ] {
            assert_eq!(
                CheckedUnit::from_source_with_evidence(source, structural(), limits).unwrap_err(),
                EvidenceError::Check(CheckError::Diagnostics(expected.clone()))
            );
        }
    }
}

#[test]
fn equal_spans_use_structural_links_and_statement_expression_roles_stay_distinct() {
    let mut program = Parser::new()
        .parse_source("fn main() { let x = 1; print(x); }")
        .unwrap();
    let span = program.functions[0].span;
    if let sovra::compiler::ast::Statement::Let {
        value,
        span: statement_span,
        ..
    } = &mut program.functions[0].body[0]
    {
        *statement_span = span;
        value.span = span;
    }
    let unit = CheckedUnit::from_program_with_evidence(&program, structural(), generous()).unwrap();
    assert_eq!(unit.source(), None);
    let evidence = unit.semantic_evidence().unwrap();
    let local = evidence
        .facts()
        .iter()
        .find(|f| matches!(f.kind, FactKind::Local { .. }))
        .unwrap();
    let expr = evidence
        .facts()
        .iter()
        .find(|f| f.kind == FactKind::Expression && unit.nodes()[f.node.ordinal()].span == span)
        .unwrap();
    assert_ne!(local.node, expr.node);
    assert_eq!(local.ty, expr.ty);
}
