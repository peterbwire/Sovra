//! Deterministic local identities, full linked lowering, escaped and bounded inspection.
use sovra::compiler::v2::*;
fn check(source: &str) -> CheckedProgramSource {
    check_program_source("root.svr", source, ProgramSourceLimits::DEFAULT).unwrap()
}
fn render(program: &CheckedProgramSource) -> String {
    program
        .render_ir(ProgramLoweringLimits::DEFAULT, ProgramIrLimits::DEFAULT)
        .unwrap()
}
#[test]
fn independent_enum_program_has_owned_links_payloads_roots_and_instructions() {
    let text = render(&check(include_str!("../examples/enum-functions/main.svr")));
    for part in [
        "sovra-ir v2 inspection-only\n",
        "materialization-model 2\n",
        "enum e0 \"Outcome\"",
        "variant 0 \"Value\" payload",
        "record \"Packet\"",
        "function f0 \"main\"",
        "function f1 \"make\"",
        "call function f1",
        "call function f2",
        "call constructor e0 variant 0",
        "switch m0 e0",
        "payload m0",
        "root r0",
        "pc0 enter",
    ] {
        assert!(text.contains(part), "missing {part}: {text}");
    }
    assert!(!text.contains("unit:") && !text.contains("environment:"));
}
#[test]
fn independently_checked_and_repeated_programs_ignore_process_identity_and_labels() {
    let source = include_str!("../examples/control-matches/main.svr");
    let first = check(source);
    let text = render(&first);
    assert_eq!(text, render(&first));
    for label in ["different.svr", "/untrusted\n\x1b/path.svr"] {
        let next = check_program_source(label, source, ProgramSourceLimits::DEFAULT).unwrap();
        assert_eq!(text, render(&next));
    }
    for part in [
        "branch_false",
        "jump pc",
        "snapshot m",
        "release_match",
        "store d",
        "switch m",
    ] {
        assert!(text.contains(part), "{part}");
    }
}
#[test]
fn conversions_arrays_records_fields_indexes_and_parameter_slots_are_visible() {
    let program=check("struct Pair{x:Float}fn main(){let mut xs=[1.0,2];xs[0]=3;let p=Pair{x:widen(2)};print(p.x);print(xs[0]);}fn widen(n:Int)->Float{return n+1;}");
    let text = render(&program);
    for part in [
        "-> Float",
        "array [",
        "record [",
        "field v",
        "index v",
        "store_index d",
        "parameter 0 d",
        "binary \"+\"",
        "context c",
        "convert v",
    ] {
        assert!(text.contains(part), "missing {part}: {text}");
    }
}
#[test]
fn helpers_without_main_and_mutual_calls_lower_without_running() {
    let program=check("fn even(n:Int)->Bool{if n==0{return true;}else{return odd(n-1);}}fn odd(n:Int)->Bool{if n==0{return false;}else{return even(n-1);}}");
    let text = render(&program);
    assert!(text.contains("function f0 \"even\""));
    assert!(text.contains("call function f0") && text.contains("call function f1"));
}
#[test]
fn recursive_nominal_payloads_remain_flat_and_nullary_is_distinct_from_unit() {
    let text = render(&check(
        "enum Node{Next(Node),End,Done(Unit)}fn helper(){let node=Node::End();}",
    ));
    assert!(text.contains("enum e0"));
    assert!(text.contains("variant 1 \"End\" nullary"));
    assert!(text.contains("variant 2 \"Done\" payload"));
    assert!(text.len() < 8192);
}
#[test]
fn output_boundaries_count_final_escaped_utf8_bytes_exactly() {
    let program = check("fn helper(){print(\"λ\\n\\t\\\"\");}");
    let text = render(&program);
    assert!(text.contains("\\n") && text.contains("\\t"));
    assert!(!text.contains('\t'));
    let exact = program
        .render_ir(
            ProgramLoweringLimits::DEFAULT,
            ProgramIrLimits {
                max_bytes: text.len(),
            },
        )
        .unwrap();
    assert_eq!(exact, text);
    for max_bytes in [0, text.len() - 1] {
        assert_eq!(
            program.render_ir(
                ProgramLoweringLimits::DEFAULT,
                ProgramIrLimits { max_bytes }
            ),
            Err(ProgramIrError::Limit)
        );
    }
}
#[test]
fn compiler_refusals_precede_rendering_without_fallback_or_partial_text() {
    let program = check("fn helper(){print(1);}");
    let mut lowering = ProgramLoweringLimits::DEFAULT;
    lowering.max_mapping_nodes = 0;
    assert!(matches!(
        program.render_ir(lowering, ProgramIrLimits::DEFAULT),
        Err(ProgramIrError::Lowering(_))
    ));
    let mut lowering = ProgramLoweringLimits::DEFAULT;
    lowering.max_instructions = 0;
    assert!(matches!(
        program.render_ir(lowering, ProgramIrLimits::DEFAULT),
        Err(ProgramIrError::Lowering(_))
    ));
}
#[test]
fn standard_calls_are_inspected_without_host_grants_or_execution() {
    let text=render(&check("fn helper(){let line=std::read_line();std::write_text(\"p\",line.text);std::set_exit_code(7);while true{}}"));
    for name in ["std::read_line", "std::write_text", "std::set_exit_code"] {
        assert!(text.contains(&format!("call standard {name:?}")));
    }
}
