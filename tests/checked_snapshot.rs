//! Ownership/index foundation only; inferred semantic facts are a later slice.

use sovra::compiler::ast::{Expression, ExpressionKind, Statement};
use sovra::compiler::checked::{CheckError, CheckedUnit, EvidenceLimits, NodeKind};
use sovra::compiler::diagnostics::Span;
use sovra::compiler::parser::Parser;
use sovra::compiler::resources::check_ast_budget;
use sovra::compiler::semantic::{SemanticAnalyzer, TypedProgram};

fn limits() -> EvidenceLimits {
    EvidenceLimits {
        max_nodes: 4096,
        max_source_bytes: 16384,
    }
}

#[test]
fn snapshot_owns_checked_normalization_and_clones_are_isolated_from_detached_ast() {
    let source = "type Count = Int; fn keep(value: Count) -> Count { return value; }";
    let mut original = Parser::new().parse_source(source).unwrap();
    let expected = SemanticAnalyzer::new().analyze(&original).unwrap();
    let unit = CheckedUnit::from_source(source, limits()).unwrap();
    let clone = unit.clone();
    assert_eq!(unit.program(), &expected.program);
    assert_eq!(unit.source(), Some(source));
    assert!(std::ptr::eq(unit.program(), clone.program()));
    let ast_only = CheckedUnit::from_program(&original, limits()).unwrap();
    assert_eq!(ast_only.source(), None);
    // Public legacy construction/access remains supported, without borrowing
    // mutable state from either immutable snapshot.
    let mut detached = TypedProgram {
        program: unit.program().clone(),
    };
    detached.program.functions[0].name = "changed".into();
    original.functions.clear();
    drop(unit);
    assert_eq!(clone.program(), &expected.program);
    assert_eq!(ast_only.program(), &expected.program);
}

#[test]
fn deterministic_index_covers_all_ast_forms_and_obeys_parent_order() {
    let source = r#"
type Count = Int;
struct Point { x: Float }
fn keep(value: Count) -> Count {
    if value > 0 { return value; } else { return 0; }
}
fn main() {
    let mut xs = [1, 2]; xs[0] = keep(2);
    let p = Point { x: 1 }; print(p.x);
    print("λ😀"); print(true); print(1.5); print(std::len("Sovra"));
    while false { print(xs[0]); }
    return;
}
mod tools {
    type N = Int;
    struct Item { x: N }
    fn value(v: N) -> N { return v; }
}
"#;
    let one = CheckedUnit::from_source(source, limits()).unwrap();
    let two = CheckedUnit::from_source(source, limits()).unwrap();
    assert_eq!(one.nodes(), two.nodes());
    assert_eq!(
        one.nodes().len(),
        check_ast_budget(one.program(), usize::MAX).unwrap()
    );
    for (ordinal, node) in one.nodes().iter().enumerate() {
        assert_eq!(node.id.ordinal(), ordinal);
        if let Some(parent) = node.parent {
            assert!(parent.ordinal() < ordinal);
        }
        assert!(node.span.start <= node.span.end && node.span.end <= source.len());
        assert!(source.is_char_boundary(node.span.start) && source.is_char_boundary(node.span.end));
    }
    for kind in [
        NodeKind::Module,
        NodeKind::Alias,
        NodeKind::Record,
        NodeKind::Field,
        NodeKind::Function,
        NodeKind::Parameter,
        NodeKind::Statement,
        NodeKind::Expression,
    ] {
        assert!(one.nodes().iter().any(|node| node.kind == kind), "{kind:?}");
    }
    let function = CheckedUnit::from_source("fn main() { print(1); }", limits()).unwrap();
    assert_eq!(
        function
            .nodes()
            .iter()
            .map(|node| (node.kind, node.parent.map(|id| id.ordinal())))
            .collect::<Vec<_>>(),
        [
            (NodeKind::Function, None),
            (NodeKind::Statement, Some(0)),
            (NodeKind::Expression, Some(1)),
            (NodeKind::Expression, Some(2)),
            (NodeKind::Expression, Some(2))
        ]
    );
}

#[test]
fn exact_zero_and_overflow_storage_limits_fail_without_partial_snapshot() {
    let source = "fn main() { print(\"λ😀\"); }";
    let parsed = Parser::new().parse_source(source).unwrap();
    let count = check_ast_budget(&parsed, usize::MAX).unwrap();
    let exact = EvidenceLimits {
        max_nodes: count,
        max_source_bytes: source.len(),
    };
    assert_eq!(
        CheckedUnit::from_source(source, exact)
            .unwrap()
            .nodes()
            .len(),
        count
    );
    assert_eq!(
        CheckedUnit::from_source(
            source,
            EvidenceLimits {
                max_nodes: count - 1,
                ..exact
            }
        )
        .unwrap_err(),
        CheckError::NodeLimit
    );
    assert_eq!(
        CheckedUnit::from_source(
            source,
            EvidenceLimits {
                max_source_bytes: source.len() - 1,
                ..exact
            }
        )
        .unwrap_err(),
        CheckError::SourceLimit
    );
    assert_eq!(
        CheckedUnit::from_source(
            source,
            EvidenceLimits {
                max_nodes: 0,
                ..exact
            }
        )
        .unwrap_err(),
        CheckError::NodeLimit
    );
    assert_eq!(
        CheckedUnit::from_source(
            source,
            EvidenceLimits {
                max_source_bytes: 0,
                ..exact
            }
        )
        .unwrap_err(),
        CheckError::SourceLimit
    );
    assert!(CheckedUnit::from_program(
        &parsed,
        EvidenceLimits {
            max_source_bytes: 0,
            ..exact
        }
    )
    .is_ok());
    let empty = CheckedUnit::from_source(
        "",
        EvidenceLimits {
            max_nodes: 0,
            max_source_bytes: 0,
        },
    )
    .unwrap();
    assert!(empty.nodes().is_empty());
}

#[test]
fn equal_synthetic_spans_do_not_alias_structural_ids_or_claim_source_provenance() {
    let mut ast = Parser::new().parse_source("fn main() { 1; }").unwrap();
    let span = Span {
        start: 0,
        end: 0,
        line: 0,
        column: 0,
    };
    ast.functions[0].span = span;
    let Statement::Expression(value) = &mut ast.functions[0].body[0] else {
        unreachable!()
    };
    value.span = span;
    let unit = CheckedUnit::from_program(&ast, limits()).unwrap();
    assert_eq!(unit.source(), None);
    assert_eq!(unit.nodes().len(), 3);
    assert!(unit.nodes().iter().all(|node| node.span == span));
    for (index, node) in unit.nodes().iter().enumerate() {
        assert_eq!(node.id.ordinal(), index);
    }
}

#[test]
fn invalid_and_excess_depth_inputs_keep_existing_diagnostics() {
    for source in ["fn main( {", "fn main() { print(missing); }", "fn f(x) {}"] {
        let expected = Parser::new()
            .parse_source(source)
            .and_then(|ast| SemanticAnalyzer::new().analyze(&ast))
            .unwrap_err();
        assert_eq!(
            CheckedUnit::from_source(source, limits()).unwrap_err(),
            CheckError::Diagnostics(expected)
        );
    }
    let mut ast = Parser::new().parse_source("fn main() { 1; }").unwrap();
    let span = ast.functions[0].span;
    let mut expression = Expression {
        kind: ExpressionKind::Integer("1".into()),
        span,
    };
    for _ in 0..300 {
        expression = Expression {
            kind: ExpressionKind::ArrayLiteral(vec![expression]),
            span,
        };
    }
    ast.functions[0].body = vec![Statement::Expression(expression)];
    let expected = SemanticAnalyzer::new().analyze(&ast).unwrap_err();
    assert_eq!(expected.items[0].code, "E3018");
    assert_eq!(
        CheckedUnit::from_program(&ast, limits()).unwrap_err(),
        CheckError::Diagnostics(expected)
    );
    ast.drop_iterative();
}
