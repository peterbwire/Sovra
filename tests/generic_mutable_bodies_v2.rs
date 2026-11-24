//! Fixed symbolic mutable locals and direct assignment.
use sovra::compiler::v2::*;
fn table(source: &str) -> CheckedGenericSignatures {
    let registry = register_generic_types(&[], GenericRegistryLimits::DEFAULT).unwrap();
    let env =
        prepare_generic_aliases(&registry, &[], GenericAliasEnvironmentLimits::DEFAULT).unwrap();
    let types = check_generic_declarations(&env, &[], GenericDeclarationLimits::DEFAULT).unwrap();
    let syntax = parse_generic_function(source, GenericFunctionLimits::DEFAULT).unwrap();
    check_generic_signatures(&types, &[&syntax], GenericSignatureLimits::DEFAULT).unwrap()
}
fn check(source: &str) -> CheckedGenericCopyBodies {
    check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).unwrap()
}
#[test]
fn opaque_mutable_copy_preserves_binding_and_original_target_ranges() {
    let source = "// λ😀\nfn copy<T>(x:T)->T{let mut y=x;y=x;return y;}";
    let checked = check(source);
    let body = &checked.bodies()[0];
    let binding = &body.bindings()[1];
    assert_eq!(binding.kind, GenericCopyBindingKind::MutableLocal);
    assert_eq!(&source[binding.name_span.start..binding.name_span.end], "y");
    let GenericCopyStatement::Assign {
        binding: id,
        target_span,
        value,
        conversion,
        ..
    } = body.statements()[1]
    else {
        panic!()
    };
    assert_eq!(id, binding.id);
    assert_eq!(&source[target_span.start..target_span.end], "y");
    assert_eq!(conversion, GenericCopyConversion::Identity);
    assert!(
        matches!(body.expression(value).unwrap().kind,GenericCopyExpressionKind::Read(id) if id==body.bindings()[0].id)
    );
}
#[test]
fn assignment_has_explicit_widening_without_changing_declared_type() {
    let checked = check("fn widen<T>()->Float{let mut value:Float=1;value=2;return value;}");
    let body = &checked.bodies()[0];
    assert_eq!(
        body.bindings()[0].ty.primitive(),
        Some(GenericPrimitive::Float)
    );
    assert!(matches!(
        body.statements()[1],
        GenericCopyStatement::Assign {
            conversion: GenericCopyConversion::IntToFloat,
            ..
        }
    ));
}
#[test]
fn writes_reject_immutable_parameters_unknown_targets_and_type_changes() {
    for source in [
        "fn bad<T>(x:T)->T{x=x;return x;}",
        "fn bad<T>(x:T)->T{let y=x;y=x;return y;}",
        "fn bad<T>(x:T)->T{missing=x;return x;}",
        "fn bad<T>(x:T)->T{let mut y=x;y=1;return y;}",
        "fn bad<T>()->Int{let mut y=1;y=2.0;return y;}",
        "fn bad<T>(x:[T])->[T]{let mut y=x;y[0]=x;return y;}",
        "fn bad<T>(x:T)->T{let mut y=x;y.field=x;return y;}",
        "fn bad<T>(x:T)->T{let mut y=x;return y;y=1;}",
    ] {
        assert!(
            check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
}
#[test]
fn shadowing_resolves_initializer_first_and_assignment_to_current_binding() {
    let checked = check("fn shadow<T>(x:T)->T{let mut x=x;let mut x=x;x=x;return x;}");
    let body = &checked.bodies()[0];
    assert!(
        matches!(body.expressions()[0].kind,GenericCopyExpressionKind::Read(id) if id==body.bindings()[0].id)
    );
    assert!(
        matches!(body.expressions()[1].kind,GenericCopyExpressionKind::Read(id) if id==body.bindings()[1].id)
    );
    assert!(
        matches!(body.statements()[2],GenericCopyStatement::Assign{binding,..} if binding==body.bindings()[2].id)
    );
    assert!(check_generic_copy_bodies(
        &table("fn bad<T>(x:T)->T{let mut y=x;let y=y;y=x;return y;}"),
        GenericCopyBodyLimits::DEFAULT
    )
    .is_err());
}
#[test]
fn mutable_statements_charge_existing_cumulative_budgets() {
    let table = table("fn copy<T>(x:T)->T{let mut y=x;y=x;return y;}");
    let checked = check_generic_copy_bodies(&table, GenericCopyBodyLimits::DEFAULT).unwrap();
    let exact = GenericCopyBodyLimits {
        max_statements: checked.statement_count(),
        max_work: checked.work(),
        max_expressions: checked.expression_count(),
        ..GenericCopyBodyLimits::DEFAULT
    };
    assert!(check_generic_copy_bodies(&table, exact).is_ok());
    for limits in [
        GenericCopyBodyLimits {
            max_statements: exact.max_statements - 1,
            ..exact
        },
        GenericCopyBodyLimits {
            max_work: exact.max_work - 1,
            ..exact
        },
        GenericCopyBodyLimits {
            max_expressions: exact.max_expressions - 1,
            ..exact
        },
    ] {
        assert!(check_generic_copy_bodies(&table, limits).is_err());
    }
}
#[test]
fn typed_mutable_array_copies_retain_symbolic_type_without_element_mutation() {
    let checked = check("fn arrays<T>(x:[T],other:[T])->[T]{let mut y:[T]=x;y=other;return y;}");
    let body = &checked.bodies()[0];
    assert_eq!(
        body.bindings()[2].kind,
        GenericCopyBindingKind::MutableLocal
    );
    assert!(matches!(
        body.statements()[0],
        GenericCopyStatement::Let {
            annotation: Some(_),
            conversion: GenericCopyConversion::Identity,
            ..
        }
    ));
    assert!(matches!(
        body.statements()[1],
        GenericCopyStatement::Assign {
            conversion: GenericCopyConversion::Identity,
            ..
        }
    ));
    assert!(check_generic_copy_bodies(
        &table("fn bad<T,U>(x:T,z:U)->T{let mut y:T=x;y=z;return y;}"),
        GenericCopyBodyLimits::DEFAULT
    )
    .is_err());
}
