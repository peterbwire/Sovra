//! Checked value type matching, separate from source calls and execution.
use sovra::compiler::v2::*;
fn fixture(
    callee: &str,
    explicit: &str,
) -> (InstantiatedGenericSignature, CheckedGenericCopyBodies) {
    let registry = register_generic_types(&[], GenericRegistryLimits::DEFAULT).unwrap();
    let env =
        prepare_generic_aliases(&registry, &[], GenericAliasEnvironmentLimits::DEFAULT).unwrap();
    let types = check_generic_declarations(&env, &[], GenericDeclarationLimits::DEFAULT).unwrap();
    let caller = parse_generic_function(
        "// λ😀\nfn caller<U>(x:U,ints:[Int],floats:[Float]){x;1;1.0;ints;floats;}",
        GenericFunctionLimits::DEFAULT,
    )
    .unwrap();
    let callee = parse_generic_function(callee, GenericFunctionLimits::DEFAULT).unwrap();
    let signatures =
        check_generic_signatures(&types, &[&caller, &callee], GenericSignatureLimits::DEFAULT)
            .unwrap();
    let arguments = check_generic_type_arguments(
        &types,
        &parse_generic_type_arguments(explicit, GenericTypeArgumentLimits::DEFAULT).unwrap(),
        caller.parameters(),
        GenericArgumentCheckLimits::DEFAULT,
    )
    .unwrap();
    let binding = bind_generic_type_arguments(
        &signatures,
        signatures.find("caller").unwrap(),
        signatures.find("target").unwrap(),
        &arguments,
        GenericCallBindingLimits::DEFAULT,
    )
    .unwrap();
    let instantiated =
        instantiate_generic_signature(&binding, GenericSignatureInstantiationLimits::DEFAULT)
            .unwrap();
    let bodies = check_generic_copy_bodies(&signatures, GenericCopyBodyLimits::DEFAULT).unwrap();
    (instantiated, bodies)
}
#[test]
fn opaque_values_match_exact_caller_identity_and_retain_expression_order() {
    let (signature, bodies) = fixture(
        "fn target<A,B>(left:A,right:B)->A{return left;}",
        "::<U,Int>",
    );
    let body = &bodies.bodies()[0];
    let values = [body.expressions()[0].id, body.expressions()[1].id];
    let checked =
        check_generic_call_values(&signature, body, &values, GenericCallValueLimits::DEFAULT)
            .unwrap();
    assert_eq!(checked.arguments()[0].expression, values[0]);
    assert_eq!(checked.arguments()[1].expression, values[1]);
    assert!(checked
        .arguments()
        .iter()
        .all(|a| a.conversion == GenericCopyConversion::Identity));
    assert!(matches!(
        check_generic_call_values(
            &signature,
            body,
            &[values[1], values[0]],
            GenericCallValueLimits::DEFAULT
        ),
        Err(GenericCallValueError::Type { argument: 0, .. })
    ));
}
#[test]
fn concrete_scalar_widening_is_explicit_without_aggregate_coercion() {
    let (signature, bodies) = fixture("fn target<T>(value:Float)->Float{return value;}", "::<U>");
    let body = &bodies.bodies()[0];
    let checked = check_generic_call_values(
        &signature,
        body,
        &[body.expressions()[1].id],
        GenericCallValueLimits::DEFAULT,
    )
    .unwrap();
    assert_eq!(
        checked.arguments()[0].conversion,
        GenericCopyConversion::IntToFloat
    );
    let checked = check_generic_call_values(
        &signature,
        body,
        &[body.expressions()[2].id],
        GenericCallValueLimits::DEFAULT,
    )
    .unwrap();
    assert_eq!(
        checked.arguments()[0].conversion,
        GenericCopyConversion::Identity
    );
    let (signature, bodies) = fixture("fn target<T>(value:T)->T{return value;}", "::<[Float]>");
    let body = &bodies.bodies()[0];
    assert!(matches!(
        check_generic_call_values(
            &signature,
            body,
            &[body.expressions()[3].id],
            GenericCallValueLimits::DEFAULT
        ),
        Err(GenericCallValueError::Type { .. })
    ));
}
#[test]
fn foreign_body_and_expression_owners_and_wrong_counts_refuse() {
    let (signature, bodies) = fixture("fn target<T>(value:T)->T{return value;}", "::<U>");
    let body = &bodies.bodies()[0];
    let (_, foreign) = fixture("fn target<T>(value:T)->T{return value;}", "::<U>");
    assert_eq!(
        check_generic_call_values(
            &signature,
            &foreign.bodies()[0],
            &[foreign.bodies()[0].expressions()[0].id],
            GenericCallValueLimits::DEFAULT
        )
        .unwrap_err(),
        GenericCallValueError::Caller
    );
    assert_eq!(
        check_generic_call_values(
            &signature,
            body,
            &[bodies.bodies()[1].expressions()[0].id],
            GenericCallValueLimits::DEFAULT
        )
        .unwrap_err(),
        GenericCallValueError::Expression { argument: 0 }
    );
    assert_eq!(
        check_generic_call_values(&signature, body, &[], GenericCallValueLimits::DEFAULT)
            .unwrap_err(),
        GenericCallValueError::Arity {
            expected: 1,
            actual: 0
        }
    );
}
#[test]
fn mismatches_keep_original_caller_expression_ranges() {
    let (signature, bodies) = fixture("fn target<T>(value:T)->T{return value;}", "::<Int>");
    let body = &bodies.bodies()[0];
    let GenericCallValueError::Type { argument, span } = check_generic_call_values(
        &signature,
        body,
        &[body.expressions()[0].id],
        GenericCallValueLimits::DEFAULT,
    )
    .unwrap_err() else {
        panic!()
    };
    assert_eq!(argument, 0);
    let source = body.signature().syntax().source();
    assert_eq!(&source[span.start..span.end], "x");
    assert_eq!(span.line, 1);
}
#[test]
fn exact_cumulative_and_independent_comparison_caps_never_become_type_decisions() {
    let (signature, bodies) = fixture(
        "fn target<A,B>(left:A,right:B)->A{return left;}",
        "::<[Int],[Float]>",
    );
    let body = &bodies.bodies()[0];
    let values = [body.expressions()[3].id, body.expressions()[4].id];
    let checked =
        check_generic_call_values(&signature, body, &values, GenericCallValueLimits::DEFAULT)
            .unwrap();
    let exact = GenericCallValueLimits {
        max_arguments: 2,
        max_pairs: checked.pairs(),
        max_edges: checked.edges(),
        max_work: checked.work(),
        ..GenericCallValueLimits::DEFAULT
    };
    assert!(check_generic_call_values(&signature, body, &values, exact).is_ok());
    for limits in [
        GenericCallValueLimits {
            max_arguments: 1,
            ..exact
        },
        GenericCallValueLimits {
            max_pairs: exact.max_pairs - 1,
            ..exact
        },
        GenericCallValueLimits {
            max_edges: exact.max_edges - 1,
            ..exact
        },
        GenericCallValueLimits {
            max_work: exact.max_work - 1,
            ..exact
        },
        GenericCallValueLimits {
            comparison: GenericTypeEquivalenceLimits {
                max_pairs: 1,
                ..exact.comparison
            },
            ..exact
        },
    ] {
        assert!(check_generic_call_values(&signature, body, &values, limits).is_err());
    }
    assert!(matches!(
        check_generic_call_values(
            &signature,
            body,
            &values,
            GenericCallValueLimits {
                max_pairs: 0,
                ..exact
            }
        ),
        Err(GenericCallValueError::Comparison { .. })
    ));
}
#[test]
fn empty_value_lists_still_check_work_and_preserve_implicit_unit() {
    let (signature, bodies) = fixture("fn target<T>(){}", "::<U>");
    let body = &bodies.bodies()[0];
    let checked =
        check_generic_call_values(&signature, body, &[], GenericCallValueLimits::DEFAULT).unwrap();
    assert!(matches!(
        checked.return_type(),
        GenericSignatureReturn::ImplicitUnit
    ));
    assert!(check_generic_call_values(
        &signature,
        body,
        &[],
        GenericCallValueLimits {
            max_work: 0,
            ..GenericCallValueLimits::DEFAULT
        }
    )
    .is_err());
    drop(bodies);
    drop(signature);
    assert!(checked.arguments().is_empty());
    assert_eq!(checked.body().signature().syntax().name(), "caller");
}

#[test]
fn same_signature_rechecked_in_another_arena_cannot_supply_foreign_value_ids() {
    let (signature, bodies) = fixture("fn target<T>(value:T)->T{return value;}", "::<U>");
    let rechecked = check_generic_copy_bodies(
        signature.binding().signatures(),
        GenericCopyBodyLimits::DEFAULT,
    )
    .unwrap();
    let original = &bodies.bodies()[0];
    let fresh = &rechecked.bodies()[0];
    assert_eq!(original.signature().id(), fresh.signature().id());
    assert_eq!(
        check_generic_call_values(
            &signature,
            fresh,
            &[original.expressions()[0].id],
            GenericCallValueLimits::DEFAULT
        )
        .unwrap_err(),
        GenericCallValueError::Expression { argument: 0 }
    );
    assert!(check_generic_call_values(
        &signature,
        fresh,
        &[fresh.expressions()[0].id],
        GenericCallValueLimits::DEFAULT
    )
    .is_ok());
}
