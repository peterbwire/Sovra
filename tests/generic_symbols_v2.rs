//! Optional compiler-owned root declaration evidence is complete and fail closed.
use sovra::compiler::v2::*;
const SOURCE:&str="// λ😀\r\ntype Same<T>=T;\nstruct Box<T>{value:T}\nenum Choice<T>{One(T),End}\nfn pick<T>(x:std::Option<T>)->std::Option<T>{return std::Option::<T>::Some(x?);}\nfn main(){let value=pick::<Int>(std::Option::<Int>::Some(7));}";
fn compile() -> CompiledGenericSource {
    compile_generic_array_source(SOURCE, "main", GenericPropagationSourceLimits::DEFAULT).unwrap()
}
#[test]
fn root_headers_use_checked_original_declaration_names_and_omit_injected_types() {
    let p = compile();
    let symbols = generic_source_symbols(&p, GenericSymbolLimits::DEFAULT);
    assert!(symbols.contains("\"truncated\":false"));
    for name in ["Same", "Box", "Choice", "pick", "main"] {
        assert!(
            symbols.contains(&format!("\"name\":\"{name}\"")),
            "{symbols}"
        );
    }
    assert!(symbols.contains("fn pick<T>(x:std::Option<T>)->std::Option<T>"));
    assert!(symbols.contains("\"signature\":\"struct Box<T>\""));
    assert!(!symbols.contains("\"name\":\"std::Option\""));
    assert!(symbols.contains("\"definition\":3"));
    assert!(!symbols.contains("Some(x?)"));
}
#[test]
fn every_declaration_name_offset_comes_from_its_original_owner() {
    let p = compile();
    let s = generic_source_symbols(&p, GenericSymbolLimits::DEFAULT);
    for d in p.declarations() {
        let (local, name) = match d.kind() {
            GenericSourceDeclarationKind::Function(v) => (v.name_span(), v.name()),
            GenericSourceDeclarationKind::Record(v) => (v.name_span(), v.name()),
            GenericSourceDeclarationKind::Enum(v) => (v.name_span(), v.name()),
            GenericSourceDeclarationKind::Alias(v) => (v.name_span(), v.name()),
        };
        let start = d.span().start + local.start;
        let end = d.span().start + local.end;
        assert_eq!(&SOURCE[start..end], name);
        assert!(s.contains(&format!("\"name_start\":{start},\"name_end\":{end}")));
    }
}
#[test]
fn optional_projection_exhaustion_emits_no_partial_facts_or_check_failure() {
    let p = compile();
    let d = GenericSymbolLimits::DEFAULT;
    for limits in [
        GenericSymbolLimits {
            max_declarations: 4,
            ..d
        },
        GenericSymbolLimits { max_bytes: 0, ..d },
        GenericSymbolLimits {
            max_bytes: 256,
            ..d
        },
        GenericSymbolLimits {
            max_string_bytes: 0,
            ..d
        },
        GenericSymbolLimits { max_work: 0, ..d },
    ] {
        let symbols = generic_source_symbols(&p, limits);
        assert!(symbols.contains("\"unavailable\":true"));
        assert!(symbols.contains("\"declarations\":[]"));
        assert!(!symbols.contains("\"name\":"));
    }
    assert!(execute_generic_source(&p, GenericVmPolicy::DEFAULT).is_ok());
}
#[test]
fn header_limits_are_independent_and_never_truncate_a_unicode_scalar() {
    let source = format!("fn main(//{}\n ){{}}", "λ😀".repeat(1000));
    let p = compile_generic_array_source(&source, "main", GenericPropagationSourceLimits::DEFAULT)
        .unwrap();
    let s = generic_source_symbols(&p, GenericSymbolLimits::DEFAULT);
    assert!(s.contains("\"unavailable\":true"));
    assert!(s.is_char_boundary(s.len()));
}

#[test]
fn escaped_control_headers_are_accounted_before_json_emission() {
    let source = format!("fn main(//{}\n){{}}", "\u{0085}".repeat(100));
    let p = compile_generic_array_source(&source, "main", GenericPropagationSourceLimits::DEFAULT)
        .unwrap();
    let s = generic_source_symbols(&p, GenericSymbolLimits::DEFAULT);
    assert!(s.contains("\\u0085"));
    assert!(s.contains("\"truncated\":false"));
    let limited = generic_source_symbols(
        &p,
        GenericSymbolLimits {
            max_bytes: 500,
            ..GenericSymbolLimits::DEFAULT
        },
    );
    assert!(limited.contains("\"unavailable\":true"));
    assert!(!limited.contains("\"name\":"));
}

#[test]
fn provenance_comparison_work_is_admitted_for_full_declaration_bodies() {
    let source = format!("fn main(){{let text=\"{}\";}}", "x".repeat(8000));
    let p = compile_generic_array_source(&source, "main", GenericPropagationSourceLimits::DEFAULT)
        .unwrap();
    let s = generic_source_symbols(
        &p,
        GenericSymbolLimits {
            max_work: 1000,
            ..GenericSymbolLimits::DEFAULT
        },
    );
    assert!(s.contains("\"unavailable\":true"));
    assert!(
        generic_source_symbols(&p, GenericSymbolLimits::DEFAULT).contains("\"truncated\":false")
    );
}

#[test]
fn local_projection_is_bounded_and_uses_original_body_bindings() {
    let p = compile();
    let symbols = generic_source_symbols(&p, GenericSymbolLimits::DEFAULT);
    assert!(
        symbols.contains("\"signature\":\"parameter x: std::Option<T>\""),
        "{symbols}"
    );
    assert!(symbols.contains("\"signature\":\"let value: std::Option<Int>\""));
    assert!(symbols.contains("\"local_references\":[{"));
    for limits in [
        GenericSymbolLimits {
            max_locals: 0,
            ..GenericSymbolLimits::DEFAULT
        },
        GenericSymbolLimits {
            max_local_references: 0,
            ..GenericSymbolLimits::DEFAULT
        },
    ] {
        let unavailable = generic_source_symbols(&p, limits);
        assert!(unavailable.contains("\"unavailable\":true"));
        assert!(!unavailable.contains("\"name\":"));
    }
}

#[test]
fn local_hover_renders_checked_alias_expansion_and_symbolic_nominal_arguments() {
    let source="struct Pair<A,B>{left:A,right:B}type Same<T>=Pair<[T],std::Option<T>>;fn helper<T>(x:Same<T>)->Same<T>{let mut y=x;return y;}fn main(){let a=1;let b:[Int]=[a];let c:[[Bool]]=[[true]];}";
    let p = compile_generic_array_source(source, "main", GenericPropagationSourceLimits::DEFAULT)
        .unwrap();
    let symbols = generic_source_symbols(&p, GenericSymbolLimits::DEFAULT);
    for signature in [
        "parameter x: Pair<[T], std::Option<T>>",
        "let mut y: Pair<[T], std::Option<T>>",
        "let a: Int",
        "let b: [Int]",
        "let c: [[Bool]]",
    ] {
        assert!(
            symbols.contains(&format!("\"signature\":\"{signature}\"")),
            "{symbols}"
        );
    }
    assert!(!symbols.contains("inferred_type"));
}

#[test]
fn expanded_type_rendering_exhaustion_discards_all_optional_facts() {
    let source =
        "type L<T>=std::Result<[T],std::Option<[T]>>;fn f<T>(x:L<T>)->L<T>{return x;}fn main(){}";
    let p = compile_generic_array_source(source, "main", GenericPropagationSourceLimits::DEFAULT)
        .unwrap();
    // Header fits this limit; expanded binding label exceeds it.
    let symbols = generic_source_symbols(
        &p,
        GenericSymbolLimits {
            max_string_bytes: 44,
            ..GenericSymbolLimits::DEFAULT
        },
    );
    assert!(symbols.contains("\"unavailable\":true"), "{symbols}");
    assert!(!symbols.contains("\"name\":"));
    assert!(execute_generic_source(&p, GenericVmPolicy::DEFAULT).is_ok());
}

#[test]
fn function_header_type_references_preserve_alias_and_type_namespace_identity() {
    let source="// λ😀\r\nstruct Same<T>{value:T}type Alias<T>=Same<T>;enum Choice<T>{One(T),End}fn unused<T>(a:Alias<T>,b:[Choice<T>])->Same<T>{return Same::<T>{value:a.value};}fn Same()->Int{return 1;}fn main(){let n=Same();}";
    let p = compile_generic_array_source(source, "main", GenericPropagationSourceLimits::DEFAULT)
        .unwrap();
    let symbols = generic_source_symbols(&p, GenericSymbolLimits::DEFAULT);
    for (text, offset, definition) in [
        ("a:Alias<T>", 2, 1),
        ("b:[Choice<T>]", 3, 2),
        ("->Same<T>", 2, 0),
        ("n=Same()", 2, 4),
    ] {
        let start = source.find(text).unwrap() + offset;
        let end = start
            + if definition == 1 {
                5
            } else if definition == 2 {
                6
            } else {
                4
            };
        assert!(
            symbols.contains(&format!(
                "{{\"start\":{start},\"end\":{end},\"definition\":{definition}}}"
            )),
            "{symbols}"
        );
    }
    // One call and three annotation sites, including an unused generic function.
    let refs = symbols
        .split("\"references\":[")
        .nth(1)
        .unwrap()
        .split("],\"locals\":")
        .next()
        .unwrap();
    assert_eq!(refs.matches("\"definition\":").count(), 4);
    let refused = generic_source_symbols(
        &p,
        GenericSymbolLimits {
            max_references: 3,
            ..GenericSymbolLimits::DEFAULT
        },
    );
    assert!(refused.contains("\"unavailable\":true"));
    assert!(!refused.contains("\"definition\":"));
}

#[test]
fn compiler_injected_types_and_symbolic_parameters_have_no_root_definition_reference() {
    let source="struct Box<T>{value:T}fn unused<Box>(x:Box)->Box{return x;}fn fallible(x:std::Option<Int>)->std::Option<Int>{return x;}fn main(){}";
    let p = compile_generic_array_source(source, "main", GenericPropagationSourceLimits::DEFAULT)
        .unwrap();
    let symbols = generic_source_symbols(&p, GenericSymbolLimits::DEFAULT);
    assert!(symbols.contains("\"references\":[]"), "{symbols}");
}
