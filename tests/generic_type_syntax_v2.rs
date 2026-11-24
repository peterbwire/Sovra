//! Approved E2 annotation grammar, without unchecked semantic/executable acceptance.
use sovra::compiler::{lexer::Lexer, v2::*};
fn parse(source: &str) -> GenericTypeSyntax {
    parse_generic_type(source, GenericTypeLimits::DEFAULT).unwrap()
}
#[test]
fn qualified_nested_applications_and_arrays_retain_original_spans_and_postorder() {
    let source = "// λ😀\n std :: Result < [pkg::Entry<Int, String>], std::Error > ";
    let syntax = parse(source);
    assert_eq!(syntax.source(), source);
    let nodes = syntax.nodes();
    assert_eq!(nodes.len(), 6);
    assert_eq!(syntax.edges(), 5);
    assert_eq!(
        source.get(nodes[0].span.start..nodes[0].span.end),
        Some("Int")
    );
    assert_eq!(nodes[0].span.line, 1);
    assert_eq!(
        source.get(syntax.root().span.start..syntax.root().span.end),
        Some("std :: Result < [pkg::Entry<Int, String>], std::Error >")
    );
    assert_eq!(
        syntax.root().kind,
        GenericTypeKind::Named {
            name: "std::Result".into(),
            arguments: vec![3, 4]
        }
    );
    for (index, node) in nodes.iter().enumerate() {
        match &node.kind {
            GenericTypeKind::Named { arguments, .. } => {
                assert!(arguments.iter().all(|child| *child < index))
            }
            GenericTypeKind::Array { element } => assert!(*element < index),
        }
    }
}
#[test]
fn malformed_or_non_type_forms_never_return_partial_success() {
    for source in [
        "",
        "[]",
        "A<>",
        "A<Int,>",
        "A<,Int>",
        "A<Int",
        "A<Int>>",
        "A<Int String>",
        "A::",
        "A::<Int>",
        "[Int",
        "Int]",
        "Int String",
        "fn x<T>(){}",
        "T:std::Key",
        "A(7)",
        "<Int>",
        "A<int>=x",
        "A<@>",
    ] {
        assert!(
            parse_generic_type(source, GenericTypeLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
}
#[test]
fn exact_source_token_node_edge_and_text_limits_admit_and_one_under_refuses() {
    let source = "std::Result<[Int], E>";
    let syntax = parse(source);
    let exact = GenericTypeLimits {
        max_source_bytes: source.len(),
        max_tokens: Lexer::new().tokenize(source).unwrap().len(),
        max_nodes: syntax.nodes().len(),
        max_edges: syntax.edges(),
        max_name_bytes: syntax.name_bytes(),
        max_depth: 3,
    };
    assert!(parse_generic_type(source, exact).is_ok());
    for limits in [
        GenericTypeLimits {
            max_source_bytes: exact.max_source_bytes - 1,
            ..exact
        },
        GenericTypeLimits {
            max_tokens: exact.max_tokens - 1,
            ..exact
        },
        GenericTypeLimits {
            max_nodes: exact.max_nodes - 1,
            ..exact
        },
        GenericTypeLimits {
            max_edges: exact.max_edges - 1,
            ..exact
        },
        GenericTypeLimits {
            max_name_bytes: exact.max_name_bytes - 1,
            ..exact
        },
        GenericTypeLimits {
            max_depth: exact.max_depth - 1,
            ..exact
        },
    ] {
        assert!(parse_generic_type(source, limits).is_err(), "{limits:?}");
    }
}
#[test]
fn depth_and_width_are_independently_bounded_including_pending_parent_nodes() {
    let deep = format!("{}Int{}", "[".repeat(127), "]".repeat(127));
    assert_eq!(parse(&deep).nodes().len(), 128);
    let too_deep = format!("[{deep}]");
    assert!(parse_generic_type(&too_deep, GenericTypeLimits::DEFAULT).is_err());
    assert!(parse_generic_type(
        "Int",
        GenericTypeLimits {
            max_depth: 129,
            ..GenericTypeLimits::DEFAULT
        }
    )
    .is_err());
    let wide = format!("Map<{}>", vec!["Int"; 100].join(","));
    assert_eq!(parse(&wide).nodes().len(), 101);
    assert!(parse_generic_type(
        &wide,
        GenericTypeLimits {
            max_nodes: 100,
            ..GenericTypeLimits::DEFAULT
        }
    )
    .is_err());
    assert!(parse_generic_type(
        "[[Int]]",
        GenericTypeLimits {
            max_nodes: 2,
            ..GenericTypeLimits::DEFAULT
        }
    )
    .is_err());
}
#[test]
fn unknown_names_and_arity_are_syntax_only_and_existing_source_checkers_stay_strict() {
    assert_eq!(parse("Unknown<Int,Int,Int>").nodes().len(), 4);
    assert!(check_program_source(
        "strict",
        "fn identity<T>(value:T)->T{return value;}fn main(){}",
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
    assert!(check_program_source(
        "strict",
        "fn main(){let x:Unknown<Int>=1;}",
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
}
#[test]
fn zero_budgets_refuse_without_fallback_and_bare_names_need_no_edges() {
    for limits in [
        GenericTypeLimits {
            max_nodes: 0,
            ..GenericTypeLimits::DEFAULT
        },
        GenericTypeLimits {
            max_name_bytes: 0,
            ..GenericTypeLimits::DEFAULT
        },
        GenericTypeLimits {
            max_depth: 0,
            ..GenericTypeLimits::DEFAULT
        },
    ] {
        assert!(parse_generic_type("Int", limits).is_err());
    }
    assert!(parse_generic_type(
        "Int",
        GenericTypeLimits {
            max_edges: 0,
            ..GenericTypeLimits::DEFAULT
        }
    )
    .is_ok());
    assert!(parse_generic_type(
        "[Int]",
        GenericTypeLimits {
            max_edges: 0,
            ..GenericTypeLimits::DEFAULT
        }
    )
    .is_err());
}
