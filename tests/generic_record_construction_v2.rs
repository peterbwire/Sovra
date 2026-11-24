//! Explicit generic record construction with exact fields, contexts and provenance.
use sovra::compiler::v2::*;
fn table(source: &str) -> CheckedGenericSignatures {
    let records: Vec<_> = [
        "struct Box<T>{value:T,weight:Float}",
        "struct Pair<A,B>{first:A,second:B}",
        "struct Meta<T>{items:[T],nested:Box<T>}",
        "struct Keyed<T:std::Key>{value:T}",
        "struct Empty<T>{}",
    ]
    .iter()
    .map(|s| parse_generic_record(s, GenericRecordLimits::DEFAULT).unwrap())
    .collect();
    let alias = parse_generic_alias("type Alias<T>=Box<T>;", GenericAliasLimits::DEFAULT).unwrap();
    let mut declarations: Vec<_> = records.iter().map(GenericTypeDeclaration::Record).collect();
    declarations.push(GenericTypeDeclaration::Alias(&alias));
    let registry = register_generic_types(&declarations, GenericRegistryLimits::DEFAULT).unwrap();
    let env = prepare_generic_aliases(&registry, &[&alias], GenericAliasEnvironmentLimits::DEFAULT)
        .unwrap();
    let types =
        check_generic_declarations(&env, &declarations, GenericDeclarationLimits::DEFAULT).unwrap();
    let syntax = parse_generic_function(source, GenericFunctionLimits::DEFAULT).unwrap();
    check_generic_signatures(&types, &[&syntax], GenericSignatureLimits::DEFAULT).unwrap()
}
fn check(source: &str) -> CheckedGenericCopyBodies {
    check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).unwrap()
}
#[test]
fn explicit_record_head_and_field_values_retain_original_source_and_nominal_identity() {
    let source = "// λ😀\nfn make<U>(x:U)->Box<U>{return Box::<U>{value:x,weight:1};}";
    let checked = check(source);
    let body = &checked.bodies()[0];
    let e = body.expressions().last().unwrap();
    let GenericCopyExpressionKind::Record {
        declaration,
        type_range,
        fields,
    } = &e.kind
    else {
        panic!()
    };
    assert_eq!(
        &source[e.span.start..e.span.end],
        "Box::<U>{value:x,weight:1}"
    );
    assert_eq!(&source[type_range.start..type_range.end], "Box::<U>");
    assert_eq!(fields[0].field_index, 0);
    assert_eq!(fields[1].conversion, GenericCopyConversion::IntToFloat);
    let ty = e.ty.expanded().unwrap();
    assert!(
        matches!(ty.nodes()[ty.root()].kind,ResolvedGenericTypeKind::Declared{declaration:id,..} if id==*declaration)
    );
    let origin = ty.nodes()[ty.root()].origin;
    assert_eq!(
        &ty.node_source(ty.root()).unwrap()[origin.span.start..origin.span.end],
        "Box::<U>"
    );
    assert!(std::ptr::eq(
        ty.syntax().source().as_ptr(),
        body.signature().syntax().source().as_ptr()
    ));
    assert_eq!(checked.record_constructions(), 1);
    assert_eq!(checked.call_usage().calls, 0);
}
#[test]
fn source_field_order_is_separate_from_declaration_indices() {
    let checked = check("fn pair<A,B>(x:A,y:B)->Pair<A,B>{return Pair::<A,B>{second:y,first:x};}");
    let body = &checked.bodies()[0];
    let GenericCopyExpressionKind::Record { fields, .. } = &body.expressions().last().unwrap().kind
    else {
        panic!()
    };
    assert_eq!(
        fields.iter().map(|f| f.field_index).collect::<Vec<_>>(),
        [1, 0]
    );
    assert!(
        matches!(body.expression(fields[0].value).unwrap().kind,GenericCopyExpressionKind::Read(id) if id==body.bindings()[1].id)
    );
}
#[test]
fn nested_records_arrays_and_empty_records_receive_exact_field_contexts() {
    assert_eq!(check("fn make<U>(x:U)->Meta<U>{return Meta::<U>{items:[x],nested:Box::<U>{value:x,weight:1}};}").record_constructions(),2);
    assert_eq!(
        check("fn empty<U>()->Empty<U>{return Empty::<U>{};}").record_constructions(),
        1
    );
    assert!(
        !check("fn read<U>(x:U)->U{return Box::<U>{value:x,weight:1}.value;}").bodies()[0]
            .implicit_unit_return()
    );
}
#[test]
fn missing_duplicate_unknown_incompatible_and_implicit_constructors_refuse() {
    for source in [
        "fn bad<U>(x:U){Box::<U>{value:x};}",
        "fn bad<U>(x:U){Box::<U>{value:x,value:x};}",
        "fn bad<U>(x:U){Box::<U>{value:x,missing:1};}",
        "fn bad<U>(x:U){Box::<U>{value:1,weight:1};}",
        "fn bad<U>(x:U){Box::<U,U>{value:x,weight:1};}",
        "fn bad<U>(x:U){Box{value:x,weight:1};}",
        "fn bad<U>(x:U){Alias::<U>{value:x,weight:1};}",
        "fn bad<U>(x:U){return;Box::<U>{value:missing,weight:1};}",
        "fn bad<U>(x:U){module::Box::<U>{value:x,weight:1};}",
    ] {
        assert!(
            check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
}
#[test]
fn type_bounds_and_type_value_namespaces_remain_explicit() {
    assert!(check_generic_copy_bodies(
        &table("fn bad<U>(x:U){Keyed::<U>{value:x};}"),
        GenericCopyBodyLimits::DEFAULT
    )
    .is_err());
    assert!(
        !check("fn good<U:std::Key>(x:U)->Keyed<U>{return Keyed::<U>{value:x};}").bodies()[0]
            .implicit_unit_return()
    );
    assert_eq!(
        check("fn names<U>(x:U)->Box<U>{let Box=x;return Box::<U>{value:Box,weight:1};}")
            .record_constructions(),
        1
    );
}
#[test]
fn constructor_and_shared_input_generated_field_work_budgets_are_exact() {
    let signatures = table("fn make<U>(x:U)->Box<U>{return Box::<U>{value:x,weight:1};}");
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
        max_record_constructions: 1,
        calls,
        max_field_nodes: checked.field_nodes(),
        max_field_edges: checked.field_edges(),
        max_work: checked.work(),
        ..GenericCopyBodyLimits::DEFAULT
    };
    assert!(check_generic_copy_bodies(&signatures, exact).is_ok());
    for limits in [
        GenericCopyBodyLimits {
            max_record_constructions: 0,
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
            calls: GenericBodyCallLimits {
                arguments: GenericArgumentCheckLimits {
                    max_input_nodes: usage.input_nodes - 1,
                    ..calls.arguments
                },
                ..calls
            },
            ..exact
        },
        GenericCopyBodyLimits {
            calls: GenericBodyCallLimits {
                arguments: GenericArgumentCheckLimits {
                    max_name_bytes: usage.name_bytes - 1,
                    ..calls.arguments
                },
                ..calls
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
fn constructor_prefix_tokens_and_names_are_charged_and_profiles_do_not_reset() {
    let source = "fn make<U>(x:U)->Box<U>{return Box::<U>{value:x,weight:1};}";
    let signatures = table(source);
    let checked = check_generic_copy_bodies(&signatures, GenericCopyBodyLimits::DEFAULT).unwrap();
    let usage = checked.call_usage();
    let base = GenericBodyCallLimits::DEFAULT;
    for arguments in [
        GenericArgumentCheckLimits {
            max_arguments: usage.type_arguments - 1,
            ..base.arguments
        },
        GenericArgumentCheckLimits {
            max_source_bytes: usage.source_bytes - 1,
            ..base.arguments
        },
        GenericArgumentCheckLimits {
            max_tokens: usage.tokens - 1,
            ..base.arguments
        },
        GenericArgumentCheckLimits {
            max_input_edges: usage.input_edges - 1,
            ..base.arguments
        },
    ] {
        assert!(check_generic_copy_bodies(
            &signatures,
            GenericCopyBodyLimits {
                calls: GenericBodyCallLimits { arguments, ..base },
                ..GenericCopyBodyLimits::DEFAULT
            }
        )
        .is_err());
    }
    let first = signatures.signatures()[0].syntax();
    let second = parse_generic_function(
        "fn other<V>(x:V)->Box<V>{return Box::<V>{value:x,weight:1};}",
        GenericFunctionLimits::DEFAULT,
    )
    .unwrap();
    let combined = check_generic_signatures(
        signatures.types(),
        &[first, &second],
        GenericSignatureLimits::DEFAULT,
    )
    .unwrap();
    assert!(check_generic_copy_bodies(
        &combined,
        GenericCopyBodyLimits {
            max_record_constructions: 1,
            ..GenericCopyBodyLimits::DEFAULT
        }
    )
    .is_err());
    assert!(check_generic_copy_bodies(
        &combined,
        GenericCopyBodyLimits {
            calls: GenericBodyCallLimits {
                max_values: 2,
                ..base
            },
            ..GenericCopyBodyLimits::DEFAULT
        }
    )
    .is_err());
    assert!(check_generic_copy_bodies(
        &signatures,
        GenericCopyBodyLimits {
            field_expansion: GenericFieldSubstitutionLimits {
                max_work: 0,
                ..GenericFieldSubstitutionLimits::DEFAULT
            },
            ..GenericCopyBodyLimits::DEFAULT
        }
    )
    .is_err());
    assert!(check_program_source("strict", source, ProgramSourceLimits::DEFAULT).is_err());
}

#[test]
fn parenthesized_record_literals_keep_the_original_head_and_value_ranges() {
    let source = "fn make<U>(x:U)->U{return ((Box::<U>{value:x,weight:1})).value;}";
    let checked = check(source);
    let body = &checked.bodies()[0];
    let constructor = body
        .expressions()
        .iter()
        .find(|e| matches!(e.kind, GenericCopyExpressionKind::Record { .. }))
        .unwrap();
    let GenericCopyExpressionKind::Record { type_range, .. } = constructor.kind else {
        panic!()
    };
    assert_eq!(&source[type_range.start..type_range.end], "Box::<U>");
    assert_eq!(
        &source[constructor.span.start..constructor.span.end],
        "((Box::<U>{value:x,weight:1}))"
    );
}
#[test]
fn record_initializers_and_generic_call_arguments_compose_in_original_source() {
    let checked =
        check("fn make<U>(x:U)->Box<U>{return ((Box::<U>{value:make::<U>(x).value,weight:1}));}");
    assert_eq!(checked.record_constructions(), 1);
    assert_eq!(checked.call_usage().calls, 1);
    let checked=check("fn relay<U>(x:Box<U>)->Box<U>{return relay::<U>((Box::<U>{value:x.value,weight:x.weight}));}");
    assert_eq!(checked.record_constructions(), 1);
    assert_eq!(checked.call_usage().calls, 1);
    let body = &checked.bodies()[0];
    let GenericCopyExpressionKind::Call { arguments, .. } =
        &body.expressions().last().unwrap().kind
    else {
        panic!()
    };
    assert!(matches!(
        body.expression(arguments[0].expression).unwrap().kind,
        GenericCopyExpressionKind::Record { .. }
    ));
}
