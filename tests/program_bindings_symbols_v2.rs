//! Scope-owned local navigation, including payloads and indexed writes.
use sovra::compiler::v2::*;
fn check(source: &str) -> CheckedProgramSource {
    check_program_source("fixture", source, ProgramSourceLimits::DEFAULT).unwrap()
}
#[test]
fn parameter_shadowing_initializer_reads_and_cross_function_ids_stay_distinct() {
    let source="fn helper(n:Int)->Int{let x=n;if true{let n=x;print(n);}return n;}fn main(){let n=helper(7);print(n);}";
    let p = check(source);
    let output = p.render_symbols(ProgramSymbolLimits::DEFAULT);
    assert!(!output.contains("unavailable"), "{output}");
    let mut offset = 0;
    for function in p.functions() {
        let facts = function.bindings();
        for usage in facts.uses() {
            let d = facts.declaration(usage.declaration).unwrap();
            let expected = format!(
                "{{\"start\":{},\"end\":{},\"definition\":{}}}",
                usage.span.start,
                usage.span.start + d.name.len(),
                offset + d.id.ordinal()
            );
            assert!(output.contains(&expected), "missing {expected} in {output}");
        }
        offset += facts.declarations().len();
    }
    assert!(output.contains("parameter n: Int"));
    assert!(output.contains("let n: Int"));
    assert!(p
        .render_function_symbols(ProgramSymbolLimits::DEFAULT)
        .contains("\"locals\":[],\"local_references\":[]"));
}
#[test]
fn match_scrutinees_and_nested_payloads_link_their_actual_scope_declarations() {
    let source="enum E{A(Int),End}fn main(){let e=E::A(7);match e{E::A(n)=>{let e=E::A(n);match e{E::A(n)=>{print(n);},E::End()=>{}}print(n);},E::End()=>{}}}";
    let p = check(source);
    let output = p.render_symbols(ProgramSymbolLimits::DEFAULT);
    assert!(!output.contains("unavailable"), "{output}");
    assert_eq!(output.matches("match payload n: Int").count(), 2);
    let facts = p.functions()[0].bindings();
    for usage in facts.uses() {
        let d = facts.declaration(usage.declaration).unwrap();
        assert!(output.contains(&format!(
            "{{\"start\":{},\"end\":{},\"definition\":{}}}",
            usage.span.start,
            usage.span.start + d.name.len(),
            d.id.ordinal()
        )));
    }
}
#[test]
fn indexed_writes_select_only_the_root_name_and_preserve_index_read_links() {
    let source = "fn main(){let mut rows=[[1]];let i=0;rows[i]=[2];rows=rows;print(rows[i][0]);}";
    let p = check(source);
    let out = p.render_symbols(ProgramSymbolLimits::DEFAULT);
    assert!(!out.contains("unavailable"), "{out}");
    let write = source.find("rows[i]=").unwrap();
    assert!(out.contains(&format!(
        "{{\"start\":{write},\"end\":{},\"definition\":0}}",
        write + 4
    )));
    assert!(out.contains("let mut rows: Array<Array<Int>>"));
}
#[test]
fn aggregate_binding_counts_and_complete_bytes_share_optional_admission() {
    let p = check("fn main(){let x=1;print(x);}");
    let out = p.render_symbols(ProgramSymbolLimits::DEFAULT);
    let count = p.functions().len()
        + p.call_links().len()
        + p.functions()
            .iter()
            .map(|f| f.bindings().declarations().len() + f.bindings().uses().len())
            .sum::<usize>();
    let exact = ProgramSymbolLimits {
        max_items: count,
        max_bytes: out.len(),
    };
    assert_eq!(p.render_symbols(exact), out);
    for limits in [
        ProgramSymbolLimits {
            max_items: count - 1,
            ..exact
        },
        ProgramSymbolLimits {
            max_bytes: out.len() - 1,
            ..exact
        },
    ] {
        let bad = p.render_symbols(limits);
        assert!(bad.contains("\"unavailable\":true"));
        assert!(!bad.contains("let x"));
        assert!(bad.contains("\"declarations\":[],\"references\":[],\"locals\":[]"));
    }
}
#[test]
fn unknown_array_holes_are_explicitly_unavailable_types_not_any() {
    let p = check("fn main(){let empty=[];print(empty);}");
    let out = p.render_symbols(ProgramSymbolLimits::DEFAULT);
    assert!(!out.contains("\"unavailable\":true"), "{out}");
    assert!(out.contains("let empty (checked type unavailable)"));
    assert!(!out.contains("Any"));
}
