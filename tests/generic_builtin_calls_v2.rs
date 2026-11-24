//! Host-free standard call bridging with legacy result and adapter parity.
use sovra::compiler::v2::*;
fn compile(source: &str) -> CompiledGenericSource {
    compile_generic_builtin_source(source, "main", GenericPropagationSourceLimits::DEFAULT).unwrap()
}
fn run(source: &str) -> GenericSourceOutcome {
    execute_generic_source(&compile(source), GenericVmPolicy::DEFAULT).unwrap()
}
#[test]
fn decimal_builtin_matches_the_existing_strict_grammar_and_overflow_contract() {
    for text in [
        "",
        "0",
        "-0",
        "1",
        "-9",
        "9223372036854775807",
        "-9223372036854775808",
        "9223372036854775808",
        "-9223372036854775809",
        "01",
        "+1",
        " 1",
        "1 ",
        "1.0",
        "１２",
        "--1",
    ] {
        let out = run(&format!(
            "fn main()->std::ParsedInt{{return std::parse_int({text:?});}}"
        ));
        let expected = sovra::compiler::stdlib::parse_int(text);
        assert_eq!(
            out.execution().value().field(0).unwrap().as_bool(),
            Some(expected.is_some()),
            "{text:?}"
        );
        assert_eq!(
            out.execution().value().field(1).unwrap().as_int(),
            Some(expected.unwrap_or_default())
        );
    }
}
#[test]
fn split_builtin_preserves_first_match_empty_delimiter_unicode_and_absence() {
    for (text, delimiter) in [
        ("left,right,rest", ","),
        ("none", ":"),
        ("", ":"),
        ("abc", ""),
        ("", ""),
        ("éλé", "λ"),
        ("aaaa", "aa"),
        ("a,b", ",b"),
        ("end:", ":"),
        (":start", ":"),
    ] {
        let out = run(&format!(
            "fn main()->std::SplitOnce{{return std::split_once({text:?},{delimiter:?});}}"
        ));
        let expected = text.split_once(delimiter);
        let (found, before, after) = expected.map_or((false, text, ""), |(a, b)| (true, a, b));
        let value = out.execution().value();
        assert_eq!(value.field(0).unwrap().as_bool(), Some(found));
        assert_eq!(value.field(1).unwrap().as_str(), Some(before));
        assert_eq!(value.field(2).unwrap().as_str(), Some(after));
    }
}
#[test]
fn standard_calls_compose_with_explicit_adapters_questions_and_generic_instances() {
    let source=format!("{STANDARD_RESULT_ADAPTER_SOURCE}\nfn take<T>(fallback:T)->std::Option<Int>{{let pair=adapt_split_once(std::split_once(\"41,tail\",\",\"))?;let n=adapt_parsed_int(std::parse_int(pair[0]))?;return std::Option::<Int>::Some(n+1);}}fn main()->std::Option<Int>{{return take::<Bool>(true);}}");
    let out = run(&source);
    assert_eq!(
        out.execution().value().payload().unwrap().as_int(),
        Some(42)
    );
    assert!(out.program().has_standard_builtins());
    assert!(compile_generic_adapter_source(
        &source,
        "main",
        GenericPropagationSourceLimits::DEFAULT
    )
    .is_err());
    let absent=run(&format!("{STANDARD_RESULT_ADAPTER_SOURCE}fn main()->std::Option<Int>{{let n=adapt_parsed_int(std::parse_int(\"bad\"))?;return std::Option::<Int>::Some(1/0);}}"));
    assert!(absent.execution().value().payload().is_none());
}
#[test]
fn standard_calls_reject_wrong_arity_types_type_arguments_and_host_operations() {
    for call in [
        "std::parse_int()",
        "std::parse_int(1)",
        "std::parse_int(\"1\",\"2\")",
        "std::parse_int::<Int>(\"1\")",
        "std::split_once(\"x\")",
        "std::split_once(\"x\",true)",
        "std::read_text(\"x\")",
        "std::read_line()",
    ] {
        let source = format!("fn main(){{}}fn unused(){{{call};}}");
        assert!(
            compile_generic_builtin_source(
                &source,
                "main",
                GenericPropagationSourceLimits::DEFAULT
            )
            .is_err(),
            "{call}"
        );
    }
}
#[test]
fn native_builtin_profile_rechecks_and_refuses_older_environment_downgrades() {
    let source = "fn main(){let p=std::parse_int(\"7\");if p.ok&&p.value==7{}else{let bad=1/0;}}";
    let program = compile(source);
    let image = program.encode_generic_native_image().unwrap();
    assert_eq!(&image[8..12], &63u32.to_le_bytes());
    assert!(execute_generic_native_image(&image).is_ok());
    for word in [1u32, 3, 7, 15, 31, 32, 47, 55, 59, 61, 62, u32::MAX] {
        let mut bad = image.clone();
        bad[8..12].copy_from_slice(&word.to_le_bytes());
        assert!(decode_generic_native_image(&bad).is_err(), "{word}");
    }
}
#[test]
fn builtin_execution_uses_exact_shared_work_nodes_text_and_instruction_fuel() {
    let source = "fn main()->std::SplitOnce{return std::split_once(\"é,left,right\",\",\");}";
    let program = compile(source);
    let out = execute_generic_source(&program, GenericVmPolicy::DEFAULT).unwrap();
    let d = GenericVmPolicy::DEFAULT;
    let remaining = out.execution().remaining();
    let mut exact = d.clone();
    exact.max_work = out.execution().work();
    exact.runtime.execution.max_instructions = Some(out.execution().instructions());
    exact.runtime.materialization.max_value_nodes = Some(
        d.runtime.materialization.max_value_nodes.unwrap()
            - remaining.materialization.max_value_nodes.unwrap(),
    );
    exact.runtime.materialization.max_text_bytes = Some(
        d.runtime.materialization.max_text_bytes.unwrap()
            - remaining.materialization.max_text_bytes.unwrap(),
    );
    assert!(execute_generic_source(&program, exact.clone()).is_ok());
    for cap in 0..4 {
        let mut bad = exact.clone();
        match cap {
            0 => bad.max_work -= 1,
            1 => *bad.runtime.execution.max_instructions.as_mut().unwrap() -= 1,
            2 => {
                *bad.runtime
                    .materialization
                    .max_value_nodes
                    .as_mut()
                    .unwrap() -= 1
            }
            _ => *bad.runtime.materialization.max_text_bytes.as_mut().unwrap() -= 1,
        };
        assert!(execute_generic_source(&program, bad).is_err());
    }
}
#[test]
fn builtin_compilation_charges_argument_edges_and_retains_original_call_ranges() {
    let source = "// λ\nfn main()->std::ParsedInt{return std::parse_int(\"9\");}";
    let p = compile(source);
    let builtin = p.ir().functions()[0]
        .instructions()
        .iter()
        .find(|i| matches!(i.operation, GenericIrOperation::Builtin { .. }))
        .unwrap();
    let span = p.file_span(0, builtin.span).unwrap();
    assert_eq!(&source[span.start..span.end], "std::parse_int(\"9\")");
    let d = GenericPropagationSourceLimits::DEFAULT;
    let mut bad = d;
    bad.source.bodies.calls.max_values = 0;
    // A zero shared value allowance must refuse without a host or legacy fallback.
    assert!(compile_generic_builtin_source(source, "main", bad).is_err());
}

#[test]
fn split_scan_work_is_admitted_before_scanning_and_locates_the_original_call() {
    let source = format!(
        "fn main()->std::SplitOnce{{return std::split_once({:?},{:?});}}",
        "a".repeat(5000),
        "b".repeat(5000)
    );
    let p = compile(&source);
    let error = execute_generic_source(&p, GenericVmPolicy::DEFAULT).unwrap_err();
    assert!(matches!(error.error.kind, GenericVmErrorKind::Limit(_)));
    let span = error.span.unwrap();
    assert!(source[span.start..span.end].starts_with("std::split_once("));
    let mut limits = GenericPropagationSourceLimits::DEFAULT;
    limits.source.bodies.calls.max_calls = 0;
    assert!(
        compile_generic_builtin_source("fn main(){std::parse_int(\"1\");}", "main", limits)
            .is_err()
    );
}
