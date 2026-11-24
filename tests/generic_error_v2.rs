//! Compiler-owned std::Error construction, nominal identity, fields and admission.
use sovra::compiler::v2::*;
fn compile(source: &str) -> CompiledGenericSource {
    compile_generic_source(source, "main", GenericSourceLimits::DEFAULT).unwrap()
}
fn run(source: &str) -> GenericSourceOutcome {
    execute_generic_source(&compile(source), GenericVmPolicy::DEFAULT).unwrap()
}
#[test]
fn canonical_fields_and_source_order_initializers_execute() {
    let out=run("fn main()->std::Error{return std::Error{offset:std::Option::<Int>::None(),message:\"bad\",code:\"E\"};}");
    let value = out.execution().value();
    assert_eq!(value.field(0).unwrap().as_str(), Some("E"));
    assert_eq!(value.field(1).unwrap().as_str(), Some("bad"));
    assert!(value.field(2).unwrap().payload().is_none());
    assert_eq!(value.footprint(), (4, 46));
}
#[test]
fn result_error_payloads_and_optional_offsets_flow_through_calls() {
    let out=run("fn fail<T>()->std::Result<T,std::Error>{return std::Result::<T,std::Error>::Err(std::Error{code:\"parse\",message:\"invalid\",offset:std::Option::<Int>::Some(12)});} fn main()->Int{let result=fail::<String>();match result{std::Result::Ok(text)=>{return 0;},std::Result::Err(error)=>{let offset=error.offset;match offset{std::Option::Some(value)=>{return value;},std::Option::None()=>{return 0;}}}}}");
    assert_eq!(out.execution().value().as_int(), Some(12));
}
#[test]
fn user_error_types_remain_separate_and_standard_identity_cannot_be_forged() {
    let source="struct Error<T>{value:T} fn main()->Bool{let user=Error::<Int>{value:3};let error=std::Error{code:\"e\",message:\"m\",offset:std::Option::<Int>::None()};return user.value==3&&error.code==\"e\";}";
    assert_eq!(run(source).execution().value().as_bool(), Some(true));
    for source in [
        "struct std::Error{code:String} fn main(){}",
        "struct Error<T>{value:T} fn main()->std::Error{return Error::<Int>{value:1};}",
    ] {
        assert!(compile_generic_source(source, "main", GenericSourceLimits::DEFAULT).is_err());
    }
    let syntax =
        parse_generic_record("struct Error<T>{value:T}", GenericRecordLimits::DEFAULT).unwrap();
    assert!(!syntax.is_standard_error());
    assert_eq!(syntax.qualified_name(), "Error");
}
#[test]
fn missing_duplicate_unknown_or_incompatible_fields_and_type_arguments_refuse() {
    for construction in [
        "std::Error{code:\"e\",message:\"m\"}",
        "std::Error{code:\"e\",message:\"m\",offset:1}",
        "std::Error{code:1,message:\"m\",offset:std::Option::<Int>::None()}",
        "std::Error{code:\"e\",message:\"m\",offset:std::Option::<String>::None()}",
        "std::Error{code:\"e\",code:\"m\",offset:std::Option::<Int>::None()}",
        "std::Error{code:\"e\",message:\"m\",other:std::Option::<Int>::None()}",
        "std::Error::<Int>{code:\"e\",message:\"m\",offset:std::Option::<Int>::None()}",
    ] {
        let source = format!("fn main()->std::Error{{return {construction};}}");
        assert!(
            compile_generic_source(&source, "main", GenericSourceLimits::DEFAULT).is_err(),
            "{construction}"
        );
    }
}
#[test]
fn aliases_grouping_comments_and_owned_snapshots_keep_the_checked_type() {
    let out=run("type Failure<T>=std::Result<T,std::Error>;fn main()->Bool{let mut error=(std // qualified head\n::Error{code:\"e\",message:\"m\",offset:std::Option::<Int>::None()});let old=error;error=std::Error{code:\"changed\",message:\"new\",offset:std::Option::<Int>::Some(9)};let result:Failure<Int>=std::Result::<Int,std::Error>::Err(old);match result{std::Result::Ok(value)=>{return false;},std::Result::Err(original)=>{return original.code==\"e\"&&error.code==\"changed\";}}}");
    assert_eq!(out.execution().value().as_bool(), Some(true));
}
#[test]
fn standard_record_provenance_and_zero_parameter_shape_are_explicit() {
    let p = compile("fn main(){}");
    let definitions = p
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
    let errors: Vec<_> = definitions
        .iter()
        .filter_map(|d| {
            if let CheckedGenericDefinition::Record(r) = d {
                r.syntax().is_standard_error().then_some(r)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(errors.len(), 1);
    let syntax = errors[0].syntax();
    assert_eq!(syntax.qualified_name(), "std::Error");
    assert!(syntax.parameters().parameters().is_empty());
    assert_eq!(syntax.fields().len(), 3);
    assert_eq!(p.declarations().len(), 1);
}
#[test]
fn semantic_and_constructor_syntax_limits_include_standard_record_work() {
    let source="fn main()->std::Error{return std::Error{code:\"e\",message:\"m\",offset:std::Option::<Int>::None()};}";
    for allowance in [0, 1, 2] {
        let mut limits = GenericSourceLimits::DEFAULT;
        limits.registry.max_declarations = allowance;
        assert!(compile_generic_source(source, "main", limits).is_err());
    }
    for allowance in [0, 1, 2, 3] {
        let mut limits = GenericSourceLimits::DEFAULT;
        limits.bodies.calls.syntax.types.max_tokens = allowance;
        assert!(compile_generic_source(source, "main", limits).is_err());
    }
    let mut policy = GenericVmPolicy::DEFAULT;
    policy.runtime.materialization.max_text_bytes = Some(43);
    assert!(execute_generic_source(&compile(source), policy).is_err());
}
#[test]
fn standard_error_requires_its_native_feature_and_old_profiles_are_retained() {
    let p=compile("fn main(){let error=std::Error{code:\"e\",message:\"m\",offset:std::Option::<Int>::None()};}");
    let image = p.encode_generic_native_image().unwrap();
    assert_eq!(u32::from_le_bytes(image[8..12].try_into().unwrap()), 7);
    execute_generic_native_image(&image).unwrap();
    for features in [1u32, 3, 5] {
        let mut old = image.clone();
        old[8..12].copy_from_slice(&features.to_le_bytes());
        assert!(decode_generic_native_image(&old).is_err());
    }
    let p = compile("fn main(){let optional=std::Option::<Int>::Some(1);}");
    let mut old = p.encode_generic_native_image().unwrap();
    old[8..12].copy_from_slice(&3u32.to_le_bytes());
    execute_generic_native_image(&old).unwrap();
}

#[test]
fn plain_standard_constructor_type_input_limits_are_exact() {
    let source="fn make(offset:std::Option<Int>)->std::Error{return std::Error{code:\"e\",message:\"m\",offset:offset};}fn main(){}";
    macro_rules! exact {
        ($field:ident,$value:expr) => {{
            let mut limits = GenericSourceLimits::DEFAULT;
            limits.bodies.calls.syntax.types.$field = $value;
            compile_generic_source(source, "main", limits).unwrap();
            limits.bodies.calls.syntax.types.$field = $value - 1;
            assert!(
                compile_generic_source(source, "main", limits).is_err(),
                stringify!($field)
            );
        }};
    }
    exact!(max_source_bytes, 10);
    exact!(max_tokens, 4);
    exact!(max_nodes, 1);
    exact!(max_name_bytes, 10);
    exact!(max_depth, 1);
    let mut limits = GenericSourceLimits::DEFAULT;
    limits.bodies.calls.syntax.max_arguments = 0;
    compile_generic_source(source, "main", limits).unwrap();
}
