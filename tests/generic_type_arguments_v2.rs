//! Explicit generic call/constructor argument syntax, independent of execution.
use sovra::compiler::v2::*;
fn parse(source: &str) -> GenericTypeArgumentsSyntax {
    parse_generic_type_arguments(source, GenericTypeArgumentLimits::DEFAULT).unwrap()
}
#[test]
fn nested_qualified_arguments_retain_roots_postorder_and_original_source() {
    let source = "// λ😀\n ::<Int, std::Result<[T], Error>> ";
    let syntax = parse(source);
    assert_eq!(syntax.roots().len(), 2);
    assert_eq!(syntax.source(), source);
    assert_eq!(
        &source[syntax.span().start..syntax.span().end],
        "::<Int, std::Result<[T], Error>>"
    );
    assert_eq!(syntax.span().line, 1);
    for (index, node) in syntax.nodes().iter().enumerate() {
        match &node.kind {
            GenericTypeKind::Array { element } => assert!(*element < index),
            GenericTypeKind::Named { arguments, .. } => {
                assert!(arguments.iter().all(|child| *child < index))
            }
        }
    }
    let node = &syntax.nodes()[syntax.roots()[1]];
    assert_eq!(
        &source[node.span.start..node.span.end],
        "std::Result<[T], Error>"
    );
}
#[test]
fn syntax_does_not_assume_names_arity_bounds_or_callee_existence() {
    assert_eq!(parse("::<Missing<T>, Unknown>").roots().len(), 2);
    assert!(check_program_source(
        "strict",
        "fn identity<T>(x:T)->T{return x;}fn main(){identity::<Int>(1);}",
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
}
#[test]
fn malformed_empty_trailing_and_expression_forms_refuse_without_partial_output() {
    for source in [
        "",
        "<Int>",
        "::<>",
        "::<Int,>",
        "::<,Int>",
        "::<Int",
        "::<Int>(1)",
        "::<1>",
        "::<[T,T]>",
        "::<T:std::Key>",
        "::<Int> extra",
    ] {
        assert!(
            parse_generic_type_arguments(source, GenericTypeArgumentLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
}
#[test]
fn argument_and_shared_syntax_limits_are_exact_and_cumulative() {
    let source = "::<Pair<Int,[T]>,String>";
    let syntax = parse(source);
    let exact = GenericTypeArgumentLimits {
        max_arguments: 2,
        types: GenericTypeLimits {
            max_source_bytes: source.len(),
            max_tokens: syntax.token_count(),
            max_nodes: syntax.nodes().len(),
            max_edges: syntax.edges(),
            max_name_bytes: syntax.name_bytes(),
            max_depth: 3,
        },
    };
    assert!(parse_generic_type_arguments(source, exact).is_ok());
    for types in [
        GenericTypeLimits {
            max_source_bytes: source.len() - 1,
            ..exact.types
        },
        GenericTypeLimits {
            max_tokens: syntax.token_count() - 1,
            ..exact.types
        },
        GenericTypeLimits {
            max_nodes: syntax.nodes().len() - 1,
            ..exact.types
        },
        GenericTypeLimits {
            max_edges: syntax.edges() - 1,
            ..exact.types
        },
        GenericTypeLimits {
            max_name_bytes: syntax.name_bytes() - 1,
            ..exact.types
        },
        GenericTypeLimits {
            max_depth: 2,
            ..exact.types
        },
    ] {
        assert!(
            parse_generic_type_arguments(source, GenericTypeArgumentLimits { types, ..exact })
                .is_err()
        );
    }
    assert!(parse_generic_type_arguments(
        source,
        GenericTypeArgumentLimits {
            max_arguments: 1,
            ..exact
        }
    )
    .is_err());
}
#[test]
fn zero_and_excessive_depth_limits_never_fall_back() {
    for limits in [
        GenericTypeArgumentLimits {
            max_arguments: 0,
            ..GenericTypeArgumentLimits::DEFAULT
        },
        GenericTypeArgumentLimits {
            types: GenericTypeLimits {
                max_depth: 0,
                ..GenericTypeLimits::DEFAULT
            },
            ..GenericTypeArgumentLimits::DEFAULT
        },
        GenericTypeArgumentLimits {
            types: GenericTypeLimits {
                max_depth: 129,
                ..GenericTypeLimits::DEFAULT
            },
            ..GenericTypeArgumentLimits::DEFAULT
        },
    ] {
        assert!(parse_generic_type_arguments("::<T>", limits).is_err());
    }
}
#[test]
fn bounded_diagnostics_keep_original_unicode_coordinates() {
    let source = "// λ😀\n::<Int,>";
    let GenericTypeError::Syntax { span, .. } =
        parse_generic_type_arguments(source, GenericTypeArgumentLimits::DEFAULT).unwrap_err()
    else {
        panic!()
    };
    assert_eq!(&source[span.start..span.end], ">");
    assert_eq!(span.line, 1);
    let deep = format!("::<{}T{}>", "[".repeat(129), "]".repeat(129));
    assert!(parse_generic_type_arguments(&deep, GenericTypeArgumentLimits::DEFAULT).is_err());
}
