//! Concrete String facts preserve original source; no execution or allocation.
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
fn string_operators_preserve_result_and_operand_identity() {
    for (op, expected) in [
        ("+", GenericStringOperator::Concatenate),
        ("<", GenericStringOperator::Less),
        ("<=", GenericStringOperator::LessEqual),
        (">", GenericStringOperator::Greater),
        (">=", GenericStringOperator::GreaterEqual),
    ] {
        let result = if op == "+" { "String" } else { "Bool" };
        let checked = check(&format!(
            "fn text<T>(a:String,b:String)->{result}{{return a{op}b;}}"
        ));
        let body = &checked.bodies()[0];
        let e = &body.expressions()[2];
        assert_eq!(
            e.ty.primitive(),
            Some(if op == "+" {
                GenericPrimitive::String
            } else {
                GenericPrimitive::Bool
            })
        );
        let GenericCopyExpressionKind::String {
            operator,
            left,
            right,
        } = e.kind
        else {
            panic!()
        };
        assert_eq!(operator, expected);
        assert!(
            matches!(body.expression(left).unwrap().kind,GenericCopyExpressionKind::Read(id) if id==body.bindings()[0].id)
        );
        assert!(
            matches!(body.expression(right).unwrap().kind,GenericCopyExpressionKind::Read(id) if id==body.bindings()[1].id)
        );
    }
}
#[test]
fn no_formatting_opaque_string_assumptions_or_unsupported_string_operators() {
    for source in [
        "fn bad<T>(a:T,b:T)->T{return a+b;}",
        "fn bad<T:std::Key>(a:T,b:T)->Bool{return a<b;}",
        "fn bad<T>(a:String,b:Int)->String{return a+b;}",
        "fn bad<T>(a:Int,b:String)->String{return a+b;}",
        "fn bad<T>(a:String,b:Bool)->Bool{return a<b;}",
        "fn bad<T>(a:String,b:String)->String{return a-b;}",
        "fn bad<T>(a:String,b:String)->String{return a*b;}",
        "fn bad<T>(a:String,b:String)->String{return a/b;}",
    ] {
        assert!(
            check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
}
#[test]
fn unicode_nested_concatenation_retains_original_expression_spans() {
    let source = "// λ😀\nfn text<T>(a:String)->Bool{return a+\"λ😀\"<\"z\";}";
    let checked = check(source);
    let body = &checked.bodies()[0];
    let e = body.expressions().last().unwrap();
    assert_eq!(&source[e.span.start..e.span.end], "a+\"λ😀\"<\"z\"");
    let GenericCopyExpressionKind::String {
        operator: GenericStringOperator::Less,
        left,
        ..
    } = e.kind
    else {
        panic!()
    };
    assert!(matches!(
        body.expression(left).unwrap().kind,
        GenericCopyExpressionKind::String {
            operator: GenericStringOperator::Concatenate,
            ..
        }
    ));
}
#[test]
fn string_nodes_charge_cumulative_expression_and_work_limits() {
    let table = table("fn text<T>(a:String,b:String)->String{return a+b+a;}");
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
fn transparent_string_aliases_are_accepted() {
    let alias = parse_generic_alias("type Text<T>=String;", GenericAliasLimits::DEFAULT).unwrap();
    let declarations = [GenericTypeDeclaration::Alias(&alias)];
    let registry = register_generic_types(&declarations, GenericRegistryLimits::DEFAULT).unwrap();
    let env = prepare_generic_aliases(&registry, &[&alias], GenericAliasEnvironmentLimits::DEFAULT)
        .unwrap();
    let types =
        check_generic_declarations(&env, &declarations, GenericDeclarationLimits::DEFAULT).unwrap();
    let syntax = parse_generic_function(
        "fn text<T>(a:Text<T>,b:String)->String{return a+b;}",
        GenericFunctionLimits::DEFAULT,
    )
    .unwrap();
    let signatures =
        check_generic_signatures(&types, &[&syntax], GenericSignatureLimits::DEFAULT).unwrap();
    assert!(check_generic_copy_bodies(&signatures, GenericCopyBodyLimits::DEFAULT).is_ok());
}
