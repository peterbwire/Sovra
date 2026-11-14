//! Same-file type modules preserve nominal qualification and reject executable bodies.
use sovra::compiler::v2::*;
fn check(s: &str) -> CheckedProgramSource {
    check_program_source("module.svr", s, ProgramSourceLimits::DEFAULT).unwrap()
}
#[test]
fn module_local_aliases_records_and_recursive_enums_execute_from_root_functions() {
    let s="mod types{struct Item{value:Int}enum E{Value(Item),Next(E),End}type Alias=E;}fn make()->types::Alias{return types::Alias::Value(types::Item{value:7});}fn main(){let e=make();match e{types::E::End()=>{},types::Alias::Next(next)=>{print(next);},types::Alias::Value(item)=>{print(item.value);}}}";
    let p = check(s);
    let out = p
        .execute(
            p.function_id("main").unwrap(),
            &[],
            ProgramLoweringLimits::DEFAULT,
            command_runtime_policy(),
        )
        .unwrap();
    assert_eq!(out.output, ["7"]);
    let metadata = p.render_symbols(ProgramSymbolLimits::DEFAULT);
    assert!(!metadata.contains("\"unavailable\":true"), "{metadata}");
    assert!(metadata.contains("types::E::Value(types::Item) -> types::E"));
}
#[test]
fn root_and_module_homonyms_and_two_module_homonyms_remain_nominally_distinct() {
    let s="enum E{V(Int)}mod a{enum E{V(Int)}}mod b{enum E{V(Int)}}fn main(){let e=a::E::V(1);match e{a::E::V(n)=>{print(n);}}}";
    let p = check(s);
    assert!(p
        .render_symbols(ProgramSymbolLimits::DEFAULT)
        .contains("a::E::V(Int) -> a::E"));
    for body in [
        "let e:a::E=b::E::V(1);",
        "let e:a::E=E::V(1);",
        "let e=a::E::V(1);match e{b::E::V(n)=>{}}",
    ] {
        let source = format!(
            "enum E{{V(Int)}}mod a{{enum E{{V(Int)}}}}mod b{{enum E{{V(Int)}}}}fn main(){{{body}}}"
        );
        assert!(
            check_program_source("bad", &source, ProgramSourceLimits::DEFAULT).is_err(),
            "{body}"
        );
    }
}
#[test]
fn syntax_projection_retains_original_global_spans_and_module_paths() {
    let source = "// λ😀\r\nmod models{enum E{End}type Alias=E;}enum Root{End}fn main(){}";
    let p = parse_program_syntax(source, EntrySourceLimits::DEFAULT, 128).unwrap();
    assert_eq!(p.enum_modules().collect::<Vec<_>>(), [Some("models"), None]);
    for e in p.enums() {
        assert_eq!(&source[e.name_span.start..e.name_span.end], e.name);
    }
    let module = p
        .declarations()
        .iter()
        .find(|d| d.kind == EntryDeclarationKind::Module)
        .unwrap();
    assert_eq!(
        &source[module.span.start..module.span.end],
        "mod models{enum E{End}type Alias=E;}"
    );
    assert_eq!(p.core_declarations().modules[0].name, "models");
}
#[test]
fn unsupported_module_functions_nested_modules_imports_and_bad_syntax_never_pass() {
    for source in [
        "mod m{mod n{}}fn main(){}",
        "export mod m{}fn main(){}",
        "mod m{use pkg::api;}fn main(){}",
        "mod m{enum E{End}",
        "mod {}fn main(){}",
        "mod m enum E{End}fn main(){}",
    ] {
        assert!(
            parse_program_syntax(source, EntrySourceLimits::DEFAULT, 128).is_err(),
            "{source}"
        );
        assert!(
            check_program_source("bad", source, ProgramSourceLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
}
#[test]
fn module_containers_and_children_share_existing_declaration_admission() {
    let source = "mod m{enum E{End}type A=E;}fn main(){}";
    let mut limits = EntrySourceLimits::DEFAULT;
    limits.max_declarations = 4;
    assert!(parse_program_syntax(source, limits, 128).is_ok());
    limits.max_declarations = 3;
    assert!(parse_program_syntax(source, limits, 128).is_err());
    let p = check("mod empty{}fn main(){}");
    assert!(p.function_id("main").is_some());
}

#[test]
fn module_qualifier_work_is_admitted_before_scope_expansion() {
    let name = "m".repeat(1000);
    let source = format!("mod {name}{{enum E{{End}}}}fn main(){{}}");
    let mut limits = EntrySourceLimits::DEFAULT;
    // Contextual enum, E and End are identifier tokens; one extra unit owns the module.
    limits.types.max_text_bytes = 4000;
    assert!(parse_program_syntax(&source, limits, 128).is_ok());
    limits.types.max_text_bytes = 3999;
    assert!(matches!(
        parse_program_syntax(&source, limits, 128),
        Err(EntrySourceError::Limit("module qualifier bytes"))
    ));
}

#[test]
fn unused_module_type_errors_duplicates_and_alias_cycles_are_not_ignored() {
    for source in [
        "mod m{struct R{x:Missing}}fn main(){}",
        "mod m{}mod m{}fn main(){}",
        "mod m{type A=B;type B=A;}fn main(){}",
    ] {
        assert!(
            check_program_source("bad", source, ProgramSourceLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
}

#[test]
fn raw_module_function_syntax_never_establishes_checked_or_executable_success() {
    let source = "mod m{fn unused(){missing();}}fn main(){}";
    let raw = parse_program_syntax(source, EntrySourceLimits::DEFAULT, 128).unwrap();
    assert_eq!(
        raw.function_modules().collect::<Vec<_>>(),
        [Some("m"), None]
    );
    assert_eq!(raw.functions()[0].name, "unused");
    assert!(raw.core_declarations().modules[0].functions.is_empty());
    assert!(check_program_source("bad", source, ProgramSourceLimits::DEFAULT).is_err());
    assert!(check_entry_source("bad", source, EntrySourceLimits::DEFAULT).is_err());
}

#[test]
fn raw_module_bodies_preserve_namespaces_spans_and_shared_function_allowance() {
    let source="// λ😀\r\nmod a{enum E{End}export fn same(x:E)->E{match x{E::End()=>{return x;}}}}mod b{fn same(){print(1);}}fn main(){}";
    let raw = parse_program_syntax(source, EntrySourceLimits::DEFAULT, 3).unwrap();
    assert_eq!(
        raw.function_modules().collect::<Vec<_>>(),
        [Some("a"), Some("b"), None]
    );
    assert_eq!(
        raw.functions()
            .iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>(),
        ["same", "same", "main"]
    );
    assert!(raw.functions()[0].is_exported);
    for f in raw.functions() {
        assert!(source[f.span.start..f.span.end].contains(&format!("fn {}", f.name)));
    }
    assert!(parse_program_syntax(source, EntrySourceLimits::DEFAULT, 2).is_err());
    assert!(check_program_source("checked", source, ProgramSourceLimits::DEFAULT).is_ok());
    assert!(parse_program_syntax(
        "mod a{fn broken(){let x=;}}",
        EntrySourceLimits::DEFAULT,
        128
    )
    .is_err());
}
