//! Validated structural ownership for checked callable facts, without IR execution.
use sovra::compiler::{
    ast::{ExpressionKind, Statement},
    parser::Parser,
    v2::{
        check_enum_types, CheckedEnumTypes, EnumSource, EnumTypeLimits,
        MappedExpressionRole as Role, MappedFact as Fact, MappedSyntax as Syntax,
        MappingError as Error, MappingLimits as Limits, ScopedCallableLimits,
    },
};
fn env() -> CheckedEnumTypes {
    check_enum_types(
        "owner",
        "types",
        &Parser::new().parse_source("struct Item{x:Int}").unwrap(),
        &[EnumSource {
            label: "enum",
            source: "enum E{V(Float),Rows([[Float]]),Empty}",
        }],
        EnumTypeLimits::DEFAULT,
    )
    .unwrap()
}
#[test]
fn every_checked_fact_maps_once_and_parent_ids_are_structural_preorder() {
    let env = env();
    let source="fn f(input:E)->Float{let mut rows=[1.0];rows[0]=2;match input{E::V(n)=>{return n;},E::Rows(r)=>{return r[0][0];},E::Empty()=>{return rows[0];}}}";
    let checked = env
        .check_scoped_callable_function_source("f", source, ScopedCallableLimits::DEFAULT)
        .unwrap();
    let mapped = checked.map_for_lowering(Limits::DEFAULT).unwrap();
    assert!(matches!(mapped.nodes()[0].syntax, Syntax::Function(_)));
    assert!(mapped.nodes()[0].parent.is_none());
    for (ordinal, node) in mapped.nodes().iter().enumerate() {
        assert_eq!(node.id.ordinal(), ordinal);
        assert!(mapped.node(node.id).is_some());
        if ordinal > 0 {
            assert!(node.parent.unwrap().ordinal() < ordinal);
        }
    }
    let t = checked.typed();
    let expected = t.types().expressions().len()
        + t.callable().evidence().calls().len()
        + t.callable().evidence().contexts().len()
        + checked.bindings().declarations().len()
        + checked.bindings().uses().len();
    assert_eq!(mapped.links().len(), expected);
    for link in mapped.links() {
        let node = mapped.node(link.node).unwrap();
        match link.fact {
            Fact::Expression(id) => {
                assert_eq!(node.syntax.span(), t.types().expression(id).unwrap().span)
            }
            Fact::Declaration(id) => assert_eq!(
                node.syntax.span(),
                checked.bindings().declaration(id).unwrap().span
            ),
            Fact::Use(id) => assert_eq!(
                node.syntax.span(),
                checked.bindings().usage(id).unwrap().span
            ),
            Fact::Context(i) => assert_eq!(
                node.syntax.span(),
                t.callable().evidence().contexts()[i].value_span
            ),
            Fact::Call(_) => assert!(matches!(
                node.syntax,
                Syntax::Expression {
                    syntax: sovra::compiler::ast::Expression {
                        kind: ExpressionKind::Call { .. },
                        ..
                    },
                    role: Role::Value
                }
            )),
        }
    }
}
#[test]
fn call_heads_and_direct_or_indexed_writes_are_not_fabricated_typed_values() {
    let checked = env()
        .check_scoped_callable_function_source(
            "f",
            "fn f()->E{let mut rows=[1];let mut n=0;n=1;rows[n]=2;return E::V(n);}",
            ScopedCallableLimits::DEFAULT,
        )
        .unwrap();
    let mapped = checked.map_for_lowering(Limits::DEFAULT).unwrap();
    for role in [Role::CallHead, Role::Write, Role::IndexedWrite] {
        let nodes: Vec<_> = mapped
            .nodes()
            .iter()
            .filter(|n| matches!(n.syntax,Syntax::Expression {role:r,..} if r==role))
            .collect();
        assert!(!nodes.is_empty());
        for node in nodes {
            assert!(!mapped
                .links()
                .iter()
                .any(|l| l.node == node.id && matches!(l.fact, Fact::Expression(_))));
        }
    }
}
#[test]
fn reordered_arms_multiple_matches_and_payload_declarations_keep_source_structure() {
    let source="fn f(input:E){match input{E::Empty()=>{},E::Rows(r)=>{print(r);},E::V(n)=>{print(n);}}match input{E::V(n)=>{print(n);},E::Rows(r)=>{},E::Empty()=>{}}}";
    let checked = env()
        .check_scoped_callable_function_source("f", source, ScopedCallableLimits::DEFAULT)
        .unwrap();
    let mapped = checked.map_for_lowering(Limits::DEFAULT).unwrap();
    let arms: Vec<_> = mapped
        .nodes()
        .iter()
        .filter_map(|n| match n.syntax {
            Syntax::Arm {
                match_index,
                variant,
                ..
            } => Some((match_index, variant)),
            _ => None,
        })
        .collect();
    assert_eq!(arms, vec![(0, 2), (0, 1), (0, 0), (1, 0), (1, 1), (1, 2)]);
    assert_eq!(
        mapped
            .nodes()
            .iter()
            .filter(|n| matches!(n.syntax, Syntax::Scrutinee { .. }))
            .count(),
        2
    );
    assert_eq!(
        mapped
            .nodes()
            .iter()
            .filter(|n| matches!(n.syntax, Syntax::Payload { .. }))
            .count(),
        4
    );
}
#[test]
fn nested_core_nodes_mixed_array_contexts_and_record_fields_map_without_new_inference() {
    let source="fn f(n:Int)->E{let item=Item{x:n};if n>0{return f(n-1);}else{while false{print(item.x);}return E::Rows([[1,2.0],[3,4]]);}}";
    let checked = env()
        .check_scoped_callable_function_source("f", source, ScopedCallableLimits::DEFAULT)
        .unwrap();
    let mapped = checked.map_for_lowering(Limits::DEFAULT).unwrap();
    assert!(mapped
        .nodes()
        .iter()
        .any(|n| matches!(n.syntax, Syntax::Statement(Statement::While { .. }))));
    assert!(mapped.nodes().iter().any(|n| matches!(
        n.syntax,
        Syntax::Expression {
            syntax: sovra::compiler::ast::Expression {
                kind: ExpressionKind::StructLiteral { .. },
                ..
            },
            ..
        }
    )));
    assert_eq!(
        mapped
            .links()
            .iter()
            .filter(|l| matches!(l.fact, Fact::Context(_)))
            .count(),
        checked.typed().callable().evidence().contexts().len()
    );
}
#[test]
fn original_unicode_crlf_spans_and_grouped_expressions_are_preserved() {
    let source = "// λ😀\r\nfn f(n:Int)->E{\r\nlet x=(n+1);\r\nreturn E::V(x);\r\n}";
    let checked = env()
        .check_scoped_callable_function_source("f", source, ScopedCallableLimits::DEFAULT)
        .unwrap();
    let mapped = checked.map_for_lowering(Limits::DEFAULT).unwrap();
    let grouped = mapped
        .nodes()
        .iter()
        .find(|n| &source[n.syntax.span().start..n.syntax.span().end] == "(n+1)")
        .unwrap();
    assert_eq!(grouped.syntax.span().line, 2);
    assert!(mapped
        .links()
        .iter()
        .any(|l| l.node == grouped.id && matches!(l.fact, Fact::Expression(_))));
}
#[test]
fn mapping_limits_are_exact_zero_is_real_and_errors_do_not_poison_other_maps() {
    let checked = env()
        .check_scoped_callable_function_source(
            "f",
            "fn f(n:Int)->Int{return n;}",
            ScopedCallableLimits::DEFAULT,
        )
        .unwrap();
    let exact = Limits {
        max_nodes: 4,
        max_fact_links: 4,
    };
    let mapped = checked.map_for_lowering(exact).unwrap();
    assert_eq!(mapped.nodes().len(), 4);
    assert_eq!(mapped.links().len(), 4);
    assert!(matches!(
        checked.map_for_lowering(Limits {
            max_nodes: 3,
            ..exact
        }),
        Err(Error::Limit("nodes"))
    ));
    assert!(matches!(
        checked.map_for_lowering(Limits {
            max_fact_links: 3,
            ..exact
        }),
        Err(Error::Limit("fact links"))
    ));
    assert!(checked.map_for_lowering(exact).is_ok());
    let empty = env()
        .check_scoped_callable_function_source("f", "fn f(){}", ScopedCallableLimits::DEFAULT)
        .unwrap();
    assert!(empty
        .map_for_lowering(Limits {
            max_nodes: 1,
            max_fact_links: 0
        })
        .is_ok());
    assert!(matches!(
        empty.map_for_lowering(Limits {
            max_nodes: 0,
            max_fact_links: 0
        }),
        Err(Error::Limit("nodes"))
    ));
}
#[test]
fn mapping_ids_are_local_and_clones_preserve_original_mapping_identity() {
    let checked = env()
        .check_scoped_callable_function_source("f", "fn f(){}", ScopedCallableLimits::DEFAULT)
        .unwrap();
    let a = checked.map_for_lowering(Limits::DEFAULT).unwrap();
    let b = checked.map_for_lowering(Limits::DEFAULT).unwrap();
    let id = a.nodes()[0].id;
    assert!(b.node(id).is_none());
    assert!(a.clone().node(id).is_some());
}
