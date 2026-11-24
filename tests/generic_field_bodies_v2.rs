//! Parameter-independent symbolic record field reads.
use sovra::compiler::v2::*;
fn table(source: &str) -> CheckedGenericSignatures {
    let record = parse_generic_record(
        "struct Box<T>{count:Int,values:[String],value:T,nested:Box<Int>,dependent:[T],erased:Ignore<T>}",
        GenericRecordLimits::DEFAULT,
    )
    .unwrap();
    let alias = parse_generic_alias("type Ignore<T>=Int;", GenericAliasLimits::DEFAULT).unwrap();
    let declarations = [
        GenericTypeDeclaration::Record(&record),
        GenericTypeDeclaration::Alias(&alias),
    ];
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
fn concrete_fields_retain_nominal_identity_source_and_receiver() {
    let source = "// λ😀\nfn count<T>(box:Box<T>)->Int{return box.count;}";
    let checked = check(source);
    let body = &checked.bodies()[0];
    let e = &body.expressions()[1];
    assert_eq!(&source[e.span.start..e.span.end], "box.count");
    assert_eq!(e.ty.primitive(), Some(GenericPrimitive::Int));
    let GenericCopyExpressionKind::Field {
        receiver,
        field_index,
        ..
    } = e.kind
    else {
        panic!()
    };
    assert_eq!(field_index, 0);
    assert!(
        matches!(body.expression(receiver).unwrap().kind,GenericCopyExpressionKind::Read(id) if id==body.bindings()[0].id)
    );
}
#[test]
fn concrete_array_and_nested_record_fields_compose_with_existing_operations() {
    assert!(
        !check("fn item<T>(box:Box<T>)->String{return box.values[0];}").bodies()[0]
            .implicit_unit_return()
    );
    assert!(
        !check("fn nested<T>(box:Box<T>)->Int{return box.nested.count+1;}").bodies()[0]
            .implicit_unit_return()
    );
}
#[test]
fn dependent_fields_unknown_fields_and_nonrecords_are_refused() {
    for source in [
        "fn bad<T>(box:Box<T>)->Int{return box.missing;}",
        "fn bad<T>(x:T)->Int{return x.count;}",
        "fn bad<T>(x:Int)->Int{return x.count;}",
        "fn bad<T>(box:Box<T>){return;box.missing;}",
        "fn bad<T>(box:Box<T>){let mut b=box;b.count=1;}",
    ] {
        assert!(
            check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
}
#[test]
fn field_lookup_and_type_walk_charge_expression_and_work_limits() {
    let table = table("fn count<T>(box:Box<T>)->Int{return box.nested.count;}");
    let checked = check_generic_copy_bodies(&table, GenericCopyBodyLimits::DEFAULT).unwrap();
    let exact = GenericCopyBodyLimits {
        max_expressions: checked.expression_count(),
        max_work: checked.work(),
        ..GenericCopyBodyLimits::DEFAULT
    };
    assert!(check_generic_copy_bodies(&table, exact).is_ok());
    for limits in [
        GenericCopyBodyLimits {
            max_expressions: exact.max_expressions - 1,
            ..exact
        },
        GenericCopyBodyLimits {
            max_work: exact.max_work - 1,
            ..exact
        },
    ] {
        assert!(check_generic_copy_bodies(&table, limits).is_err());
    }
}

#[test]
fn unused_alias_parameter_nodes_do_not_make_a_concrete_field_dependent() {
    let checked = check("fn erased<T>(box:Box<T>)->Int{return box.erased+1;}");
    assert!(!checked.bodies()[0].implicit_unit_return());
}
