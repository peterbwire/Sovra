//! Bounded symbolic Option/Result failure compatibility without enabling source question marks.
use sovra::compiler::v2::*;
fn bodies(functions: &str) -> CheckedGenericCopyBodies {
    let source = format!("{functions} fn main(){{}}");
    compile_generic_source(&source, "main", GenericSourceLimits::DEFAULT)
        .unwrap()
        .ir()
        .plans()
        .layouts()
        .literals()
        .concrete()
        .graph()
        .bodies()
        .clone()
}
fn request(bodies: &CheckedGenericCopyBodies, function: &str) -> GenericPropagationRequest {
    let function = bodies.signatures().find(function).unwrap();
    let body = bodies.body(function).unwrap();
    let operand=body.expressions().iter().find(|e|matches!(e.kind,GenericCopyExpressionKind::Read(binding) if body.binding(binding).unwrap().name.as_ref()=="x")).unwrap().id;
    GenericPropagationRequest { function, operand }
}
fn plan(functions: &str) -> CheckedGenericPropagations {
    let b = bodies(functions);
    plan_generic_propagations(
        &b,
        &[request(&b, "test")],
        GenericPropagationLimits::DEFAULT,
    )
    .unwrap()
}
#[test]
fn option_success_and_enclosing_return_payloads_may_differ() {
    let p=plan("fn test(x:std::Option<Int>)->std::Option<String>{let copy=x;return std::Option::<String>::None();}");
    let site = &p.plans()[0];
    assert_eq!(site.kind(), GenericStandardEnum::Option);
    assert_eq!(site.success_variant(), 0);
    assert_eq!(site.failure_variant(), 1);
    assert!(site.failure_type().is_none());
    assert!(matches!(
        site.success_type().nodes()[site.success_type().root()].kind,
        ResolvedGenericTypeKind::Primitive(GenericPrimitive::Int)
    ));
    let result = site.return_type();
    let ResolvedGenericTypeKind::Declared { arguments, .. } = &result.nodes()[result.root()].kind
    else {
        panic!("expected return wrapper")
    };
    assert!(matches!(
        result.nodes()[arguments[0]].kind,
        ResolvedGenericTypeKind::Primitive(GenericPrimitive::String)
    ));
}
#[test]
fn result_error_payload_is_exact_while_success_payload_can_change() {
    let p=plan("fn test<T,U,E>(x:std::Result<T,E>,fallback:U)->std::Result<U,E>{let copy=x;return std::Result::<U,E>::Ok(fallback);}");
    let site = &p.plans()[0];
    assert_eq!(site.kind(), GenericStandardEnum::Result);
    assert!(site.failure_type().is_some());
    assert!(matches!(
        site.success_type().nodes()[site.success_type().root()].kind,
        ResolvedGenericTypeKind::Parameter(_)
    ));
}
#[test]
fn aliases_and_structured_errors_preserve_canonical_identity() {
    let p=plan("type Failure<T>=std::Result<T,std::Error>;fn test(x:Failure<Int>)->Failure<Bool>{let copy=x;return std::Result::<Bool,std::Error>::Ok(true);}");
    assert_eq!(p.plans()[0].kind(), GenericStandardEnum::Result);
}
#[test]
fn error_widening_and_distinct_symbolic_error_parameters_refuse() {
    for source in ["fn test(x:std::Result<Int,Int>)->std::Result<Int,Float>{let copy=x;return std::Result::<Int,Float>::Ok(1);}","fn test<A,B>(x:std::Result<Int,A>)->std::Result<Int,B>{let copy=x;return std::Result::<Int,B>::Ok(1);}"] {
        let b=bodies(source);let result=plan_generic_propagations(&b,&[request(&b,"test")],GenericPropagationLimits::DEFAULT);assert!(matches!(result,Err(GenericPropagationError::ErrorType {..})));
    }
}
#[test]
fn arrays_records_user_wrappers_and_wrapper_family_changes_refuse() {
    for source in ["enum Option<T>{Some(T),None}fn test(x:Option<Int>)->std::Option<Int>{let copy=x;return std::Option::<Int>::None();}","fn test(x:std::Error)->std::Option<Int>{let copy=x;return std::Option::<Int>::None();}","fn test(x:[std::Option<Int>])->std::Option<Int>{let copy=x;return std::Option::<Int>::None();}","fn test(x:std::Option<Int>)->std::Result<Int,String>{let copy=x;return std::Result::<Int,String>::Ok(1);}","fn test(x:std::Option<Int>){let copy=x;}"]{
        let b=bodies(source);assert!(plan_generic_propagations(&b,&[request(&b,"test")],GenericPropagationLimits::DEFAULT).is_err(),"{source}");
    }
}
#[test]
fn foreign_tables_and_same_signature_different_body_arenas_refuse() {
    let source = "fn test(x:std::Option<Int>)->std::Option<Int>{let copy=x;return x;}";
    let a = bodies(source);
    let b = bodies(source);
    assert!(matches!(
        plan_generic_propagations(
            &a,
            &[request(&b, "test")],
            GenericPropagationLimits::DEFAULT
        ),
        Err(GenericPropagationError::Ownership)
    ));
    let second = check_generic_copy_bodies(a.signatures(), GenericCopyBodyLimits::DEFAULT).unwrap();
    let r = request(&second, "test");
    assert!(matches!(
        plan_generic_propagations(&a, &[r], GenericPropagationLimits::DEFAULT),
        Err(GenericPropagationError::Ownership)
    ));
}
#[test]
fn duplicate_source_sites_refuse_and_empty_batch_needs_no_allowance() {
    let b = bodies("fn test(x:std::Option<Int>)->std::Option<Int>{let copy=x;return x;}");
    let r = request(&b, "test");
    assert!(matches!(
        plan_generic_propagations(&b, &[r, r], GenericPropagationLimits::DEFAULT),
        Err(GenericPropagationError::Duplicate)
    ));
    let zero = GenericPropagationLimits {
        max_sites: 0,
        max_contexts: 0,
        max_pairs: 0,
        max_edges: 0,
        max_work: 0,
    };
    assert_eq!(
        plan_generic_propagations(&b, &[], zero)
            .unwrap()
            .usage()
            .work,
        0
    );
    assert!(plan_generic_propagations(&b, &[r], zero).is_err());
}
#[test]
fn every_whole_batch_quota_has_an_exact_boundary() {
    let b=bodies("fn test(x:std::Result<Int,[String]>)->std::Result<Bool,[String]>{let copy=x;return std::Result::<Bool,[String]>::Ok(true);}fn other(x:std::Result<String,[String]>)->std::Result<Int,[String]>{let copy=x;return std::Result::<Int,[String]>::Ok(1);}");
    let requests = [request(&b, "test"), request(&b, "other")];
    let usage = plan_generic_propagations(&b, &requests, GenericPropagationLimits::DEFAULT)
        .unwrap()
        .usage();
    macro_rules! boundary {
        ($limit:ident,$cost:ident) => {{
            let mut l = GenericPropagationLimits::DEFAULT;
            l.$limit = usage.$cost;
            plan_generic_propagations(&b, &requests, l).unwrap();
            assert!(usage.$cost > 0);
            l.$limit -= 1;
            assert!(
                plan_generic_propagations(&b, &requests, l).is_err(),
                stringify!($limit)
            );
        }};
    }
    boundary!(max_sites, sites);
    boundary!(max_contexts, contexts);
    boundary!(max_pairs, pairs);
    boundary!(max_edges, edges);
    boundary!(max_work, work);
}
#[test]
fn owned_plans_retain_original_body_spans_and_contexts() {
    let p=plan("fn test(x:std::Option<String>)->std::Option<Int>{let copy=x;return std::Option::<Int>::None();}");
    let cloned = p.clone();
    drop(p);
    let site = &cloned.plans()[0];
    let expression = site.body().expression(site.operand()).unwrap();
    let source = site.body().signature().syntax().source();
    assert_eq!(&source[expression.span.start..expression.span.end], "x");
    assert_eq!(
        site.success_type()
            .environment()
            .registry()
            .header(site.declaration())
            .unwrap()
            .name
            .as_ref(),
        "std::Option"
    );
}
#[test]
fn postfix_syntax_is_still_refused_in_every_source_image_profile() {
    let source="fn test(x:std::Option<Int>)->std::Option<Int>{let value=x?;return std::Option::<Int>::Some(value);}fn main(){}";
    assert!(compile_generic_source(source, "main", GenericSourceLimits::DEFAULT).is_err());
    for features in [1u32, 3, 7] {
        let mut image = b"SVRG2\0".to_vec();
        image.extend_from_slice(&[1, 2]);
        image.extend_from_slice(&features.to_le_bytes());
        image.extend_from_slice(&(source.len() as u32).to_le_bytes());
        image.extend_from_slice(source.as_bytes());
        assert!(decode_generic_native_image(&image).is_err());
    }
}

#[test]
fn failed_batch_retains_the_correct_function_and_original_operand_range() {
    let b=bodies("fn first(x:std::Option<Int>)->std::Option<Bool>{let copy=x;return std::Option::<Bool>::None();}fn second(x:std::Result<Int,Int>)->std::Result<String,Float>{let copy=x;return std::Result::<String,Float>::Ok(\"ok\");}");
    let requests = [request(&b, "first"), request(&b, "second")];
    let error =
        plan_generic_propagations(&b, &requests, GenericPropagationLimits::DEFAULT).unwrap_err();
    let GenericPropagationError::ErrorType { function, span } = error else {
        panic!("expected exact error-type refusal")
    };
    assert_eq!(function, requests[1].function);
    assert_eq!(
        span,
        b.body(function)
            .unwrap()
            .expression(requests[1].operand)
            .unwrap()
            .span
    );
}
