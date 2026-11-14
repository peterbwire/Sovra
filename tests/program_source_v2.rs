//! Source-owned cross-function checking/links; linked VM execution is deliberately pending.
use sovra::compiler::v2::*;
fn check(source: &str) -> CheckedProgramSource {
    check_program_source("program.svr", source, ProgramSourceLimits::DEFAULT).unwrap()
}
fn lower(function: &CheckedScopedCallableFunction) {
    let map = function.map_for_lowering(MappingLimits::DEFAULT).unwrap();
    let expressions = map.emit_expression_ir(ExpressionIrLimits::DEFAULT).unwrap();
    let ir = expressions
        .emit_function_ir(FunctionIrLimits::DEFAULT)
        .unwrap();
    ir.validate(FunctionIrLimits::DEFAULT).unwrap();
}
#[test]
fn forward_nominal_calls_link_source_owned_function_ids_and_lower_without_name_lookup() {
    let source="enum E{V(Int),End} fn main(){let v=make(7);match v{E::V(n)=>{print(n);},E::End()=>{}}} fn make(n:Int)->E{return E::V(n);}";
    let program = check(source);
    assert_eq!(program.source(), source);
    assert_eq!(program.source_label(), "program.svr");
    let main = program.function_id("main").unwrap();
    let make = program.function_id("make").unwrap();
    assert_eq!(
        program.call_links(),
        &[ProgramCallLink {
            caller: main,
            callee: make,
            span: program.call_links()[0].span
        }]
    );
    assert_eq!(
        &source[program.call_links()[0].span.start..program.call_links()[0].span.end],
        "make"
    );
    assert_eq!(program.functions().len(), 2);
    assert_eq!(program.declarations().len(), 3);
    for function in program.functions() {
        lower(function);
    }
    assert_eq!(
        program
            .function(make)
            .unwrap()
            .typed()
            .callable()
            .function()
            .syntax()
            .name,
        "make"
    );
}
#[test]
fn mutual_and_self_recursion_have_canonical_program_targets() {
    let program=check("fn even(n:Int)->Bool{if n==0{return true;}else{return odd(n-1);}} fn odd(n:Int)->Bool{if n==0{return false;}else{return even(n-1);}} fn self_call(n:Int)->Int{if n>0{return self_call(n-1);}else{return 0;}}");
    let even = program.function_id("even").unwrap();
    let odd = program.function_id("odd").unwrap();
    let own = program.function_id("self_call").unwrap();
    assert!(program
        .call_links()
        .iter()
        .any(|l| l.caller == even && l.callee == odd));
    assert!(program
        .call_links()
        .iter()
        .any(|l| l.caller == odd && l.callee == even));
    assert!(program
        .call_links()
        .iter()
        .any(|l| l.caller == own && l.callee == own));
    assert_eq!(program.call_links().len(), 3);
    for function in program.functions() {
        lower(function);
    }
}
#[test]
fn numeric_alias_and_array_parameter_contexts_survive_cross_calls() {
    let program=check("type Number=Float;fn f()->Number{return other(7,[1,2]);} fn other(n:Number,a:[Float])->Float{return n+a[0];}");
    let function = &program.functions()[0];
    assert!(function
        .typed()
        .callable()
        .evidence()
        .contexts()
        .iter()
        .any(|c| c.actual == sovra::compiler::semantic::Type::Int
            && c.expected == sovra::compiler::semantic::Type::Float));
    for function in program.functions() {
        lower(function);
    }
}
#[test]
fn all_bodies_signatures_and_calls_are_checked_including_unused_functions() {
    for source in [
        "fn main(){}fn unused()->Int{return true;}",
        "fn f(){missing();}fn g(){}",
        "fn f(){g(true);}fn g(n:Int){}",
        "fn f(){g();}fn g(n:Int){}",
        "fn f(){}fn f(){}",
        "fn f(){}fn g(n){}",
        "fn f(){}fn g()->Unknown{return 7;}",
        "fn f(){}fn print(){}",
        "enum A{V(Int)}enum B{V(Int)}fn f(){take(A::V(1));}fn take(value:B){}",
    ] {
        assert!(
            check_program_source("s", source, ProgramSourceLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
}
#[test]
fn identities_refuse_same_ordinal_from_another_program() {
    let first = check("fn f(){}");
    let second = check("fn f(){}");
    let a = first.function_id("f").unwrap();
    let b = second.function_id("f").unwrap();
    assert_eq!(a.ordinal(), b.ordinal());
    assert_ne!(a, b);
    assert!(first.function(a).is_some());
    assert!(second.function(a).is_none());
    assert!(first.function_id("missing").is_none());
}
#[test]
fn cumulative_signature_text_fact_and_link_caps_are_exact() {
    let source = "fn a()->Int{return b();}fn b()->Int{return 7;}";
    let program = check(source);
    assert_eq!(program.signature_copies(), 6);
    assert_eq!(program.signature_text_bytes(), 32);
    let exact = ProgramSourceLimits {
        max_signature_copies: program.signature_copies(),
        max_signature_text_bytes: program.signature_text_bytes(),
        max_evidence_items: program.evidence_items(),
        max_call_links: 1,
        ..ProgramSourceLimits::DEFAULT
    };
    assert!(check_program_source("s", source, exact).is_ok());
    for limits in [
        ProgramSourceLimits {
            max_signature_copies: 5,
            ..exact
        },
        ProgramSourceLimits {
            max_signature_text_bytes: 31,
            ..exact
        },
        ProgramSourceLimits {
            max_evidence_items: program.evidence_items() - 1,
            ..exact
        },
        ProgramSourceLimits {
            max_call_links: 0,
            ..exact
        },
        ProgramSourceLimits {
            max_functions: 1,
            ..exact
        },
    ] {
        assert!(check_program_source("s", source, limits).is_err());
    }
}
#[test]
fn constructors_are_charged_in_each_signature_table_without_becoming_program_links() {
    let program = check("enum E{A,B(Int)}fn a()->E{return b();}fn b()->E{return E::B(7);}");
    assert_eq!(program.signature_copies(), 10);
    assert_eq!(program.call_links().len(), 1);
    assert_eq!(
        program.functions()[1].typed().callable().evidence().calls()[0].target,
        CallableTarget::Constructor {
            enum_id: program.functions()[1]
                .typed()
                .callable()
                .function()
                .environment()
                .find("E")
                .unwrap(),
            variant: 1
        }
    );
}
#[test]
fn standalone_vm_refuses_program_calls_before_any_effect_or_materialization() {
    let program = check("fn main(){print(7);if false{helper();}}fn helper(){}");
    let map = program.functions()[0]
        .map_for_lowering(MappingLimits::DEFAULT)
        .unwrap();
    let expressions = map.emit_expression_ir(ExpressionIrLimits::DEFAULT).unwrap();
    let ir = expressions
        .emit_function_ir(FunctionIrLimits::DEFAULT)
        .unwrap();
    let policy = RuntimeVmPolicy {
        runtime: sovra::compiler::execution::RuntimePolicy {
            materialization: sovra::compiler::execution::MaterializationLimits {
                max_value_nodes: Some(0),
                max_text_bytes: Some(0),
            },
            ..sovra::compiler::execution::RuntimePolicy::UNLIMITED
        },
        ..RuntimeVmPolicy::DEFAULT
    };
    assert_eq!(
        ir.execute(&[], policy).unwrap_err(),
        RuntimeVmError::Invalid("program call requires linked execution")
    );
}
#[test]
fn prior_single_entry_api_stays_strict_and_program_scope_still_refuses_modules_imports() {
    assert!(check_entry_source("s", "fn a(){}fn b(){}", EntrySourceLimits::DEFAULT).is_err());
    for source in [
        "module m{fn f(){}}",
        "use x::m;fn f(){}",
        "enum E{A}",
        "fn f(){",
    ] {
        assert!(check_program_source("s", source, ProgramSourceLimits::DEFAULT).is_err());
    }
}
#[test]
fn original_unicode_crlf_call_spans_are_retained() {
    let source = "// 😀\r\nfn a(){print(\"λ😀\");b();}\r\nfn b(){}";
    let program = check(source);
    let link = program.call_links()[0];
    assert_eq!(&source[link.span.start..link.span.end], "b");
    assert_eq!(link.span.line, 1);
    assert_eq!(link.span.column, "fn a(){print(\"λ😀\");".chars().count());
}
