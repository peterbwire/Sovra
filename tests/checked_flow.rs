//! Flow facts preserve existing name resolution and conversion semantics.

use sovra::compiler::checked::{
    CheckError, CheckedUnit, CoercionKind, DeclarationKind, EvidenceError, EvidenceLimits,
    FlowEvidenceLimits, NodeId, RecordedType, SemanticEvidenceLimits, UseKind, UseTarget,
};
use sovra::compiler::{ir, parser::Parser, semantic::SemanticAnalyzer};

fn structural() -> EvidenceLimits {
    EvidenceLimits {
        max_nodes: 8192,
        max_source_bytes: 16384,
    }
}
fn types() -> SemanticEvidenceLimits {
    SemanticEvidenceLimits {
        max_facts: 8192,
        max_types: 4096,
        max_name_bytes: 65536,
    }
}
fn flow_limits() -> FlowEvidenceLimits {
    FlowEvidenceLimits {
        max_declarations: 4096,
        max_uses: 8192,
        max_coercions: 4096,
        max_name_bytes: 65536,
    }
}
fn snapshot(source: &str) -> CheckedUnit {
    CheckedUnit::from_source_with_flow_evidence(source, structural(), types(), flow_limits())
        .unwrap()
}
fn declaration(unit: &CheckedUnit, prefix: &str) -> NodeId {
    let start = unit.source().unwrap().find(prefix).unwrap();
    unit.semantic_evidence()
        .unwrap()
        .flow()
        .unwrap()
        .declarations()
        .iter()
        .find(|d| unit.nodes()[d.node.ordinal()].span.start == start)
        .unwrap()
        .node
}
fn target(unit: &CheckedUnit, start: usize, kind: UseKind) -> UseTarget {
    unit.semantic_evidence()
        .unwrap()
        .flow()
        .unwrap()
        .uses()
        .iter()
        .find(|u| u.kind == kind && unit.nodes()[u.node.ordinal()].span.start == start)
        .unwrap()
        .target
}

#[test]
fn scoped_uses_resolve_initializers_before_bindings_and_restore_every_branch() {
    let source = r#"
fn same(value: Int) -> Int { return value; }
fn main() {
 let mut x = 1;
 if true { let x = x + 1; print(x); }
 else { let x = 2; print(x); }
 while false { let x = x + 2; print(x); }
 x = x + 1; print(x);
 let same = 3; print(same); print(same(4)); std::print(same);
 let mut xs = [1.0]; xs[0] = 2;
}
"#;
    let unit = snapshot(source);
    let outer = declaration(&unit, "let mut x");
    let inner = declaration(&unit, "let x = x + 1");
    let branch = declaration(&unit, "let x = 2");
    let loop_local = declaration(&unit, "let x = x + 2");
    assert_eq!(
        target(&unit, source.find("x + 1").unwrap(), UseKind::Read),
        UseTarget::Declaration(outer)
    );
    assert_eq!(
        target(&unit, source.find("x + 2").unwrap(), UseKind::Read),
        UseTarget::Declaration(outer)
    );
    for (index, decl) in [inner, branch, loop_local, outer].iter().enumerate() {
        let start = source.match_indices("print(x)").nth(index).unwrap().0 + 6;
        assert_eq!(
            target(&unit, start, UseKind::Read),
            UseTarget::Declaration(*decl)
        );
    }
    let assignment = source.rfind("x = x + 1;").unwrap();
    assert_eq!(
        target(&unit, assignment, UseKind::Write),
        UseTarget::Declaration(outer)
    );
    assert_eq!(
        target(&unit, assignment + 4, UseKind::Read),
        UseTarget::Declaration(outer)
    );
    let function = declaration(&unit, "fn same");
    let local = declaration(&unit, "let same");
    assert_eq!(
        target(&unit, source.find("same(4)").unwrap(), UseKind::Call),
        UseTarget::Declaration(function)
    );
    assert_eq!(
        target(
            &unit,
            source.find("print(same)").unwrap() + 6,
            UseKind::Read
        ),
        UseTarget::Declaration(local)
    );
    let parameter = declaration(&unit, "value: Int");
    assert_eq!(
        target(
            &unit,
            source.find("return value").unwrap() + 7,
            UseKind::Read
        ),
        UseTarget::Declaration(parameter)
    );
    let array = declaration(&unit, "let mut xs");
    let index = source.find("xs[0]").unwrap();
    assert_eq!(
        target(&unit, index, UseKind::Read),
        UseTarget::Declaration(array)
    );
    assert_eq!(
        target(&unit, index, UseKind::IndexedWrite),
        UseTarget::Declaration(array)
    );
    let flow = unit.semantic_evidence().unwrap().flow().unwrap();
    let builtins: Vec<_> = flow
        .uses()
        .iter()
        .filter_map(|u| match u.target {
            UseTarget::Builtin(name) => Some(name),
            _ => None,
        })
        .collect();
    assert!(!builtins.is_empty());
    assert!(builtins.iter().all(|name| *name == "std::print"));
    for usage in flow.uses() {
        if let UseTarget::Declaration(id) = usage.target {
            assert!(flow.declarations().iter().any(|decl| decl.node == id));
        }
    }
}

#[test]
fn coercion_edges_cover_existing_scalar_array_and_context_requirements() {
    let source = r#"
struct Point { x: Float }
fn takes(v: Float) -> Float { return v; }
fn returns() -> Float { return 1; }
fn main() {
 let mut x: Float = 1; x = 2;
 let mut xs = [0.0]; xs[0] = 3;
 let ints = [[1]]; let mut floats = [[0.0]]; floats = ints;
 let p = Point { x: 4 };
 let mix = [1, 2.0]; let nested = [[2.0], [1]];
 print(takes(5)); print(1 + 2.0); print(1 < 2.0); print(1 == 2.0);
 let identity: Float = 2.0; let hole = [[]];
}
"#;
    let unit = snapshot(source);
    let evidence = unit.semantic_evidence().unwrap();
    let flow = evidence.flow().unwrap();
    let actual: Vec<_> = flow
        .coercions()
        .iter()
        .map(|edge| {
            let span = unit.nodes()[edge.value.ordinal()].span;
            (edge.kind, &source[span.start..span.end], edge.array_depth)
        })
        .collect();
    assert_eq!(
        actual,
        [
            (CoercionKind::Return, "1", 0),
            (CoercionKind::Initialization, "1", 0),
            (CoercionKind::Assignment, "2", 0),
            (CoercionKind::IndexedAssignment, "3", 0),
            (CoercionKind::Assignment, "ints", 2),
            (CoercionKind::RecordField, "4", 0),
            (CoercionKind::ArrayElement, "1", 0),
            (CoercionKind::ArrayElement, "[1]", 1),
            (CoercionKind::Argument, "5", 0),
            (CoercionKind::BinaryOperand, "1", 0),
            (CoercionKind::BinaryOperand, "1", 0),
            (CoercionKind::BinaryOperand, "1", 0),
        ]
    );
    for kind in [
        CoercionKind::Initialization,
        CoercionKind::Assignment,
        CoercionKind::IndexedAssignment,
        CoercionKind::Return,
        CoercionKind::Argument,
        CoercionKind::RecordField,
        CoercionKind::ArrayElement,
        CoercionKind::BinaryOperand,
    ] {
        assert!(
            flow.coercions().iter().any(|edge| edge.kind == kind),
            "{kind:?}"
        );
    }
    for edge in flow.coercions() {
        let mut from = edge.from;
        let mut to = edge.to;
        for _ in 0..edge.array_depth {
            let RecordedType::Array(child) = evidence.types()[from.ordinal()] else {
                panic!("array source")
            };
            from = child;
            let RecordedType::Array(child) = evidence.types()[to.ordinal()] else {
                panic!("array target")
            };
            to = child;
        }
        assert_eq!(evidence.types()[from.ordinal()], RecordedType::Int);
        assert_eq!(evidence.types()[to.ordinal()], RecordedType::Float);
        assert!(edge.owner.ordinal() < unit.nodes().len());
        assert!(edge.value.ordinal() < unit.nodes().len());
    }
    assert!(flow
        .coercions()
        .iter()
        .any(|edge| edge.kind == CoercionKind::Assignment && edge.array_depth == 2));
    assert!(flow
        .coercions()
        .iter()
        .any(|edge| edge.kind == CoercionKind::ArrayElement && edge.array_depth == 1));
    let identity = source.find("let identity").unwrap();
    let hole = source.find("let hole").unwrap();
    assert!(!flow
        .coercions()
        .iter()
        .any(|edge| [identity, hole].contains(&unit.nodes()[edge.owner.ordinal()].span.start)));
    let parsed = Parser::new().parse_source(source).unwrap();
    let legacy = SemanticAnalyzer::new().analyze(&parsed).unwrap();
    assert_eq!(unit.program(), &legacy.program);
    let detached = sovra::compiler::semantic::TypedProgram {
        program: unit.program().clone(),
    };
    assert_eq!(ir::lower(&detached), ir::lower(&legacy));
}

#[test]
fn module_targets_and_same_spelled_declarations_have_structural_identities() {
    let source = "mod m { export fn value(x: Int) -> Int { return x; } } \
        fn value(x: Int) -> Int { return x; } fn main() { print(m::value(1)); print(value(2)); }";
    let mut ast = Parser::new().parse_source(source).unwrap();
    let span = ast.functions[0].span;
    ast.modules[0].functions[0].span = span;
    let unit =
        CheckedUnit::from_program_with_flow_evidence(&ast, structural(), types(), flow_limits())
            .unwrap();
    assert!(unit.source().is_none());
    let flow = unit.semantic_evidence().unwrap().flow().unwrap();
    let declarations: Vec<_> = flow
        .declarations()
        .iter()
        .filter(|d| d.name.as_ref() == "value" && d.kind == DeclarationKind::Function)
        .collect();
    assert_eq!(declarations.len(), 2);
    assert_ne!(declarations[0].node, declarations[1].node);
    let calls: Vec<_> = flow
        .uses()
        .iter()
        .filter_map(|u| match u.target {
            UseTarget::Declaration(id) if u.kind == UseKind::Call => Some(id),
            _ => None,
        })
        .collect();
    assert_eq!(calls, [declarations[1].node, declarations[0].node]);
}

#[test]
fn exact_and_overflow_flow_limits_are_atomic_and_preserve_old_type_only_api() {
    let source =
        "fn f(v: Float) -> Float { return v; } fn main() { let x: Float = 1; print(f(2)); }";
    let unit = snapshot(source);
    let flow = unit.semantic_evidence().unwrap().flow().unwrap();
    let exact = FlowEvidenceLimits {
        max_declarations: flow.declarations().len(),
        max_uses: flow.uses().len(),
        max_coercions: flow.coercions().len(),
        max_name_bytes: flow.name_bytes(),
    };
    // f/main/v/x; return-v read plus print/f calls; initializer and argument widening.
    assert_eq!(
        exact,
        FlowEvidenceLimits {
            max_declarations: 4,
            max_uses: 3,
            max_coercions: 2,
            max_name_bytes: 7,
        }
    );
    assert!(
        CheckedUnit::from_source_with_flow_evidence(source, structural(), types(), exact).is_ok()
    );
    for limit in [
        FlowEvidenceLimits {
            max_declarations: exact.max_declarations - 1,
            ..exact
        },
        FlowEvidenceLimits {
            max_uses: exact.max_uses - 1,
            ..exact
        },
        FlowEvidenceLimits {
            max_coercions: exact.max_coercions - 1,
            ..exact
        },
        FlowEvidenceLimits {
            max_name_bytes: exact.max_name_bytes - 1,
            ..exact
        },
        FlowEvidenceLimits {
            max_declarations: 0,
            ..exact
        },
        FlowEvidenceLimits {
            max_uses: 0,
            ..exact
        },
        FlowEvidenceLimits {
            max_coercions: 0,
            ..exact
        },
        FlowEvidenceLimits {
            max_name_bytes: 0,
            ..exact
        },
    ] {
        assert_eq!(
            CheckedUnit::from_source_with_flow_evidence(source, structural(), types(), limit)
                .unwrap_err(),
            EvidenceError::Limit
        );
    }
    let plain = CheckedUnit::from_source_with_evidence(source, structural(), types()).unwrap();
    assert!(plain.semantic_evidence().unwrap().flow().is_none());
    assert_eq!(plain.program(), unit.program());
    let empty = CheckedUnit::from_source_with_flow_evidence(
        "",
        structural(),
        types(),
        FlowEvidenceLimits {
            max_declarations: 0,
            max_uses: 0,
            max_coercions: 0,
            max_name_bytes: 0,
        },
    )
    .unwrap();
    assert!(empty
        .semantic_evidence()
        .unwrap()
        .flow()
        .unwrap()
        .declarations()
        .is_empty());
    let clone = unit.clone();
    drop(unit);
    assert_eq!(
        clone
            .semantic_evidence()
            .unwrap()
            .flow()
            .unwrap()
            .uses()
            .len(),
        exact.max_uses
    );
}

#[test]
fn failed_sources_never_expose_partial_flow_and_keep_diagnostic_precedence() {
    for source in [
        "fn main() { let x = 1; print(missing); }",
        "fn main() { let x = 1; x = 2; }",
        "fn main() { missing(); }",
        "fn f(x) {}",
    ] {
        let expected = Parser::new()
            .parse_source(source)
            .and_then(|ast| SemanticAnalyzer::new().analyze(&ast))
            .unwrap_err();
        for limits in [
            flow_limits(),
            FlowEvidenceLimits {
                max_declarations: 0,
                max_uses: 0,
                max_coercions: 0,
                max_name_bytes: 0,
            },
        ] {
            assert_eq!(
                CheckedUnit::from_source_with_flow_evidence(source, structural(), types(), limits)
                    .unwrap_err(),
                EvidenceError::Check(CheckError::Diagnostics(expected.clone()))
            );
        }
    }
}

#[test]
fn contextual_types_share_type_limits_and_leading_trivia_preserves_flow_identities() {
    let source = "fn f() -> Float { return 1; } fn main() { let x: Float = 2; print(f()); }";
    let unit = snapshot(source);
    let evidence = unit.semantic_evidence().unwrap();
    let exact = SemanticEvidenceLimits {
        max_facts: evidence.facts().len(),
        max_types: evidence.types().len(),
        max_name_bytes: evidence.name_bytes(),
    };
    assert!(CheckedUnit::from_source_with_flow_evidence(
        source,
        structural(),
        exact,
        flow_limits()
    )
    .is_ok());
    assert_eq!(
        CheckedUnit::from_source_with_flow_evidence(
            source,
            structural(),
            SemanticEvidenceLimits {
                max_types: exact.max_types - 1,
                ..exact
            },
            flow_limits()
        )
        .unwrap_err(),
        EvidenceError::Limit
    );
    let prefixed = format!("// λ😀 e\u{301}\r\n\t{source}");
    let changed = snapshot(&prefixed);
    let newer = changed.semantic_evidence().unwrap();
    assert_eq!(evidence.types(), newer.types());
    assert_eq!(evidence.facts(), newer.facts());
    let old_flow = evidence.flow().unwrap();
    let new_flow = newer.flow().unwrap();
    assert_eq!(old_flow.declarations(), new_flow.declarations());
    assert_eq!(old_flow.uses(), new_flow.uses());
    assert_eq!(old_flow.coercions(), new_flow.coercions());
    let prefix_bytes = prefixed.len() - source.len();
    for (old, new) in unit.nodes().iter().zip(changed.nodes()) {
        assert_eq!(old.id, new.id);
        assert_eq!(new.span.start, old.span.start + prefix_bytes);
        assert_eq!(new.span.end, old.span.end + prefix_bytes);
    }
}
