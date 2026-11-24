//! Original-token expression parsing for explicit generic call value ranges.
use sovra::compiler::{ast::ExpressionKind, v2::*};
fn syntax(source: &str) -> GenericCallSyntax {
    parse_generic_call(source, GenericCallSyntaxLimits::DEFAULT).unwrap()
}
fn parse(source: &str) -> ParsedGenericCallExpressions {
    parse_generic_call_expressions(&syntax(source), GenericCallExpressionLimits::DEFAULT).unwrap()
}
#[test]
fn values_parse_with_original_unicode_spans_and_operator_precedence() {
    let source = "// λ😀\ncall::<T>(x+2*3, [\"λ😀\",\"z\"], box.value[0])";
    let parsed = parse(source);
    assert_eq!(parsed.expressions().len(), 3);
    for (expression, range) in parsed
        .expressions()
        .iter()
        .zip(parsed.syntax().value_ranges())
    {
        assert_eq!(expression.span, *range);
    }
    let ExpressionKind::Binary {
        operator, right, ..
    } = &parsed.expressions()[0].kind
    else {
        panic!()
    };
    assert_eq!(operator, "+");
    assert!(matches!(&right.kind,ExpressionKind::Binary{operator,..} if operator=="*"));
    assert_eq!(
        &source[parsed.expressions()[1].span.start..parsed.expressions()[1].span.end],
        "[\"λ😀\",\"z\"]"
    );
}
#[test]
fn opaque_envelopes_with_invalid_expression_grammar_fail_here() {
    for source in [
        "call::<T>(let x)",
        "call::<T>(x y)",
        "call::<T>(x=1)",
        "call::<T>(1;)",
        "call::<T>(1+)",
    ] {
        let call = syntax(source);
        assert!(
            parse_generic_call_expressions(&call, GenericCallExpressionLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
}
#[test]
fn unknown_names_and_plain_calls_are_syntax_not_type_or_callee_proofs() {
    let parsed = parse("call::<Missing>(unknown, helper(x))");
    assert!(matches!(
        parsed.expressions()[0].kind,
        ExpressionKind::Identifier(_)
    ));
    assert!(matches!(
        parsed.expressions()[1].kind,
        ExpressionKind::Call { .. }
    ));
    assert!(parse_generic_call_expressions(
        &syntax("call::<T>(other::<Int>(1))"),
        GenericCallExpressionLimits::DEFAULT
    )
    .is_err());
}
#[test]
fn errors_keep_argument_index_and_original_call_coordinates() {
    let source = "// λ😀\ncall::<T>(x, let value)";
    let GenericCallExpressionError::Parse {
        argument,
        diagnostics,
    } = parse_generic_call_expressions(&syntax(source), GenericCallExpressionLimits::DEFAULT)
        .unwrap_err()
    else {
        panic!()
    };
    assert_eq!(argument, 1);
    let span = diagnostics.items[0].span;
    assert_eq!(&source[span.start..span.end], "let");
    assert_eq!(span.line, 1);
    let source = "call::<T>(x y)";
    let GenericCallExpressionError::Boundary { span, .. } =
        parse_generic_call_expressions(&syntax(source), GenericCallExpressionLimits::DEFAULT)
            .unwrap_err()
    else {
        panic!()
    };
    assert_eq!(&source[span.start..span.end], "y");
}
#[test]
fn cumulative_ast_root_source_token_edge_and_work_budgets_are_exact() {
    let call = syntax("call::<T>(x+1,[x,y],f(x))");
    let parsed =
        parse_generic_call_expressions(&call, GenericCallExpressionLimits::DEFAULT).unwrap();
    let exact = GenericCallExpressionLimits {
        max_source_bytes: call.source().len(),
        max_tokens: parsed.tokens(),
        max_values: 3,
        max_nodes: parsed.nodes(),
        max_edges: parsed.edges(),
        max_work: parsed.work(),
        ..GenericCallExpressionLimits::DEFAULT
    };
    assert!(parse_generic_call_expressions(&call, exact).is_ok());
    for limits in [
        GenericCallExpressionLimits {
            max_source_bytes: exact.max_source_bytes - 1,
            ..exact
        },
        GenericCallExpressionLimits {
            max_tokens: exact.max_tokens - 1,
            ..exact
        },
        GenericCallExpressionLimits {
            max_values: 2,
            ..exact
        },
        GenericCallExpressionLimits {
            max_nodes: exact.max_nodes - 1,
            ..exact
        },
        GenericCallExpressionLimits {
            max_edges: exact.max_edges - 1,
            ..exact
        },
        GenericCallExpressionLimits {
            max_work: exact.max_work - 1,
            ..exact
        },
    ] {
        assert!(
            parse_generic_call_expressions(&call, limits).is_err(),
            "{limits:?}"
        );
    }
}
#[test]
fn empty_lists_still_consume_token_work_and_shared_asts_remain_owned() {
    let call = syntax("call::<T>()");
    let parsed =
        parse_generic_call_expressions(&call, GenericCallExpressionLimits::DEFAULT).unwrap();
    assert_eq!(parsed.nodes(), 0);
    assert!(parsed.expressions().is_empty());
    assert!(parse_generic_call_expressions(
        &call,
        GenericCallExpressionLimits {
            max_work: 0,
            ..GenericCallExpressionLimits::DEFAULT
        }
    )
    .is_err());
    let parsed = parse("call::<T>([x+1])");
    let clone = parsed.clone();
    drop(parsed);
    assert!(matches!(
        clone.expressions()[0].kind,
        ExpressionKind::ArrayLiteral(_)
    ));
    drop(clone);
}
