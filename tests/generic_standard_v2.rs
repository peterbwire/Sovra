//! Compiler-owned qualified standard enums over the complete checked source pipeline.
use sovra::compiler::v2::*;
fn run(source: &str) -> GenericSourceOutcome {
    let checked = compile_generic_source(source, "main", GenericSourceLimits::DEFAULT).unwrap();
    execute_generic_source(&checked, GenericVmPolicy::DEFAULT).unwrap()
}
#[test]
fn qualified_option_some_none_and_match_execute() {
    let out=run("fn main()->Int{let item=std::Option::<Int>::Some(42);match item{std::Option::Some(value)=>{return value;},std::Option::None()=>{return 0;}}}");
    assert_eq!(out.execution().value().as_int(), Some(42));
    let out=run("fn main()->Bool{let item=std::Option::<String>::None();match item{std::Option::Some(value)=>{return false;},std::Option::None()=>{return true;}}}");
    assert_eq!(out.execution().value().as_bool(), Some(true));
}
#[test]
fn result_ok_and_error_payloads_flow_through_generic_functions() {
    let out=run("fn pass<T,E>(x:std::Result<T,E>)->std::Result<T,E>{return x;} fn main()->String{let r=pass::<Int,String>(std::Result::<Int,String>::Err(\"bad\"));match r{std::Result::Ok(value)=>{return \"unexpected\";},std::Result::Err(error)=>{return error;}}}");
    assert_eq!(out.execution().value().as_str(), Some("bad"));
}
#[test]
fn ordinary_names_remain_available_and_nominally_distinct() {
    let out=run("enum Option<T>{Value(T)} enum Result<T,E>{Value(T),Error(E)} fn main()->Int{let local=Option::<Int>::Value(3);let standard=std::Option::<Int>::Some(4);match local{Option::Value(a)=>{match standard{std::Option::Some(b)=>{return a+b;},std::Option::None()=>{return 0;}}}}}");
    assert_eq!(out.execution().value().as_int(), Some(7));
}

#[test]
fn standard_and_user_types_cannot_be_interchanged_or_matched_with_foreign_qualifiers() {
    for source in [
        "enum Option<T>{Some(T),None} fn main()->std::Option<Int>{return Option::<Int>::Some(1);}",
        "fn main()->std::Result<Int,String>{return std::Result::<Int,Bool>::Err(false);}",
        "fn main()->std::Option<Int>{return std::Option::<Float>::Some(1.0);}",
        "fn main(){let x=std::Option::<Int>::Some(1);match x{Option::Some(v)=>{},Option::None()=>{}}}",
        "fn main(){let x=std::Result::<Int,String>::Ok(1);match x{std::Option::Some(v)=>{},std::Option::None()=>{}}}",
    ] {assert!(compile_generic_source(source,"main",GenericSourceLimits::DEFAULT).is_err(),"{source}");}
}
#[test]
fn payload_arity_type_arguments_and_exhaustiveness_remain_strict() {
    for source in [
        "fn main(){let x=std::Option::<Int>::Some();}",
        "fn main(){let x=std::Option::<Int>::None(1);}",
        "fn main(){let x=std::Result::<Int>::Ok(1);}",
        "fn main(){let x=std::Result::<Int,String>::Err(false);}",
        "fn main(){let x=std::Option::Some(1);}",
        "fn main(){let x=std::Option::<Int>::Some(1);match x{std::Option::Some(v)=>{}}}",
        "fn main(){let x=std::Option::<Int>::Some(1);match x{std::Option::Some(v)=>{},std::Option::Some(v)=>{}}}",
        "fn main(){let x=std::Option::<Int>::Some(1);match x{std::Option::Some()=>{},std::Option::None()=>{}}}",
    ] {assert!(compile_generic_source(source,"main",GenericSourceLimits::DEFAULT).is_err(),"{source}");}
}
#[test]
fn qualified_standard_declarations_cannot_be_forged_from_source() {
    for source in [
        "enum std::Option<T>{Some(T),None} fn main(){}",
        "enum std::Result<T,E>{Ok(T),Err(E)} fn main(){}",
        "mod std{enum Option<T>{Some(T),None}} fn main(){}",
    ] {
        assert!(compile_generic_source(source, "main", GenericSourceLimits::DEFAULT).is_err());
    }
    let parsed =
        parse_generic_enum("enum Option<T>{Some(T),None}", GenericEnumLimits::DEFAULT).unwrap();
    assert_eq!(parsed.standard_identity(), None);
    assert_eq!(parsed.qualified_name(), "Option");
}
#[test]
fn standard_definitions_have_separate_provenance_and_consume_type_stage_budgets() {
    let program =
        compile_generic_source("fn main(){}", "main", GenericSourceLimits::DEFAULT).unwrap();
    assert_eq!(program.declarations().len(), 1);
    assert_eq!(program.usage().declarations, 1);
    let definitions = program
        .ir()
        .plans()
        .layouts()
        .literals()
        .concrete()
        .graph()
        .bodies()
        .signatures()
        .types()
        .definitions();
    let standard: Vec<_> = definitions
        .iter()
        .filter_map(|definition| {
            if let CheckedGenericDefinition::Enum(e) = definition {
                e.syntax().standard_identity()
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        standard,
        [GenericStandardEnum::Option, GenericStandardEnum::Result]
    );
    for count in [0, 1, 2] {
        let mut limits = GenericSourceLimits::DEFAULT;
        limits.registry.max_declarations = count;
        assert!(compile_generic_source("fn main(){}", "main", limits).is_err());
    }
    let mut limits = GenericSourceLimits::DEFAULT;
    limits.registry.max_declarations = 3;
    compile_generic_source("fn main(){}", "main", limits).unwrap();
    limits.declarations.max_declarations = 1;
    assert!(compile_generic_source("fn main(){}", "main", limits).is_err());
}
#[test]
fn nested_options_results_and_owned_array_snapshots_execute() {
    let out=run("fn main()->Int{let mut a:[Int]=[7];let result=std::Result::<std::Option<[Int]>,String>::Ok(std::Option::<[Int]>::Some(a));a[0]=9;match result{std::Result::Err(e)=>{return 0;},std::Result::Ok(option)=>{match option{std::Option::None()=>{return 0;},std::Option::Some(old)=>{return old[0];}}}}}");
    assert_eq!(out.execution().value().as_int(), Some(7));
}
#[test]
fn concrete_specializations_keep_standard_type_arguments_distinct() {
    let out=run("fn pass<T>(x:T)->T{return x;} fn main()->Bool{let a=pass::<std::Option<Int>>(std::Option::<Int>::Some(3));let b=pass::<std::Option<String>>(std::Option::<String>::Some(\"ok\"));return a==std::Option::<Int>::Some(3)&&b==std::Option::<String>::Some(\"ok\");}");
    assert_eq!(out.execution().value().as_bool(), Some(true));
}
#[test]
fn old_native_feature_profile_is_retained_and_does_not_gain_standard_authority() {
    let source = "fn main(){let x=std::Option::<Int>::Some(1);}";
    let checked = compile_generic_source(source, "main", GenericSourceLimits::DEFAULT).unwrap();
    let image = checked.encode_generic_native_image().unwrap();
    assert_eq!(u32::from_le_bytes(image[8..12].try_into().unwrap()), 7);
    execute_generic_native_image(&image).unwrap();
    let mut old = image;
    old[8..12].copy_from_slice(&1u32.to_le_bytes());
    assert!(decode_generic_native_image(&old).is_err());
    let checked =
        compile_generic_source("fn main(){}", "main", GenericSourceLimits::DEFAULT).unwrap();
    let mut old = checked.encode_generic_native_image().unwrap();
    old[8..12].copy_from_slice(&1u32.to_le_bytes());
    execute_generic_native_image(&old).unwrap();
}

#[test]
fn unit_payload_some_and_nullary_none_are_distinct() {
    let some =
        run("fn unit(){} fn main()->std::Option<Unit>{return std::Option::<Unit>::Some(unit());}");
    assert!(some.execution().value().payload().unwrap().is_unit());
    let none = run("fn main()->std::Option<Unit>{return std::Option::<Unit>::None();}");
    assert!(none.execution().value().payload().is_none());
    assert_ne!(
        some.execution().value().enum_tag(),
        none.execution().value().enum_tag()
    );
}

#[test]
fn transparent_aliases_resolve_to_the_same_standard_nominal_identity() {
    let out=run("type Maybe<T>=std::Option<T>; fn id<T>(x:Maybe<T>)->std::Option<T>{return x;} fn main()->Bool{let item:Maybe<Int>=std::Option::<Int>::Some(5);return id::<Int>(item)==std::Option::<Int>::Some(5);}");
    assert_eq!(out.execution().value().as_bool(), Some(true));
}
#[test]
fn standard_wrappers_do_not_prove_the_primitive_key_bound() {
    let source="fn key<T:std::Key>(x:T)->T{return x;} fn main(){let x=key::<std::Option<Int>>(std::Option::<Int>::Some(1));}";
    assert!(compile_generic_source(source, "main", GenericSourceLimits::DEFAULT).is_err());
}
#[test]
fn qualified_nominal_label_bytes_are_charged_to_materialization() {
    let source = "fn main()->std::Option<Int>{return std::Option::<Int>::Some(1);}";
    let checked = compile_generic_source(source, "main", GenericSourceLimits::DEFAULT).unwrap();
    let mut policy = GenericVmPolicy::DEFAULT;
    policy.runtime.materialization.max_text_bytes = Some(15);
    let result = execute_generic_source(&checked, policy.clone()).unwrap();
    assert_eq!(result.execution().value().footprint(), (2, 15));
    policy.runtime.materialization.max_text_bytes = Some(14);
    assert!(execute_generic_source(&checked, policy).is_err());
}
