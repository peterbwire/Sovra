//! Explicit call envelopes with grammar-checked types and opaque balanced values.
use sovra::compiler::v2::*;
fn parse(source: &str) -> GenericCallSyntax {
    parse_generic_call(source, GenericCallSyntaxLimits::DEFAULT).unwrap()
}
#[test]
fn root_and_qualified_callees_keep_original_unicode_ranges() {
    let source = "// λ😀\n pkg :: identity ::< [T],Int >(value, [1,2]) ";
    let syntax = parse(source);
    assert_eq!(syntax.source(), source);
    assert_eq!(syntax.callee(), "pkg::identity");
    assert_eq!(
        &source[syntax.callee_span().start..syntax.callee_span().end],
        "pkg :: identity"
    );
    assert_eq!(
        &source[syntax.span().start..syntax.span().end],
        "pkg :: identity ::< [T],Int >(value, [1,2])"
    );
    assert_eq!(syntax.span().line, 1);
    let values: Vec<_> = syntax
        .value_ranges()
        .iter()
        .map(|s| &source[s.start..s.end])
        .collect();
    assert_eq!(values, ["value", "[1,2]"]);
    let types = syntax.type_arguments();
    assert_eq!(types.source(), source);
    assert_eq!(
        &source[types.span().start..types.span().end],
        "::< [T],Int >"
    );
    let root = &types.nodes()[types.roots()[0]];
    assert_eq!(&source[root.span.start..root.span.end], "[T]");
}
#[test]
fn nested_calls_records_literals_and_comparisons_do_not_split_inner_commas() {
    let source = "call::<T>(other::<Int,Bool>(1,true), Box{a:1,b:[2,3]}, x<y, \",])}\")";
    let syntax = parse(source);
    assert_eq!(syntax.value_ranges().len(), 4);
    assert!(parse("call::<T>()").value_ranges().is_empty());
}
#[test]
fn opaque_values_do_not_claim_expression_validity_or_callee_resolution() {
    assert_eq!(
        parse("missing::<Unknown>(let not an expression)")
            .value_ranges()
            .len(),
        1
    );
    assert!(check_program_source(
        "strict",
        "fn main(){missing::<Int>(1);}",
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
}
#[test]
fn malformed_envelopes_mismatched_delimiters_and_trailing_input_are_rejected() {
    for source in [
        "",
        "call()",
        "call::<>()",
        "call::<T>",
        "call::<T>(,x)",
        "call::<T>(x,)",
        "call::<T>([x)",
        "call::<T>({x])",
        "call::<T>(x) tail",
        "call::<T>(x);",
        "call::",
        "call::<T>(other::<Int,Bool(x))",
    ] {
        assert!(
            parse_generic_call(source, GenericCallSyntaxLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
}
#[test]
fn whole_call_name_value_token_source_and_depth_limits_are_independent() {
    let source = "pkg::call::<[T]>(x,[y])";
    let syntax = parse(source);
    let exact = GenericCallSyntaxLimits {
        max_source_bytes: source.len(),
        max_tokens: syntax.token_count(),
        max_name_bytes: syntax.callee().len(),
        max_values: 2,
        max_depth: 2,
        ..GenericCallSyntaxLimits::DEFAULT
    };
    assert!(parse_generic_call(source, exact).is_ok());
    for limits in [
        GenericCallSyntaxLimits {
            max_source_bytes: source.len() - 1,
            ..exact
        },
        GenericCallSyntaxLimits {
            max_tokens: syntax.token_count() - 1,
            ..exact
        },
        GenericCallSyntaxLimits {
            max_name_bytes: syntax.callee().len() - 1,
            ..exact
        },
        GenericCallSyntaxLimits {
            max_values: 1,
            ..exact
        },
        GenericCallSyntaxLimits {
            max_depth: 1,
            ..exact
        },
        GenericCallSyntaxLimits {
            max_depth: 0,
            ..exact
        },
        GenericCallSyntaxLimits {
            max_depth: 129,
            ..exact
        },
        GenericCallSyntaxLimits {
            types: GenericTypeArgumentLimits {
                max_arguments: 0,
                ..exact.types
            },
            ..exact
        },
    ] {
        assert!(parse_generic_call(source, limits).is_err());
    }
}
#[test]
fn type_errors_are_translated_to_original_call_ranges_and_nested_depth_is_bounded() {
    let source = "// λ😀\ncall::<Int,>(x)";
    let GenericTypeError::Syntax { span, .. } =
        parse_generic_call(source, GenericCallSyntaxLimits::DEFAULT).unwrap_err()
    else {
        panic!()
    };
    assert_eq!(&source[span.start..span.end], ">");
    assert_eq!(span.line, 1);
    let source = format!("call::<T>({}x{})", "[".repeat(128), "]".repeat(128));
    assert!(parse_generic_call(&source, GenericCallSyntaxLimits::DEFAULT).is_err());
}
#[test]
fn relocated_type_arguments_feed_caller_scope_checking_without_source_rewrites() {
    let registry = register_generic_types(&[], GenericRegistryLimits::DEFAULT).unwrap();
    let env =
        prepare_generic_aliases(&registry, &[], GenericAliasEnvironmentLimits::DEFAULT).unwrap();
    let types = check_generic_declarations(&env, &[], GenericDeclarationLimits::DEFAULT).unwrap();
    let parameters = parse_generic_parameters("<T>", GenericParameterLimits::DEFAULT).unwrap();
    let call = parse("// λ😀\ncall::<[T]>(value)");
    let checked = check_generic_type_arguments(
        &types,
        call.type_arguments(),
        &parameters,
        GenericArgumentCheckLimits::DEFAULT,
    )
    .unwrap();
    let ty = &checked.arguments()[0];
    let node = &ty.nodes()[ty.root()];
    assert_eq!(
        &ty.node_source(ty.root()).unwrap()[node.origin.span.start..node.origin.span.end],
        "[T]"
    );
    let call = parse("// λ😀\ncall::<Missing>(value)");
    let GenericArgumentCheckError::Argument { span, .. } = check_generic_type_arguments(
        &types,
        call.type_arguments(),
        &parameters,
        GenericArgumentCheckLimits::DEFAULT,
    )
    .unwrap_err() else {
        panic!()
    };
    assert_eq!(&call.source()[span.start..span.end], "Missing");
    assert_eq!(span.line, 1);
}
