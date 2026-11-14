//! Transparent aliases retain canonical enum IDs through checking/lowering/execution.
use sovra::compiler::{parser::Parser, v2::*};
fn check(s: &str) -> CheckedProgramSource {
    check_program_source("aliases", s, ProgramSourceLimits::DEFAULT).unwrap()
}
#[test]
fn alias_chains_construct_match_and_compare_as_the_same_nominal_enum() {
    let source="enum E{Value(Float),End}type Alias=E;type Chain=Alias;fn obtain(n:Int)->Chain{if n==0{return Chain::Value(7);}return obtain(n-1);}fn main(){let e=obtain(2);print(e==Alias::Value(7.0));match e{Chain::End()=>{print(0);},Alias::Value(n)=>{print(n);}}print(Chain::End()==E::End());}";
    let p = check(source);
    let result = p
        .execute(
            p.function_id("main").unwrap(),
            &[],
            ProgramLoweringLimits::DEFAULT,
            command_runtime_policy(),
        )
        .unwrap();
    assert_eq!(result.output, ["true", "7", "true"]);
    let metadata = p.render_symbols(ProgramSymbolLimits::DEFAULT);
    assert!(!metadata.contains("\"unavailable\":true"), "{metadata}");
    assert!(metadata.contains("E::Value(Float) -> E"));
    assert!(!metadata.contains("\"name\":\"Alias::Value\""));
}
#[test]
fn closed_resolution_preserves_exact_find_and_excludes_non_enum_aliases() {
    let context=Parser::new().parse_source("type Alias=E;type Chain=Alias;type Items=[E];type Number=Int;struct R{x:Int}type Record=R;").unwrap();
    let env = check_enum_types(
        "owner",
        "context",
        &context,
        &[EnumSource {
            label: "enum",
            source: "enum E{End}",
        }],
        EnumTypeLimits::DEFAULT,
    )
    .unwrap();
    let id = env.find("E").unwrap();
    assert_eq!(env.resolve_enum("Alias"), Some(id));
    assert_eq!(env.resolve_enum("Chain"), Some(id));
    assert_eq!(env.find("Alias"), None);
    for name in ["Items", "Number", "Record", "missing"] {
        assert_eq!(env.resolve_enum(name), None);
    }
}
#[test]
fn aliases_never_mix_foreign_enums_or_bypass_arity_and_payload_checks() {
    for body in [
        "let e: E=B::Value(1);",
        "let e=A::Value(true);",
        "let e=A::Value();",
        "let e=A::End(1);",
        "let e=A::End();match e{B::End()=>{},B::Value(n)=>{}}",
    ] {
        let source = format!(
            "enum E{{Value(Int),End}}enum F{{Value(Int),End}}type A=E;type B=F;fn main(){{{body}}}"
        );
        assert!(
            check_program_source("bad", &source, ProgramSourceLimits::DEFAULT).is_err(),
            "{body}"
        );
    }
    for alias in ["[E]", "Int", "R"] {
        let s =
            format!("struct R{{x:Int}}enum E{{End}}type A={alias};fn main(){{let e=A::End();}}");
        assert!(check_program_source("bad", &s, ProgramSourceLimits::DEFAULT).is_err());
    }
}
#[test]
fn generated_alias_signatures_charge_existing_exact_and_exceeded_allowances() {
    let source = "enum E{Value(Int),End}type A=E;type B=A;fn main(){let e=B::Value(1);}";
    // One source header plus three spellings of two constructors.
    let mut limits = ProgramSourceLimits::DEFAULT;
    limits.source.callable.typed.callable.max_signatures = 7;
    assert!(check_program_source("limits", source, limits).is_ok());
    limits.source.callable.typed.callable.max_signatures = 6;
    assert!(check_program_source("limits", source, limits).is_err());
    let p = check(source);
    let used = p.signature_copies();
    let mut limits = ProgramSourceLimits::DEFAULT;
    limits.max_signature_copies = used;
    assert!(check_program_source("limits", source, limits).is_ok());
    limits.max_signature_copies = used - 1;
    assert!(check_program_source("limits", source, limits).is_err());
}
#[test]
fn alias_names_cannot_shadow_standard_callable_identities() {
    let error = check_program_source(
        "collision",
        "enum E{print(String)}type std=E;fn main(){}",
        ProgramSourceLimits::DEFAULT,
    )
    .unwrap_err()
    .into_diagnostics();
    assert!(error.items.iter().any(|d| d.code == "E3110"));
}

#[test]
fn alias_array_payload_widening_keeps_the_original_nested_snapshot() {
    let p=check("enum E{Rows([[Float]])}type Alias=E;fn main(){let ints=[[1]];let e=Alias::Rows(ints);match e{Alias::Rows(rows)=>{print(rows[0][0]/2);}}print(ints[0][0]);}");
    let result = p
        .execute(
            p.function_id("main").unwrap(),
            &[],
            ProgramLoweringLimits::DEFAULT,
            command_runtime_policy(),
        )
        .unwrap();
    assert_eq!(result.output, ["0.5", "1"]);
}
