//! Bounded optional function navigation from the same successful v2 program.
use sovra::compiler::v2::*;
fn check(source: &str) -> CheckedProgramSource {
    check_program_source("opaque", source, ProgramSourceLimits::DEFAULT).unwrap()
}
#[test]
fn forward_mutual_and_recursive_calls_use_source_owned_targets() {
    let source="// λ😀\r\nenum E{End} export fn first()->E{return second();}fn second()->E{if false{return first();}return E::End();}fn main(){let e=first();}";
    let p = check(source);
    let output = p.render_function_symbols(ProgramSymbolLimits::DEFAULT);
    assert!(!output.contains("unavailable"), "{output}");
    assert!(output.contains("export fn first()->E") || output.contains("fn first()->E"));
    assert_eq!(output.matches("\"name\":").count(), 3);
    assert_eq!(output.matches("\"definition\":").count(), 3);
    for call in p.call_links() {
        assert!(output.contains(&format!(
            "{{\"start\":{},\"end\":{},\"definition\":{}}}",
            call.span.start,
            call.span.end,
            call.callee.ordinal()
        )));
    }
    assert!(output.contains("\"locals\":[],\"local_references\":[]"));
}
#[test]
fn exact_item_and_serialized_byte_caps_admit_and_next_refuses_all_facts() {
    let p = check("fn main(){helper();}fn helper(){}");
    let output = p.render_function_symbols(ProgramSymbolLimits::DEFAULT);
    let exact = ProgramSymbolLimits {
        max_items: 3,
        max_bytes: output.len(),
    };
    assert_eq!(p.render_function_symbols(exact), output);
    for limit in [
        ProgramSymbolLimits {
            max_items: 2,
            ..exact
        },
        ProgramSymbolLimits {
            max_bytes: output.len() - 1,
            ..exact
        },
        ProgramSymbolLimits {
            max_items: 0,
            max_bytes: 0,
        },
    ] {
        let unavailable = p.render_function_symbols(limit);
        assert!(unavailable.contains("\"unavailable\":true"));
        assert!(!unavailable.contains("helper"));
        assert!(unavailable.contains("\"declarations\":[],\"references\":[]"));
    }
    assert_eq!(
        p.render_function_symbols(ProgramSymbolLimits::DEFAULT),
        output
    );
}
#[test]
fn comments_strings_and_constructor_spellings_do_not_invent_function_links() {
    let p = check(
        "enum E{End}fn main(){// helper()\nprint(\"helper()\");let e=E::End();}fn helper(){}",
    );
    let out = p.render_function_symbols(ProgramSymbolLimits::DEFAULT);
    assert!(out.contains("\"references\":[]"));
    assert!(!out.contains("unavailable"));
}
#[test]
fn oversized_source_signature_is_unavailable_without_invalidating_program() {
    let source = format!("fn main({} ){{}}", " ".repeat(4100));
    let p = check(&source);
    assert!(p
        .render_function_symbols(ProgramSymbolLimits::DEFAULT)
        .contains("\"unavailable\":true"));
    assert!(p.function_id("main").is_some());
}
#[test]
fn nominal_types_and_escaped_unicode_signature_trivia_stay_original() {
    let p=check("enum E{End}fn helper(// λ😀 \"quote\" \nvalue:E)->E{return value;}fn main(){let e=helper(E::End());}");
    let out = p.render_function_symbols(ProgramSymbolLimits::DEFAULT);
    assert!(out.contains("value:E)->E"));
    assert!(out.contains("\\\"quote\\\""));
    assert!(!out.contains("unavailable"));
}
