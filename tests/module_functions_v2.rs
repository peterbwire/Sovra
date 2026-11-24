//! Qualified module calls, closed scope types and source visibility.
use sovra::compiler::v2::*;
fn check(source: &str) -> CheckedProgramSource {
    check_program_source("modules", source, ProgramSourceLimits::DEFAULT).unwrap()
}
fn output(source: &str) -> Vec<String> {
    let p = check(source);
    p.execute(
        p.function_id("main").unwrap(),
        &[],
        ProgramLoweringLimits::DEFAULT,
        command_runtime_policy(),
    )
    .unwrap()
    .output
}
#[test]
fn exported_functions_use_local_types_aliases_records_and_private_qualified_helpers() {
    let source="mod m{struct R{x:Int}enum E{V(R),End}type Alias=E;fn hidden(n:Int)->Alias{return Alias::V(R{x:n});}export fn obtain(n:Int)->Alias{return m::hidden(n);}}fn main(){let e=m::obtain(7);match e{m::E::V(r)=>{print(r.x);},m::E::End()=>{}}}";
    assert_eq!(output(source), ["7"]);
}
#[test]
fn private_calls_are_visible_only_to_the_owning_module() {
    for source in [
        "mod m{fn hidden(){}}fn main(){m::hidden();}",
        "mod a{fn hidden(){}}mod b{export fn call(){a::hidden();}}fn main(){b::call();}",
    ] {
        assert!(check_program_source("private", source, ProgramSourceLimits::DEFAULT).is_err());
    }
    assert_eq!(
        output("mod m{fn hidden(){print(7);}export fn call(){m::hidden();}}fn main(){m::call();}"),
        ["7"]
    );
}
#[test]
fn recursion_same_named_members_and_module_main_keep_canonical_identities() {
    let source="mod a{export fn value()->Int{return 1;}export fn main(n:Int)->Int{if n==0{return a::value();}return a::main(n-1);}}mod b{export fn value()->Int{return 2;}}fn main(){print(a::main(2));print(b::value());}";
    let p = check(source);
    assert_eq!(
        p.function_name(p.function_id("a::value").unwrap()),
        Some("a::value")
    );
    assert!(p.function_id("value").is_none());
    assert_eq!(output(source), ["1", "2"]);
}
#[test]
fn same_named_module_types_do_not_retype_other_function_signatures() {
    let source="mod a{enum E{V(Int)}export fn get()->E{return E::V(7);}}mod b{enum E{V(Int)}export fn forward()->a::E{return a::get();}}fn main(){let e=b::forward();match e{a::E::V(n)=>{print(n);}}}";
    assert_eq!(output(source), ["7"]);
    assert!(check_program_source(
        "bad",
        "enum E{V(Int)}mod m{enum E{V(Int)}export fn get()->E{return E::V(7);}}fn main(){}",
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
}
#[test]
fn unused_body_errors_and_duplicate_members_refuse_the_whole_program() {
    for source in [
        "mod m{fn unused(){missing();}}fn main(){}",
        "mod m{fn f(){}fn f(){}}fn main(){}",
        "mod std{export fn print(x:Int){}}fn main(){}",
    ] {
        assert!(check_program_source("bad", source, ProgramSourceLimits::DEFAULT).is_err());
    }
}

#[test]
fn qualified_symbol_targets_and_ir_names_retain_source_owned_module_identities() {
    let source="mod m{fn hidden()->Int{return 7;}export fn value()->Int{return m::hidden();}}fn main(){print(m::value());}";
    let p = check(source);
    let symbols = p.render_symbols(ProgramSymbolLimits::DEFAULT);
    assert!(!symbols.contains("\"unavailable\":true"), "{symbols}");
    assert!(symbols.contains("\"name\":\"m::hidden\""));
    assert!(symbols.contains("\"name\":\"m::value\""));
    let ir = p
        .render_ir(ProgramLoweringLimits::DEFAULT, ProgramIrLimits::DEFAULT)
        .unwrap();
    assert!(ir.contains("\"m::hidden\""));
    assert!(ir.contains("\"m::value\""));
}
#[test]
fn scope_copies_canonical_headers_and_visible_signatures_share_exact_program_budgets() {
    let source="mod m{enum E{End}fn hidden()->E{return E::End();}export fn value()->E{return m::hidden();}}fn main(){let e=m::value();}";
    let p = check(source);
    let mut limits = ProgramSourceLimits::DEFAULT;
    limits.max_evidence_items = p.evidence_items();
    limits.max_signature_text_bytes = p.signature_text_bytes();
    limits.max_signature_copies = p.signature_copies();
    assert!(check_program_source("exact", source, limits).is_ok());
    limits.max_evidence_items -= 1;
    assert!(check_program_source("small", source, limits).is_err());
    limits.max_evidence_items += 1;
    limits.max_signature_text_bytes -= 1;
    assert!(check_program_source("small", source, limits).is_err());
}
#[test]
fn root_entry_and_root_test_discovery_never_select_module_homonyms() {
    let p = check(
        "mod m{export fn main(n:Int)->Int{return n;}fn test_private(n:Int){}}fn test_root(){}",
    );
    assert!(p.function_id("main").is_none());
    assert!(p.encode_native_image().is_err());
    assert_eq!(
        p.functions()
            .iter()
            .filter(|f| f
                .typed()
                .callable()
                .function()
                .environment()
                .module()
                .is_none())
            .count(),
        1
    );
}

#[test]
fn unqualified_calls_preserve_root_lookup_and_module_members_require_qualification() {
    assert_eq!(output("mod m{fn hidden()->Int{return 1;}export fn value()->Int{return hidden();}}fn hidden()->Int{return 2;}fn main(){print(m::value());}"),["2"]);
    assert!(check_program_source(
        "bad",
        "mod m{fn hidden(){}export fn value(){hidden();}}fn main(){}",
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
    assert_eq!(
        output("mod m{export fn print(){print(7);}}fn main(){m::print();}"),
        ["7"]
    );
}

#[test]
fn constructor_aliases_never_silently_replace_declared_module_call_targets() {
    let source="enum Q{f(Int)}type m=Q;mod m{export fn f(n:Int)->Q{print(99);return Q::f(n);}}fn main(){let q=m::f(7);}";
    let error = check_program_source("collision", source, ProgramSourceLimits::DEFAULT)
        .unwrap_err()
        .into_diagnostics();
    assert!(error.items.iter().any(|d| d.code == "E3110"), "{error:?}");
}
