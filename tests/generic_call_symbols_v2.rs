//! Source call facts and definition projection preserve identity, scope and bounds.
use sovra::compiler::v2::*;
fn compile(source: &str) -> CompiledGenericSource {
    compile_generic_array_source(source, "main", GenericPropagationSourceLimits::DEFAULT).unwrap()
}
#[test]
fn generic_plain_recursive_and_unused_calls_retain_only_original_identifier_ranges() {
    let source="// λ😀\r\nfn id<T>(x:T)->T{return x;}fn ordinary()->Int{return 1;}fn unused<T>(x:T)->T{return id::<T>(x);}fn recur<T>(x:T,n:Int)->T{if n==0{return x;}return recur::<T>(x,n-1);}fn main(){let text=\"id::<Int>(1)\";let a=id::<Int>(1);let b=id::<String>(\"x\");let c=(ordinary)();let d=recur::<Bool>(true,1);}";
    let p = compile(source);
    let bodies = p
        .ir()
        .plans()
        .layouts()
        .literals()
        .concrete()
        .graph()
        .bodies();
    let mut count = 0;
    for body in bodies.bodies() {
        for e in body.expressions() {
            if let GenericCopyExpressionKind::Call {
                signature,
                callee_span,
                ..
            } = &e.kind
            {
                let callee = bodies
                    .signatures()
                    .signature(signature.binding().callee())
                    .unwrap();
                assert_eq!(
                    &body.signature().syntax().source()[callee_span.start..callee_span.end],
                    callee.syntax().name()
                );
                assert!(callee_span.start >= e.span.start && callee_span.end <= e.span.end);
                count += 1;
            }
        }
    }
    assert_eq!(count, 6);
    let symbols = generic_source_symbols(&p, GenericSymbolLimits::DEFAULT);
    assert_eq!(
        symbols
            .split("\"references\":[")
            .nth(1)
            .unwrap()
            .split("],\"locals\":")
            .next()
            .unwrap()
            .matches("\"definition\":")
            .count(),
        6
    );
    assert!(symbols.contains("\"truncated\":false"));
}
#[test]
fn reference_quota_exhaustion_discards_the_complete_optional_batch() {
    let p = compile("fn target()->Int{return 1;}fn main(){target();target();}");
    let d = GenericSymbolLimits::DEFAULT;
    let limited = generic_source_symbols(
        &p,
        GenericSymbolLimits {
            max_references: 1,
            ..d
        },
    );
    assert!(limited.contains("\"unavailable\":true"));
    assert!(limited.contains("\"declarations\":[]"));
    assert!(!limited.contains("\"definition\":"));
    assert!(generic_source_symbols(
        &p,
        GenericSymbolLimits {
            max_references: 2,
            ..d
        }
    )
    .contains("\"truncated\":false"));
}
#[test]
fn comments_strings_and_shadowed_values_never_create_call_references() {
    let p=compile("fn helper<T>(x:T)->T{return x;}fn main(){let helper=1;let text=\"helper::<Int>(7)\";let value=helper;}");
    let symbols = generic_source_symbols(&p, GenericSymbolLimits::DEFAULT);
    assert!(symbols.contains("\"references\":[]"));
    assert!(compile_generic_array_source(
        "fn helper<T>(x:T)->T{return x;}fn main(){let helper=1;helper::<Int>(7);}",
        "main",
        GenericPropagationSourceLimits::DEFAULT
    )
    .is_err());
}
#[test]
fn call_references_are_source_sites_not_specialization_duplicates() {
    let p=compile("fn leaf<T>(x:T)->T{return x;}fn outer<T>(x:T)->T{return leaf::<T>(x);}fn main(){let a=outer::<Int>(1);let b=outer::<String>(\"s\");}");
    assert_eq!(p.ir().functions().len(), 5);
    let symbols = generic_source_symbols(&p, GenericSymbolLimits::DEFAULT);
    assert_eq!(
        symbols
            .split("\"references\":[")
            .nth(1)
            .unwrap()
            .split("],\"locals\":")
            .next()
            .unwrap()
            .matches("\"definition\":")
            .count(),
        3
    );
}

#[test]
fn function_identity_is_not_confused_with_a_type_of_the_same_name() {
    let p=compile("struct same<T>{value:T}fn same<T>(x:T)->T{return x;}fn main(){let boxed=same::<Int>{value:2};let value=same::<Int>(1);}");
    let symbols = generic_source_symbols(&p, GenericSymbolLimits::DEFAULT);
    assert_eq!(
        symbols
            .split("\"references\":[")
            .nth(1)
            .unwrap()
            .split("],\"locals\":")
            .next()
            .unwrap()
            .matches("\"definition\":")
            .count(),
        1
    );
    assert!(symbols.contains("\"definition\":1"));
}
