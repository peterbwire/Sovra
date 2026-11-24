//! Exact original literal values, source ownership and cumulative decoding admission.
use sovra::compiler::v2::*;
use std::sync::Arc;
fn concrete(root: &str, functions: &[&str]) -> CheckedConcreteGenericBodies {
    let registry = register_generic_types(&[], GenericRegistryLimits::DEFAULT).unwrap();
    let env =
        prepare_generic_aliases(&registry, &[], GenericAliasEnvironmentLimits::DEFAULT).unwrap();
    let types = check_generic_declarations(&env, &[], GenericDeclarationLimits::DEFAULT).unwrap();
    let mut syntax = vec![parse_specialization_root(root, GenericFunctionLimits::DEFAULT).unwrap()];
    syntax.extend(
        functions
            .iter()
            .map(|s| parse_generic_function(s, GenericFunctionLimits::DEFAULT).unwrap()),
    );
    let signatures = check_generic_signatures(
        &types,
        &syntax.iter().collect::<Vec<_>>(),
        GenericSignatureLimits::DEFAULT,
    )
    .unwrap();
    let bodies = check_generic_copy_bodies(&signatures, GenericCopyBodyLimits::DEFAULT).unwrap();
    let graph = discover_generic_specializations(
        &bodies,
        signatures.find("root").unwrap(),
        GenericSpecializationLimits::DEFAULT,
    )
    .unwrap();
    recheck_generic_concrete_bodies(&graph, GenericConcreteBodyLimits::DEFAULT).unwrap()
}
fn decode(input: &CheckedConcreteGenericBodies) -> CheckedGenericLiterals {
    decode_generic_literals(input, GenericLiteralLimits::DEFAULT).unwrap()
}
#[test]
fn exact_int_float_bits_bool_and_string_payloads_survive_without_textual_reparse() {
    let input=concrete("fn root(){let zero=0;let max=9223372036854775807;let float=0.1;let precise=1.0000000000000002;let yes=true;let no=false;let text=\"hi\";}",&[]);
    let decoded = decode(&input);
    let facts = decoded.bodies()[0].literals();
    assert_eq!(
        facts.iter().map(|f| f.value.clone()).collect::<Vec<_>>(),
        [
            GenericLiteralValue::Int(0),
            GenericLiteralValue::Int(i64::MAX),
            GenericLiteralValue::FloatBits(0.1f64.to_bits()),
            GenericLiteralValue::FloatBits(1.0000000000000002f64.to_bits()),
            GenericLiteralValue::Bool(true),
            GenericLiteralValue::Bool(false),
            GenericLiteralValue::String(Arc::from("hi")),
        ]
    );
    for fact in facts {
        assert_eq!(
            decoded.bodies()[0].literal(fact.expression).unwrap().value,
            fact.value
        );
    }
    let cloned = decoded.clone();
    drop(decoded);
    drop(input);
    assert_eq!(
        cloned.bodies()[0].literals()[1].value,
        GenericLiteralValue::Int(i64::MAX)
    );
}
#[test]
fn unicode_escapes_grouping_comments_and_crlf_keep_original_spans_and_values() {
    let source = concat!(
        "// λ😀\r\n",
        r#"fn root(){let text=(("λ😀\n\r\t\"\\é"));let number=( // grouping
 (17));}"#
    );
    let decoded = decode(&concrete(source, &[]));
    let facts = decoded.bodies()[0].literals();
    assert_eq!(
        facts[0].value,
        GenericLiteralValue::String(Arc::from("λ😀\n\r\t\"\\é"))
    );
    assert_eq!(facts[1].value, GenericLiteralValue::Int(17));
    assert_eq!(
        &source[facts[0].span.start..facts[0].span.end],
        r#"(("λ😀\n\r\t\"\\é"))"#
    );
    assert!(source[facts[1].span.start..facts[1].span.end].contains("// grouping"));
    assert_eq!(facts[0].span.line, 1);
}
#[test]
fn repeated_instances_share_payloads_but_charge_each_literal_reference() {
    let input = concrete(
        "fn root(){same::<Int>(1);same::<Bool>(true);}",
        &["fn same<T>(x:T)->T{let text=\"shared\";let decimal=1.25;return x;}"],
    );
    let decoded = decode(&input);
    assert_eq!(decoded.usage().instances, 3);
    assert_eq!(decoded.usage().literals, 4);
    assert_eq!(decoded.usage().references, 6);
    assert_eq!(decoded.usage().string_bytes, 6);
    let GenericLiteralValue::String(a) = &decoded.bodies()[1].literals()[0].value else {
        panic!()
    };
    let GenericLiteralValue::String(b) = &decoded.bodies()[2].literals()[0].value else {
        panic!()
    };
    assert!(Arc::ptr_eq(a, b));
}
#[test]
fn contextual_widening_keeps_original_int_payload_and_explicit_conversion_edge() {
    let decoded = decode(&concrete("fn root(){let value:Float=7;}", &[]));
    let body = &decoded.bodies()[0];
    assert_eq!(body.literals()[0].value, GenericLiteralValue::Int(7));
    assert_eq!(
        body.body()
            .expression_type(body.literals()[0].expression)
            .unwrap()
            .primitive(),
        Some(GenericPrimitive::Int)
    );
    assert!(matches!(
        body.body().body().statements()[0],
        GenericCopyStatement::Let {
            conversion: GenericCopyConversion::IntToFloat,
            ..
        }
    ));
}
#[test]
fn exact_and_one_under_cumulative_and_per_fragment_allowances_are_enforced() {
    let input = concrete(
        "fn root(){copy::<Int>((((1))));copy::<String>(\"λ\");}",
        &["fn copy<T>(x:T)->T{let text=\"shared\";return x;}"],
    );
    let usage = decode(&input).usage();
    let exact = GenericLiteralLimits {
        max_instances: usage.instances,
        max_literals: usage.literals,
        max_references: usage.references,
        max_source_bytes: usage.source_bytes,
        max_tokens: usage.tokens,
        max_string_bytes: usage.string_bytes,
        max_work: usage.work,
        ..GenericLiteralLimits::DEFAULT
    };
    assert_eq!(
        decode_generic_literals(&input, exact).unwrap().usage(),
        usage
    );
    for limits in [
        GenericLiteralLimits {
            max_instances: exact.max_instances - 1,
            ..exact
        },
        GenericLiteralLimits {
            max_literals: exact.max_literals - 1,
            ..exact
        },
        GenericLiteralLimits {
            max_references: exact.max_references - 1,
            ..exact
        },
        GenericLiteralLimits {
            max_source_bytes: exact.max_source_bytes - 1,
            ..exact
        },
        GenericLiteralLimits {
            max_tokens: exact.max_tokens - 1,
            ..exact
        },
        GenericLiteralLimits {
            max_string_bytes: exact.max_string_bytes - 1,
            ..exact
        },
        GenericLiteralLimits {
            max_work: exact.max_work - 1,
            ..exact
        },
        GenericLiteralLimits {
            max_fragment_bytes: 0,
            ..exact
        },
        GenericLiteralLimits {
            max_fragment_tokens: 0,
            ..exact
        },
        GenericLiteralLimits {
            max_fragment_tokens: 7,
            ..exact
        },
        GenericLiteralLimits {
            max_depth: 2,
            ..exact
        },
        GenericLiteralLimits {
            max_depth: 0,
            ..exact
        },
        GenericLiteralLimits {
            max_depth: 129,
            ..exact
        },
    ] {
        assert!(
            decode_generic_literals(&input, limits).is_err(),
            "{limits:?}"
        );
    }
}
#[test]
fn foreign_ids_and_nonliteral_expressions_refuse_lookup_and_empty_unit_needs_no_payload() {
    let decoded = decode(&concrete("fn root(){let value=1;let copy=value;}", &[]));
    let foreign = decode(&concrete("fn root(){let value=1;}", &[]));
    assert!(decoded.bodies()[0]
        .literal(foreign.bodies()[0].literals()[0].expression)
        .is_none());
    let read = decoded.bodies()[0]
        .body()
        .body()
        .expressions()
        .iter()
        .find(|e| matches!(e.kind, GenericCopyExpressionKind::Read(_)))
        .unwrap();
    assert!(decoded.bodies()[0].literal(read.id).is_none());
    let empty = concrete("fn root(){}", &[]);
    let decoded = decode_generic_literals(
        &empty,
        GenericLiteralLimits {
            max_instances: 1,
            max_literals: 0,
            max_references: 0,
            max_source_bytes: 0,
            max_tokens: 0,
            max_string_bytes: 0,
            ..GenericLiteralLimits::DEFAULT
        },
    )
    .unwrap();
    assert!(decoded.bodies()[0].literals().is_empty());
    assert_eq!(decoded.usage().tokens, 0);
}
#[test]
fn unreachable_literals_and_literal_text_containing_delimiters_are_preserved() {
    let decoded = decode(&concrete(
        "fn root(){return;let text=\"()=>{} // not syntax\";let flag=(false);}",
        &[],
    ));
    assert_eq!(decoded.bodies()[0].literals().len(), 2);
    assert_eq!(
        decoded.bodies()[0].literals()[0].value,
        GenericLiteralValue::String(Arc::from("()=>{} // not syntax"))
    );
    assert!(check_program_source(
        "strict",
        "fn id<T>(x:T)->T{return x;}fn main(){id::<Int>(1);}",
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
}
#[test]
fn decoded_utf8_byte_quota_counts_bytes_and_accepts_empty_and_embedded_nul_strings() {
    let decoded = decode(&concrete(
        "fn root(){let empty=\"\";let unicode=\"λ😀\";let nul=\"a\0b\";}",
        &[],
    ));
    assert_eq!(decoded.usage().string_bytes, 9);
    assert_eq!(
        decoded.bodies()[0].literals()[2].value,
        GenericLiteralValue::String(Arc::from("a\0b"))
    );
}
