//! Nominal constructor and match identities, without name-based target selection.
use sovra::compiler::v2::*;
fn check(source: &str) -> CheckedProgramSource {
    check_program_source("fixture", source, ProgramSourceLimits::DEFAULT).unwrap()
}
#[test]
fn alias_annotated_values_and_reordered_match_arms_target_canonical_variants() {
    let source="enum E{A(Int),End}type Alias=E;fn main(){let e:Alias=Alias::A(7);match e{E::End()=>{},Alias::A(n)=>{print(n);}}}";
    let p = check(source);
    let out = p.render_symbols(ProgramSymbolLimits::DEFAULT);
    assert!(!out.contains("\"unavailable\":true"), "{out}");
    assert!(out.contains("\"name\":\"E\",\"signature\":\"enum E\",\"kind\":10"));
    assert!(out.contains("\"name\":\"E::A\",\"signature\":\"E::A(Int) -> E\",\"kind\":22"));
    let variant = source.find("A(Int)").unwrap();
    assert!(out.contains(&format!(
        "\"name_start\":{variant},\"name_end\":{}",
        variant + 1
    )));
    for (spelling, definition, offset) in [
        ("Alias::A(7)", 2, 7),
        ("Alias::A(n)", 2, 7),
        ("E::End()", 3, 3),
    ] {
        let start = source.find(spelling).unwrap() + offset;
        let length = if definition == 3 { 3 } else { 1 };
        assert!(
            out.contains(&format!(
                "{{\"start\":{start},\"end\":{},\"definition\":{definition}}}",
                start + length
            )),
            "{out}"
        );
    }
    assert!(p
        .render_function_symbols(ProgramSymbolLimits::DEFAULT)
        .contains("\"references\":[]"));
}
#[test]
fn same_named_variants_recursive_controls_and_nested_matches_keep_nominal_targets() {
    let source="enum E{A(Int)}enum F{A(Int)}fn main(){let e=E::A(1);let f=F::A(2);if true{match e{E::A(n)=>{while false{match f{F::A(n)=>{print(n);}}}}}}}";
    let p = check(source);
    let out = p.render_symbols(ProgramSymbolLimits::DEFAULT);
    assert!(!out.contains("\"unavailable\":true"), "{out}");
    assert_eq!(out.matches("\"kind\":22").count(), 2);
    for (spelling, definition) in [
        ("E::A(1)", 2),
        ("F::A(2)", 4),
        ("E::A(n)", 2),
        ("F::A(n)", 4),
    ] {
        let start = source.find(spelling).unwrap() + 3;
        assert!(
            out.contains(&format!(
                "{{\"start\":{start},\"end\":{},\"definition\":{definition}}}",
                start + 1
            )),
            "{out}"
        );
    }
}
#[test]
fn nullary_unit_record_array_and_recursive_enum_payload_signatures_are_bounded() {
    let p = check("struct R{x:Int}enum E{End,UnitValue(Unit),Items([R]),Next(E)}fn main(){}");
    let out = p.render_symbols(ProgramSymbolLimits::DEFAULT);
    for signature in [
        "E::End() -> E",
        "E::UnitValue(Unit) -> E",
        "E::Items([R]) -> E",
        "E::Next(E) -> E",
    ] {
        assert!(out.contains(signature), "{out}");
    }
}
#[test]
fn enum_declarations_and_constructor_pattern_references_share_all_table_quotas() {
    let p = check("enum E{End}fn main(){let e=E::End();match e{E::End()=>{}}}");
    let out = p.render_symbols(ProgramSymbolLimits::DEFAULT);
    // function + enum/variant + two variant refs + local declaration/scrutinee use.
    let exact = ProgramSymbolLimits {
        max_items: 7,
        max_bytes: out.len(),
    };
    assert_eq!(p.render_symbols(exact), out);
    for cap in [
        ProgramSymbolLimits {
            max_items: 6,
            ..exact
        },
        ProgramSymbolLimits {
            max_bytes: out.len() - 1,
            ..exact
        },
    ] {
        let bad = p.render_symbols(cap);
        assert!(bad.contains("\"unavailable\":true"));
        assert!(!bad.contains("E::End"));
    }
}

#[test]
fn sibling_branches_and_later_matches_pair_with_checked_preorder_plans() {
    let source = "enum E{A(Int)}fn main(){let e=E::A(1);if true{match e{E::A(x)=>{print(x);}}}else{match e{E::A(y)=>{print(y);}}}match e{E::A(z)=>{print(z);}}}";
    let out = check(source).render_symbols(ProgramSymbolLimits::DEFAULT);
    assert!(!out.contains("\"unavailable\":true"), "{out}");
    for pattern in ["E::A(1)", "E::A(x)", "E::A(y)", "E::A(z)"] {
        let start = source.find(pattern).unwrap() + 3;
        assert!(
            out.contains(&format!(
                "{{\"start\":{start},\"end\":{},\"definition\":2}}",
                start + 1
            )),
            "{out}"
        );
    }
}

#[test]
fn checked_module_containers_preserve_variant_targets_and_share_projection_limits() {
    let source = "mod data{enum E{End}export fn value()->E{return E::End();}}mod empty{}fn main(){let e=data::value();match e{data::E::End()=>{}}}";
    let program = check(source);
    let out = program.render_symbols(ProgramSymbolLimits::DEFAULT);
    assert!(!out.contains("\"unavailable\":true"), "{out}");
    for name in ["data", "empty"] {
        assert!(
            out.contains(&format!(
                "\"name\":\"{name}\",\"signature\":\"mod {name}\",\"kind\":2"
            )),
            "{out}"
        );
        let start = source.find(&format!("mod {name}")).unwrap() + 4;
        assert!(out.contains(&format!(
            "\"name_start\":{start},\"name_end\":{}",
            start + name.len()
        )));
    }
    // Two functions, enum, variant, two containers; four references/bindings.
    let items = out.matches("\"name_start\":").count() + out.matches("\"definition\":").count();
    let exact = ProgramSymbolLimits {
        max_items: items,
        max_bytes: out.len(),
    };
    assert_eq!(program.render_symbols(exact), out);
    assert!(program
        .render_symbols(ProgramSymbolLimits {
            max_items: items - 1,
            ..exact
        })
        .contains("\"unavailable\":true"));
    assert!(program
        .render_symbols(ProgramSymbolLimits {
            max_bytes: out.len() - 1,
            ..exact
        })
        .contains("\"unavailable\":true"));
    assert!(!program
        .render_function_symbols(ProgramSymbolLimits::DEFAULT)
        .contains("\"kind\":2"));
    assert_eq!(out.matches("\"definition\":3}").count(), 2, "{out}");
    let variant = out.find("\"name\":\"data::E::End\"").unwrap();
    assert!(
        variant
            < out
                .find("\"name\":\"data\",\"signature\":\"mod data\"")
                .unwrap()
    );
}

#[test]
fn records_and_aliases_keep_module_ownership_original_headers_and_shared_limits() {
    let source = "// λ😀\nstruct R{x:Int}type A=R;mod m{struct R{x:Int}type A = R;}mod empty{}struct Tail{x:Int}fn main(){}";
    let program = check(source);
    let out = program.render_symbols(ProgramSymbolLimits::DEFAULT);
    assert!(!out.contains("\"unavailable\":true"), "{out}");
    for (name, signature, kind) in [
        ("R", "struct R", 23),
        ("A", "type A=R;", 26),
        ("m::R", "struct m::R", 23),
        ("m::A", "type A = R;", 26),
        ("Tail", "struct Tail", 23),
    ] {
        assert!(
            out.contains(&format!(
                "\"name\":\"{name}\",\"signature\":\"{signature}\",\"kind\":{kind}"
            )),
            "{out}"
        );
    }
    let start = source.rfind("type A = R;").unwrap();
    assert!(
        out.contains(&format!(
            "\"start\":{start},\"end\":{},\"name_start\":{},\"name_end\":{}",
            start + 11,
            start + 5,
            start + 6
        )),
        "{out}"
    );
    let exact = ProgramSymbolLimits {
        max_items: 8,
        max_bytes: out.len(),
    };
    assert_eq!(program.render_symbols(exact), out);
    for limits in [
        ProgramSymbolLimits {
            max_items: 7,
            ..exact
        },
        ProgramSymbolLimits {
            max_bytes: out.len() - 1,
            ..exact
        },
    ] {
        assert!(program
            .render_symbols(limits)
            .contains("\"unavailable\":true"));
    }
    assert!(!program
        .render_function_symbols(ProgramSymbolLimits::DEFAULT)
        .contains("\"kind\":23"));
}

#[test]
fn oversized_written_alias_header_refuses_optional_facts_without_undoing_checking() {
    let source = format!("type A{}=Int;fn main(){{}}", " ".repeat(4100));
    let program = check(&source);
    assert!(program
        .render_symbols(ProgramSymbolLimits::DEFAULT)
        .contains("\"unavailable\":true"));
    assert!(!program
        .render_function_symbols(ProgramSymbolLimits::DEFAULT)
        .contains("\"unavailable\":true"));
}

#[test]
fn explicit_enum_header_annotations_target_checked_scope_not_same_written_names() {
    let source = "mod a{enum E{End}export fn pass(x:[E])->[E]{return x;}}mod b{enum E{End}export fn pass(x:a::E)->a::E{return x;}}fn main(){}";
    let out = check(source).render_symbols(ProgramSymbolLimits::DEFAULT);
    assert!(!out.contains("\"unavailable\":true"), "{out}");
    // Three functions precede a::E, its variant, b::E and its variant.
    for fragment in ["x:[E]", "->[E]", "x:a::E", "->a::E"] {
        let start = source.find(fragment).unwrap() + fragment.len()
            - if fragment.ends_with(']') { 2 } else { 1 };
        assert!(
            out.contains(&format!(
                "{{\"start\":{start},\"end\":{},\"definition\":3}}",
                start + 1
            )),
            "{out}"
        );
    }
    let items = out.matches("\"name_start\":").count() + out.matches("\"definition\":").count();
    let exact = ProgramSymbolLimits {
        max_items: items,
        max_bytes: out.len(),
    };
    let program = check(source);
    assert_eq!(program.render_symbols(exact), out);
    assert!(program
        .render_symbols(ProgramSymbolLimits {
            max_items: items - 1,
            ..exact
        })
        .contains("\"unavailable\":true"));
}

#[test]
fn alias_scalar_and_record_header_annotations_do_not_invent_enum_definition_targets() {
    let out = check("enum E{End}type Alias=E;struct R{x:Int}fn a(x:Alias)->Alias{return x;}fn b(x:R)->R{return x;}fn main(){}").render_symbols(ProgramSymbolLimits::DEFAULT);
    assert!(!out.contains("\"unavailable\":true"), "{out}");
    // Parameter reads remain local references; no type-use references are emitted.
    assert!(out.contains("\"references\":[]"), "{out}");
}
