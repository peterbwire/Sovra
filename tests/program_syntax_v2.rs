//! Untyped original-source v2 parsing does not imply semantic/compiler success.
use sovra::compiler::{ast::Statement, v2::*};
fn parse(source: &str) -> ParsedProgramSyntax {
    parse_program_syntax(source, EntrySourceLimits::DEFAULT, 128).unwrap()
}
#[test]
fn unresolved_types_names_empty_enums_and_duplicates_remain_untyped_syntax() {
    let source="enum E{A(Missing),A}enum Empty{}type Alias=Missing;fn helper(x:Missing){print(unknown);}fn helper(){}";
    let syntax = parse(source);
    assert_eq!(syntax.enums().len(), 2);
    assert_eq!(syntax.functions().len(), 2);
    assert_eq!(
        syntax.enums()[0].variants[0].payload_type.as_deref(),
        Some("Missing")
    );
    assert!(check_program_source("bad", source, ProgramSourceLimits::DEFAULT).is_err());
}
#[test]
fn recursive_bodies_and_original_declaration_coordinates_are_retained() {
    let source="// λ😀\r\nexport enum E{A(Int),End}\r\nfn main(){let e=E::A(1);while true{match e{E::A(n)=>{if n>0{match e{E::A(m)=>{},E::End()=>{}}}},E::End()=>{}}}}";
    let syntax = parse(source);
    assert_eq!(
        syntax.declarations()[0].span.start,
        source.find("export").unwrap()
    );
    assert!(syntax.core_declarations().functions.is_empty());
    let control = &syntax.functions()[0].body[1];
    assert!(
        matches!(control,EnumFunctionStatement::Control(s) if matches!(s.header,Statement::While{..})&&matches!(s.bodies[0][0],EnumFunctionStatement::RecursiveMatch(_)))
    );
}
#[test]
fn declaration_only_and_empty_source_need_no_function_but_checked_programs_do() {
    assert!(parse("").declarations().is_empty());
    let syntax = parse("type Alias=Missing;struct S{x:Missing}enum E{}");
    assert_eq!(syntax.declarations().len(), 3);
    assert!(syntax.functions().is_empty());
    assert!(matches!(
        check_program_source("missing", "type Alias=;", ProgramSourceLimits::DEFAULT),
        Err(ProgramSourceError::Source(EntrySourceError::MissingEntry))
    ));
}
#[test]
fn syntax_errors_keep_unicode_crlf_file_offsets_without_semantic_labels() {
    let source = "// λ😀\r\nenum E{A(]}fn main(){}";
    let error = parse_program_syntax(source, EntrySourceLimits::DEFAULT, 128).unwrap_err();
    let d = ProgramSourceError::Source(error).into_diagnostics();
    assert!(d
        .items
        .iter()
        .any(|d| d.span.start >= source.find(']').unwrap()));
    assert!(d.items.iter().all(|d| d.source_file.is_none()));
}
#[test]
fn source_token_declaration_and_function_caps_refuse_without_partial_syntax() {
    let source = "fn helper(){}";
    let mut limit = EntrySourceLimits::DEFAULT;
    limit.frontend.max_source_bytes = 0;
    assert!(parse_program_syntax(source, limit, 128).is_err());
    let mut limit = EntrySourceLimits::DEFAULT;
    limit.frontend.max_tokens = 0;
    assert!(parse_program_syntax(source, limit, 128).is_err());
    let mut limit = EntrySourceLimits::DEFAULT;
    limit.max_declarations = 0;
    assert!(parse_program_syntax(source, limit, 128).is_err());
    assert!(parse_program_syntax(source, EntrySourceLimits::DEFAULT, 0).is_err());
}
#[test]
fn unsupported_modules_imports_and_unbalanced_forms_never_become_partial_success() {
    for source in [
        "module m{fn f(){}}",
        "import lib::m;fn main(){}",
        "fn f(){match x{E::A()=>{}}",
        "enum E{A(Unknown}",
    ] {
        assert!(
            parse_program_syntax(source, EntrySourceLimits::DEFAULT, 128).is_err(),
            "{source}"
        );
    }
}
#[test]
fn recursion_admission_and_contextual_identifiers_keep_prior_grammar_boundaries() {
    parse("fn enum(match:Int)->Int{return match;}fn main(){print(enum(1));}");
    let source = format!("fn main(){{{}{{}}}}", "if true{}else ".repeat(150));
    assert!(parse_program_syntax(&source, EntrySourceLimits::DEFAULT, 128).is_err());
}
