//! Complete source admission, execution, provenance and cumulative resource checks.
use sovra::compiler::{diagnostics::Span, v2::*};
fn compile(source: &str) -> CompiledGenericSource {
    compile_generic_source(source, "main", GenericSourceLimits::DEFAULT).unwrap()
}
fn value(source: &str) -> i64 {
    execute_generic_source(&compile(source), GenericVmPolicy::DEFAULT)
        .unwrap()
        .execution()
        .value()
        .as_int()
        .unwrap()
}
const MIXED: &str = r#"
// complete file, forward references and unchanged declaration sources
fn main()->Int {return helper(Choice::<Int>::One(Box::<Int>{value:41}.value));}
type Rows<T>=[T];
struct Box<T>{value:T}
enum Choice<T>{End,One(T)}
fn helper(x:Choice<Int>)->Int {match x{Choice::End()=>{return 0;},Choice::One(v)=>{return identity::<Int>(v)+1;}}}
fn identity<T>(x:T)->T{return x;}
"#;
#[test]
fn complete_source_collects_forward_types_and_executes_generic_and_plain_calls() {
    let p = compile(MIXED);
    assert_eq!(p.source(), MIXED);
    assert_eq!(p.entry(), "main");
    assert_eq!(p.declarations().len(), 6);
    assert_eq!(p.declarations()[1].name(), "Rows");
    assert_eq!(
        execute_generic_source(&p, GenericVmPolicy::DEFAULT)
            .unwrap()
            .execution()
            .value()
            .as_int(),
        Some(42)
    );
    for d in p.declarations() {
        assert!(!p.source()[d.span().start..d.span().end].is_empty());
    }
}
#[test]
fn ordinary_recursion_and_generic_to_plain_calls_execute() {
    assert_eq!(value("fn main()->Int{return call::<String>(6);} fn call<T>(n:Int)->Int{return fact(n);} fn fact(n:Int)->Int{if n<=1{return 1;}return n*fact(n-1);}"),720);
    assert_eq!(value("fn main()->Int{return even(10);} fn even(n:Int)->Int{if n==0{return 1;}return odd(n-1);} fn odd(n:Int)->Int{if n==0{return 0;}return even(n-1);}"),1);
}
#[test]
fn grouping_comments_and_string_braces_preserve_boundaries() {
    assert_eq!(
        value(
            "// {
fn main()->Int{let text=\"}\";return (helper // comment\n)();}\nfn helper()->Int{return 7;}"
        ),
        7
    );
}
#[test]
fn all_declarations_and_unreachable_bodies_are_checked() {
    for source in [
        "fn main(){} fn unused(){let x:Bool=1;}",
        "fn main(){} struct Bad<T>{value:Missing}",
        "fn main(){} fn unused(){return missing();}",
        "fn main(){} fn unused<T>(x:T)->T{return x;} fn other(){unused(1);}",
    ] {
        assert!(
            compile_generic_source(source, "main", GenericSourceLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
}
#[test]
fn generic_inference_empty_arguments_and_value_shadowing_are_refused() {
    for source in [
        "fn main()->Int{return id(1);} fn id<T>(x:T)->T{return x;}",
        "fn main()->Int{return id::<>(1);} fn id<T>(x:T)->T{return x;}",
        "fn main()->Int{let helper=1;return helper();} fn helper()->Int{return 1;}",
    ] {
        assert!(compile_generic_source(source, "main", GenericSourceLimits::DEFAULT).is_err());
    }
}
#[test]
fn unsupported_top_level_contexts_and_trailing_tokens_are_refused() {
    for source in [
        "import x; fn main(){}",
        "module x{} fn main(){}",
        "app x{} fn main(){}",
        "fn main(){} junk",
        "type Plain=Int; fn main(){}",
        "struct Plain{} fn main(){}",
        "enum Plain{A} fn main(){}",
        "export type Id<T>=T; fn main(){}",
    ] {
        assert!(
            compile_generic_source(source, "main", GenericSourceLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
}
#[test]
fn entry_must_exist_and_have_no_type_or_value_parameters() {
    for source in ["fn other(){}", "fn main<T>(){}", "fn main(x:Int){}"] {
        assert!(compile_generic_source(source, "main", GenericSourceLimits::DEFAULT).is_err());
    }
    let p = compile("export fn main()->String{return \"ok\";}");
    assert_eq!(
        execute_generic_source(&p, GenericVmPolicy::DEFAULT)
            .unwrap()
            .execution()
            .value()
            .as_str(),
        Some("ok")
    );
}
#[test]
fn runtime_failure_maps_to_original_file_without_rewriting_inner_span() {
    let source="// café\r\n\r\nfn main()->Int{return helper();}\r\n// next\r\nfn helper()->Int{return 1/0;}";
    let p = compile(source);
    let e = execute_generic_source(&p, GenericVmPolicy::DEFAULT).unwrap_err();
    let span = e.span.unwrap();
    assert!(span.start >= source.find("fn helper").unwrap());
    assert_eq!(span.line, 4);
    assert_eq!(p.file_span(e.error.function, e.error.span), Some(span));
    assert!(span.start > e.error.span.start);
}
#[test]
fn span_mapping_validates_bytes_and_recomputes_line_and_column() {
    let source = "// λ\n  fn main()->Int{return 4;}";
    let p = compile(source);
    let offset = "fn main()->Int{return ".len();
    let local = Span {
        start: offset,
        end: offset + 1,
        line: usize::MAX,
        column: usize::MAX,
    };
    let mapped = p.file_span(0, local).unwrap();
    assert_eq!(&source[mapped.start..mapped.end], "4");
    assert_eq!(mapped.line, 1);
    assert_eq!(mapped.column, 2 + offset);
    assert!(p.file_span(usize::MAX, local).is_none());
    assert!(p
        .file_span(
            0,
            Span {
                start: 2,
                end: 1,
                ..local
            }
        )
        .is_none());
    assert!(p
        .file_span(
            0,
            Span {
                start: 0,
                end: usize::MAX,
                ..local
            }
        )
        .is_none());
}
#[test]
fn cumulative_syntax_quotas_are_exact_and_not_reset_per_declaration() {
    let usage = compile(MIXED).usage();
    macro_rules! boundary {
        ($limit:ident,$used:ident) => {{
            let mut limits = GenericSourceLimits::DEFAULT;
            limits.syntax.$limit = usage.$used;
            compile_generic_source(MIXED, "main", limits).unwrap();
            assert!(usage.$used > 0);
            limits.syntax.$limit -= 1;
            assert!(
                compile_generic_source(MIXED, "main", limits).is_err(),
                stringify!($limit)
            );
        }};
    }
    boundary!(max_declarations, declarations);
    boundary!(max_retained_source_bytes, retained_source_bytes);
    boundary!(max_tokens, tokens);
    boundary!(max_name_bytes, name_bytes);
    boundary!(max_parameters, parameters);
    boundary!(max_members, members);
    boundary!(max_type_nodes, type_nodes);
    boundary!(max_type_edges, type_edges);
    boundary!(max_parameter_uses, parameter_uses);
    boundary!(max_work, work);
}
#[test]
fn syntax_profiles_and_runtime_work_refuse_without_partial_success() {
    let source = "fn main()->Int{return 1;}";
    for limits in [
        {
            let mut l = GenericSourceLimits::DEFAULT;
            l.syntax.max_depth = 0;
            l
        },
        {
            let mut l = GenericSourceLimits::DEFAULT;
            l.syntax.max_depth = 129;
            l
        },
        {
            let mut l = GenericSourceLimits::DEFAULT;
            l.syntax.max_declaration_bytes = 1;
            l
        },
        {
            let mut l = GenericSourceLimits::DEFAULT;
            l.syntax.max_entry_bytes = 3;
            l
        },
        {
            let mut l = GenericSourceLimits::DEFAULT;
            l.syntax.frontend.max_source_bytes = 1;
            l
        },
        {
            let mut l = GenericSourceLimits::DEFAULT;
            l.ir.max_work = 0;
            l
        },
    ] {
        assert!(compile_generic_source(source, "main", limits).is_err());
    }
    let mut policy = GenericVmPolicy::DEFAULT;
    policy.max_work = 0;
    assert!(execute_generic_source(&compile(source), policy).is_err());
}
#[test]
fn every_utf8_source_prefix_is_bounded_and_never_panics() {
    let source = "// λ\nfn main()->Int{return helper();} fn helper()->Int{return 2;}";
    for end in 0..=source.len() {
        if source.is_char_boundary(end) {
            let _ = compile_generic_source(&source[..end], "main", GenericSourceLimits::DEFAULT);
        }
    }
}

#[test]
fn ordinary_calls_use_shared_call_quotas_without_written_type_list_storage() {
    let source = "fn main()->Int{return helper()+helper();} fn helper()->Int{return 1;}";
    let mut limits = GenericSourceLimits::DEFAULT;
    limits.bodies.calls.max_calls = 2;
    limits.bodies.calls.arguments.max_source_bytes = 0;
    limits.bodies.calls.arguments.max_tokens = 0;
    compile_generic_source(source, "main", limits).unwrap();
    limits.bodies.calls.max_calls = 1;
    assert!(compile_generic_source(source, "main", limits).is_err());
}
#[test]
fn unicode_byte_interiors_cannot_be_mapped_as_source_ranges() {
    let p = compile("fn main()->String{return \"λ\";}");
    let at = p.source().find('λ').unwrap();
    assert!(p
        .file_span(
            0,
            Span {
                start: at + 1,
                end: at + 2,
                line: 0,
                column: 0
            }
        )
        .is_none());
}
