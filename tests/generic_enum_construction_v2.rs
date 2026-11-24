//! Explicit symbolic nullary/single-payload enums, exact owners and source tags.
use sovra::compiler::v2::*;
fn table(source: &str) -> CheckedGenericSignatures {
    let record =
        parse_generic_record("struct Box<T>{value:T}", GenericRecordLimits::DEFAULT).unwrap();
    let enums: Vec<_> = [
        "enum Choice<T>{End,One(T),Many(Rows<T>),Boxed(Box<T>),Unit(Unit)}",
        "enum Other<T>{One(T)}",
        "enum Keyed<T:std::Key>{End,One(T)}",
        "enum Same<T>{Same(T)}",
    ]
    .iter()
    .map(|s| parse_generic_enum(s, GenericEnumLimits::DEFAULT).unwrap())
    .collect();
    let aliases: Vec<_> = ["type Rows<T>=[T];", "type Alias<T>=Choice<T>;"]
        .iter()
        .map(|s| parse_generic_alias(s, GenericAliasLimits::DEFAULT).unwrap())
        .collect();
    let mut declarations = vec![GenericTypeDeclaration::Record(&record)];
    declarations.extend(enums.iter().map(GenericTypeDeclaration::Enum));
    declarations.extend(aliases.iter().map(GenericTypeDeclaration::Alias));
    let registry = register_generic_types(&declarations, GenericRegistryLimits::DEFAULT).unwrap();
    let aliases_ref: Vec<_> = aliases.iter().collect();
    let env = prepare_generic_aliases(
        &registry,
        &aliases_ref,
        GenericAliasEnvironmentLimits::DEFAULT,
    )
    .unwrap();
    let types =
        check_generic_declarations(&env, &declarations, GenericDeclarationLimits::DEFAULT).unwrap();
    let caller = parse_generic_function(source, GenericFunctionLimits::DEFAULT).unwrap();
    let identity = parse_generic_function(
        "fn identity<T>(x:T)->T{return x;}",
        GenericFunctionLimits::DEFAULT,
    )
    .unwrap();
    let unit = parse_generic_function("fn unit<T>(){}", GenericFunctionLimits::DEFAULT).unwrap();
    check_generic_signatures(
        &types,
        &[&caller, &identity, &unit],
        GenericSignatureLimits::DEFAULT,
    )
    .unwrap()
}
fn check(source: &str) -> CheckedGenericCopyBodies {
    check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).unwrap()
}
#[test]
fn nominal_variant_payload_and_original_unicode_sources_remain_exact() {
    let source = "// λ😀\nfn make<U>(x:U)->Choice<U>{return Choice::<U>::One(x);}";
    let checked = check(source);
    let body = &checked.bodies()[0];
    let e = body.expressions().last().unwrap();
    let GenericCopyExpressionKind::Enum {
        declaration,
        type_range,
        variant_index,
        variant_range,
        payload,
    } = &e.kind
    else {
        panic!()
    };
    assert_eq!(*variant_index, 1);
    assert_eq!(&source[type_range.start..type_range.end], "Choice::<U>");
    assert_eq!(&source[variant_range.start..variant_range.end], "One");
    assert_eq!(&source[e.span.start..e.span.end], "Choice::<U>::One(x)");
    let payload = payload.as_ref().unwrap();
    assert!(
        matches!(body.expression(payload.value).unwrap().kind,GenericCopyExpressionKind::Read(id) if id==body.bindings()[0].id)
    );
    let ty = e.ty.expanded().unwrap();
    assert!(
        matches!(&ty.nodes()[ty.root()].kind,ResolvedGenericTypeKind::Declared{declaration:id,..} if id==declaration)
    );
    assert_eq!(checked.enum_constructions(), 1);
    assert_eq!(checked.call_usage().calls, 0);
}
#[test]
fn nullary_variants_and_unit_payloads_are_distinct_even_without_value_bits() {
    let end = check("fn make<U>()->Choice<U>{return Choice::<U>::End();}");
    assert!(matches!(
        &end.bodies()[0].expressions().last().unwrap().kind,
        GenericCopyExpressionKind::Enum { payload: None, .. }
    ));
    let unit = check("fn make<U>()->Choice<U>{return Choice::<U>::Unit(unit::<U>());}");
    let body = &unit.bodies()[0];
    let GenericCopyExpressionKind::Enum {
        payload: Some(payload),
        ..
    } = &body.expressions().last().unwrap().kind
    else {
        panic!()
    };
    assert_eq!(
        body.expression(payload.value).unwrap().ty.primitive(),
        Some(GenericPrimitive::Unit)
    );
    let array =
        check("fn make<U>(x:U)->[Choice<U>]{return [Choice::<U>::One(x),Choice::<U>::End()];}");
    let body = &array.bodies()[0];
    let GenericCopyExpressionKind::Array(items) = &body.expressions().last().unwrap().kind else {
        panic!()
    };
    assert!(matches!(
        body.expression(items[0].value).unwrap().kind,
        GenericCopyExpressionKind::Enum {
            payload: Some(_),
            ..
        }
    ));
    assert!(matches!(
        body.expression(items[1].value).unwrap().kind,
        GenericCopyExpressionKind::Enum { payload: None, .. }
    ));
    for source in [
        "fn make<U>(x:U){Choice::<U>::End(x);}",
        "fn make<U>(){Choice::<U>::Unit();}",
        "fn make<U>(){Choice::<U>::One();}",
        "fn make<U>(){Choice::<U>::End;}",
    ] {
        assert!(
            check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
}
#[test]
fn payload_contexts_compose_with_arrays_records_nested_enums_and_calls() {
    let many = check("fn make<U>(x:U)->Choice<U>{return Choice::<U>::Many([identity::<U>(x)]);}");
    assert_eq!(many.enum_constructions(), 1);
    assert_eq!(many.call_usage().calls, 1);
    let boxed = check("fn make<U>(x:U)->Choice<U>{return Choice::<U>::Boxed(Box::<U>{value:x});}");
    assert_eq!(boxed.record_constructions(), 1);
    assert_eq!(boxed.enum_constructions(), 1);
    let nested=check("fn make<U>(x:U)->Choice<Choice<U>>{return (Choice::<Choice<U>>::One(Choice::<U>::One(x)));}");
    assert_eq!(nested.enum_constructions(), 2);
}
#[test]
fn scalar_payload_widening_never_becomes_aggregate_or_nominal_conversion() {
    let checked = check("fn make<U>()->Choice<Float>{return Choice::<Float>::One(1);}");
    assert!(
        matches!(&checked.bodies()[0].expressions().last().unwrap().kind,GenericCopyExpressionKind::Enum{payload:Some(payload),..} if payload.conversion==GenericCopyConversion::IntToFloat)
    );
    for source in [
        "fn make<U>(x:U)->Choice<U>{return Other::<U>::One(x);}",
        "fn make<U>()->Choice<Float>{return Choice::<Int>::End();}",
        "fn make<U>(xs:[Int])->Choice<Float>{return Choice::<Float>::Many(xs);}",
        "fn make<U>(x:U)->Choice<U>{return Choice::<U>::One(1);}",
    ] {
        assert!(
            check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
}
#[test]
fn variant_heads_arguments_placement_aliases_and_unreachable_errors_refuse() {
    for source in [
        "fn make<U>(x:U){Choice::<U>::Missing(x);}",
        "fn make<U>(x:U){Choice::One::<U>(x);}",
        "fn make<U>(x:U){Same::Same::<U>(x);}",
        "fn make<U>(x:U){Choice::One(x);}",
        "fn make<U>(x:U){Alias::<U>::One(x);}",
        "fn make<U>(x:U){module::Choice::<U>::One(x);}",
        "fn make<U>(x:U){Choice::<U,U>::One(x);}",
        "fn make<U>(x:U){return;Choice::<U>::One(missing);}",
    ] {
        assert!(
            check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
}
#[test]
fn key_constraints_apply_to_nullary_variants_and_value_names_do_not_replace_types() {
    for source in [
        "fn make<U>(){Keyed::<U>::End();}",
        "fn make<U>(){Keyed::<Float>::End();}",
        "fn make<U>(){Keyed::<[Int]>::End();}",
    ] {
        assert!(check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).is_err());
    }
    assert_eq!(
        check("fn make<U:std::Key>(x:U)->Keyed<U>{return Keyed::<U>::One(x);}")
            .enum_constructions(),
        1
    );
    assert_eq!(
        check("fn make<U>(x:U)->Choice<U>{let Choice=x;return Choice::<U>::One(Choice);}")
            .enum_constructions(),
        1
    );
}
#[test]
fn alias_payload_context_origins_keep_caller_parameters_and_alias_sources() {
    let checked = check("fn make<U>(x:U)->Choice<U>{return Choice::<U>::Many([x]);}");
    let body = &checked.bodies()[0];
    let array = body
        .expressions()
        .iter()
        .find(|e| matches!(e.kind, GenericCopyExpressionKind::Array(_)))
        .unwrap();
    let ty = array.ty.expanded().unwrap();
    let node = &ty.nodes()[ty.root()];
    assert!(node.origin.alias.is_some());
    assert_eq!(
        &ty.node_source(ty.root()).unwrap()[node.origin.span.start..node.origin.span.end],
        "[T]"
    );
    let ResolvedGenericTypeKind::Array { element } = node.kind else {
        panic!()
    };
    assert!(
        matches!(ty.nodes()[element].kind,ResolvedGenericTypeKind::Parameter(id) if id==body.signature().syntax().parameters().find("U").unwrap())
    );
}
#[test]
fn constructor_shared_input_field_generated_and_work_budgets_are_exact() {
    let nullary = table("fn make<U>()->Choice<U>{return Choice::<U>::End();}");
    assert!(check_generic_copy_bodies(
        &nullary,
        GenericCopyBodyLimits {
            max_record_constructions: 0,
            max_field_nodes: 0,
            max_field_edges: 0,
            calls: GenericBodyCallLimits {
                max_calls: 0,
                max_values: 0,
                ..GenericBodyCallLimits::DEFAULT
            },
            ..GenericCopyBodyLimits::DEFAULT
        }
    )
    .is_ok());

    let signatures = table("fn make<U>(x:U)->Choice<U>{return Choice::<U>::Many([x]);}");
    let checked = check_generic_copy_bodies(&signatures, GenericCopyBodyLimits::DEFAULT).unwrap();
    let usage = checked.call_usage();
    let calls = GenericBodyCallLimits {
        max_calls: 0,
        max_values: usage.values,
        max_nodes: usage.nodes,
        max_edges: usage.edges,
        arguments: GenericArgumentCheckLimits {
            max_arguments: usage.type_arguments,
            max_source_bytes: usage.source_bytes,
            max_tokens: usage.tokens,
            max_input_nodes: usage.input_nodes,
            max_input_edges: usage.input_edges,
            max_name_bytes: usage.name_bytes,
            ..GenericArgumentCheckLimits::DEFAULT
        },
        ..GenericBodyCallLimits::DEFAULT
    };
    let exact = GenericCopyBodyLimits {
        max_enum_constructions: 1,
        calls,
        max_field_nodes: checked.field_nodes(),
        max_field_edges: checked.field_edges(),
        max_work: checked.work(),
        ..GenericCopyBodyLimits::DEFAULT
    };
    assert!(check_generic_copy_bodies(&signatures, exact).is_ok());
    for limits in [
        GenericCopyBodyLimits {
            max_enum_constructions: 0,
            ..exact
        },
        GenericCopyBodyLimits {
            max_field_nodes: exact.max_field_nodes - 1,
            ..exact
        },
        GenericCopyBodyLimits {
            max_field_edges: exact.max_field_edges - 1,
            ..exact
        },
        GenericCopyBodyLimits {
            max_work: exact.max_work - 1,
            ..exact
        },
        GenericCopyBodyLimits {
            calls: GenericBodyCallLimits {
                max_values: usage.values - 1,
                ..calls
            },
            ..exact
        },
        GenericCopyBodyLimits {
            calls: GenericBodyCallLimits {
                max_nodes: usage.nodes - 1,
                ..calls
            },
            ..exact
        },
        GenericCopyBodyLimits {
            calls: GenericBodyCallLimits {
                max_edges: usage.edges - 1,
                ..calls
            },
            ..exact
        },
        GenericCopyBodyLimits {
            field_expansion: GenericFieldSubstitutionLimits {
                max_work: 0,
                ..exact.field_expansion
            },
            ..exact
        },
    ] {
        assert!(
            check_generic_copy_bodies(&signatures, limits).is_err(),
            "{limits:?}"
        );
    }
}
#[test]
fn enum_and_payload_ledgers_are_cumulative_across_functions_and_cli_stays_strict() {
    let base = table("fn make<U>(x:U)->Choice<U>{return Choice::<U>::One(x);}");
    let a = base.signatures()[0].syntax();
    let b = parse_generic_function(
        "fn other<V>(x:V)->Choice<V>{return Choice::<V>::One(x);}",
        GenericFunctionLimits::DEFAULT,
    )
    .unwrap();
    let signatures =
        check_generic_signatures(base.types(), &[a, &b], GenericSignatureLimits::DEFAULT).unwrap();
    assert!(check_generic_copy_bodies(
        &signatures,
        GenericCopyBodyLimits {
            max_enum_constructions: 1,
            ..GenericCopyBodyLimits::DEFAULT
        }
    )
    .is_err());
    assert!(check_generic_copy_bodies(
        &signatures,
        GenericCopyBodyLimits {
            calls: GenericBodyCallLimits {
                max_values: 1,
                ..GenericBodyCallLimits::DEFAULT
            },
            ..GenericCopyBodyLimits::DEFAULT
        }
    )
    .is_err());
    assert!(check_program_source(
        "strict",
        "enum Choice<T>{End}fn main(){Choice::<Int>::End();}",
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
}
