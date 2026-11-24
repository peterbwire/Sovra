//! Composite field substitution preserves graph identity and per-node sources.
use sovra::compiler::v2::*;
fn table(source: &str) -> CheckedGenericSignatures {
    let record = parse_generic_record(
        "struct Box<T>{count:Int,values:[String],value:T,nested:Box<Int>,dependent:[T],wrapped:Box<[T]>,aliased:Rows<T>,erased:Ignore<T>}",
        GenericRecordLimits::DEFAULT,
    )
    .unwrap();
    let alias = parse_generic_alias("type Ignore<T>=Int;", GenericAliasLimits::DEFAULT).unwrap();
    let pair =
        parse_generic_record("struct Pair<A,B>{second:B}", GenericRecordLimits::DEFAULT).unwrap();
    let rows = parse_generic_alias("type Rows<T>=[T];", GenericAliasLimits::DEFAULT).unwrap();
    let declarations = [
        GenericTypeDeclaration::Record(&record),
        GenericTypeDeclaration::Alias(&alias),
        GenericTypeDeclaration::Alias(&rows),
        GenericTypeDeclaration::Record(&pair),
    ];
    let registry = register_generic_types(&declarations, GenericRegistryLimits::DEFAULT).unwrap();
    let env = prepare_generic_aliases(
        &registry,
        &[&alias, &rows],
        GenericAliasEnvironmentLimits::DEFAULT,
    )
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
fn composite_arrays_preserve_caller_parameters_and_match_return_types() {
    let checked = check("fn values<U>(box:Box<U>)->[U]{return box.dependent;}");
    let body = &checked.bodies()[0];
    let ty = body.expressions().last().unwrap().ty.expanded().unwrap();
    let ResolvedGenericTypeKind::Array { element } = ty.nodes()[ty.root()].kind else {
        panic!()
    };
    assert!(
        matches!(ty.nodes()[element].kind,ResolvedGenericTypeKind::Parameter(id) if id==body.signature().syntax().parameters().find("U").unwrap())
    );
    assert!(checked.field_nodes() > 0);
    assert!(checked.field_edges() > 0);
    assert!(
        !check("fn concrete<U>(box:Box<Int>)->[Int]{return box.dependent;}").bodies()[0]
            .implicit_unit_return()
    );
}
#[test]
fn nested_nominal_fields_and_repeated_composition_keep_exact_roots() {
    assert!(
        !check("fn nested<U>(box:Box<U>)->U{return box.wrapped.value[0];}").bodies()[0]
            .implicit_unit_return()
    );
    assert!(
        !check("fn repeat<U>(box:Box<U>)->[[U]]{return box.wrapped.dependent;}").bodies()[0]
            .implicit_unit_return()
    );
    assert!(check_generic_copy_bodies(
        &table("fn bad<U,V>(box:Box<U>)->[V]{return box.dependent;}"),
        GenericCopyBodyLimits::DEFAULT
    )
    .is_err());
}
#[test]
fn composed_origins_resolve_to_their_actual_input_fragments() {
    let checked = check("fn values<U>(box:Box<U>)->[U]{return box.dependent;}");
    let body = &checked.bodies()[0];
    let ty = body.expressions().last().unwrap().ty.expanded().unwrap();
    let ResolvedGenericTypeKind::Array { element } = ty.nodes()[ty.root()].kind else {
        panic!()
    };
    let root = ty.nodes()[ty.root()].origin;
    let arg = ty.nodes()[element].origin;
    assert_eq!(
        &ty.node_source(ty.root()).unwrap()[root.span.start..root.span.end],
        "[T]"
    );
    assert_eq!(
        &ty.node_source(element).unwrap()[arg.span.start..arg.span.end],
        "U"
    );
    assert!(ty.node_source(usize::MAX).is_none());
}
#[test]
fn generated_nodes_edges_work_and_independent_depth_limits_are_enforced() {
    let table = table("fn values<U>(box:Box<U>)->[U]{return box.dependent;}");
    let checked = check_generic_copy_bodies(&table, GenericCopyBodyLimits::DEFAULT).unwrap();
    let exact = GenericCopyBodyLimits {
        max_field_nodes: checked.field_nodes(),
        max_field_edges: checked.field_edges(),
        max_work: checked.work(),
        ..GenericCopyBodyLimits::DEFAULT
    };
    assert!(check_generic_copy_bodies(&table, exact).is_ok());
    for limits in [
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
            field_expansion: GenericFieldSubstitutionLimits {
                max_nodes: 1,
                ..exact.field_expansion
            },
            ..exact
        },
        GenericCopyBodyLimits {
            field_expansion: GenericFieldSubstitutionLimits {
                max_depth: 1,
                ..exact.field_expansion
            },
            ..exact
        },
    ] {
        assert!(check_generic_copy_bodies(&table, limits).is_err());
    }
}
#[test]
fn cumulative_substitution_ledgers_do_not_reset_between_field_expressions() {
    let once = check("fn one<U>(box:Box<U>){box.dependent;}");
    let table = table("fn two<U>(box:Box<U>){box.dependent;box.dependent;}");
    assert!(check_generic_copy_bodies(
        &table,
        GenericCopyBodyLimits {
            max_field_nodes: once.field_nodes(),
            ..GenericCopyBodyLimits::DEFAULT
        }
    )
    .is_err());
    assert!(check_generic_copy_bodies(
        &table,
        GenericCopyBodyLimits {
            max_field_edges: once.field_edges(),
            ..GenericCopyBodyLimits::DEFAULT
        }
    )
    .is_err());
}

#[test]
fn alias_target_and_caller_argument_sources_survive_composition() {
    let checked = check("fn values<U>(box:Box<U>)->[U]{return box.aliased;}");
    let body = &checked.bodies()[0];
    let ty = body.expressions().last().unwrap().ty.expanded().unwrap();
    let ResolvedGenericTypeKind::Array { element } = ty.nodes()[ty.root()].kind else {
        panic!()
    };
    assert!(ty.nodes()[ty.root()].origin.alias.is_some());
    let origin = ty.nodes()[ty.root()].origin;
    assert_eq!(
        &ty.node_source(ty.root()).unwrap()[origin.span.start..origin.span.end],
        "[T]"
    );
    let origin = ty.nodes()[element].origin;
    assert_eq!(
        &ty.node_source(element).unwrap()[origin.span.start..origin.span.end],
        "U"
    );
}
