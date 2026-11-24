//! Primitive String standard calls preserve existing semantics with explicit budgets.
use sovra::compiler::v2::*;
fn quote(text: &str) -> String {
    format!(
        "\"{}\"",
        text.replace('\\', "\\\\")
            .replace('\"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    )
}
fn compile(source: &str) -> CompiledGenericSource {
    compile_generic_string_source(source, "main", GenericPropagationSourceLimits::DEFAULT).unwrap()
}
fn run(source: &str) -> GenericSourceOutcome {
    execute_generic_source(&compile(source), GenericVmPolicy::DEFAULT).unwrap()
}
#[test]
fn string_length_counts_utf8_bytes_including_controls_and_empty_strings() {
    for text in ["", "ASCII", "éλ😀", "a\nb", "\0"] {
        let source = format!("fn main()->Int{{return std::len({});}}", quote(text));
        let out = run(&source);
        assert_eq!(out.execution().value().as_int(), Some(text.len() as i64));
    }
}
#[test]
fn line_uniqueness_matches_existing_lf_trailing_line_and_cr_semantics() {
    for text in [
        "",
        "a",
        "a\n",
        "a\na",
        "\n",
        "\n\n",
        "a\nb\n",
        "a\r\na\n",
        "é\nλ\né",
        "a\n\na",
        "A\na",
    ] {
        let source = format!(
            "fn main()->Bool{{return std::lines_unique({});}}",
            quote(text)
        );
        let out = run(&source);
        assert_eq!(
            out.execution().value().as_bool(),
            Some(sovra::compiler::stdlib::lines_unique(text)),
            "{text:?}"
        );
    }
}
#[test]
fn terminal_escape_matches_the_existing_control_and_unicode_contract() {
    for text in ["", "plain\\text", "A\u{1b}\u{7f}\u{85}é😀", "\n\r\t\0"] {
        let source = format!(
            "fn main()->String{{return std::escape_terminal({});}}",
            quote(text)
        );
        let out = run(&source);
        let expected = sovra::compiler::stdlib::escape_terminal(text);
        assert_eq!(out.execution().value().as_str(), Some(expected.as_str()));
        assert_eq!(
            out.execution().value().footprint(),
            (1, expected.len() as u64)
        );
    }
}
#[test]
fn string_utilities_compose_in_generic_functions_and_preserve_old_profiles() {
    let source="fn helper<T>(x:T,text:String)->Int{return std::len(std::escape_terminal(text));}fn main()->Int{return helper::<Bool>(true,\"é\");}";
    let p = compile(source);
    assert_eq!(
        execute_generic_source(&p, GenericVmPolicy::DEFAULT)
            .unwrap()
            .execution()
            .value()
            .as_int(),
        Some(2)
    );
    assert!(compile_generic_builtin_source(
        source,
        "main",
        GenericPropagationSourceLimits::DEFAULT
    )
    .is_err());
    let image = compile("fn main(){let n=std::len(\"é\");if n!=2{let bad=1/0;}}")
        .encode_generic_native_image()
        .unwrap();
    assert_eq!(&image[8..12], &127u32.to_le_bytes());
    assert!(execute_generic_native_image(&image).is_ok());
    for word in [
        1u32,
        3,
        7,
        15,
        31,
        63,
        64,
        95,
        111,
        119,
        123,
        125,
        126,
        u32::MAX,
    ] {
        let mut bad = image.clone();
        bad[8..12].copy_from_slice(&word.to_le_bytes());
        assert!(decode_generic_native_image(&bad).is_err(), "{word}");
    }
}
#[test]
fn temporary_line_storage_and_escape_output_use_shared_exact_limits() {
    for source in [
        "fn main()->Bool{return std::lines_unique(\"a\\nb\\nc\");}",
        "fn main()->String{return std::escape_terminal(\"a\\n\");}",
    ] {
        let p = compile(source);
        let out = execute_generic_source(&p, GenericVmPolicy::DEFAULT).unwrap();
        let d = GenericVmPolicy::DEFAULT;
        let rem = out.execution().remaining();
        let mut exact = d.clone();
        exact.max_work = out.execution().work();
        exact.runtime.materialization.max_value_nodes = Some(
            d.runtime.materialization.max_value_nodes.unwrap()
                - rem.materialization.max_value_nodes.unwrap(),
        );
        exact.runtime.materialization.max_text_bytes = Some(
            d.runtime.materialization.max_text_bytes.unwrap()
                - rem.materialization.max_text_bytes.unwrap(),
        );
        exact.runtime.execution.max_instructions = Some(out.execution().instructions());
        assert!(execute_generic_source(&p, exact.clone()).is_ok());
        for which in 0..4 {
            let mut bad = exact.clone();
            match which {
                0 => bad.max_work -= 1,
                1 => {
                    *bad.runtime
                        .materialization
                        .max_value_nodes
                        .as_mut()
                        .unwrap() -= 1
                }
                2 => *bad.runtime.materialization.max_text_bytes.as_mut().unwrap() -= 1,
                _ => *bad.runtime.execution.max_instructions.as_mut().unwrap() -= 1,
            };
            assert!(execute_generic_source(&p, bad).is_err());
        }
    }
}
#[test]
fn string_builtin_calls_reject_invalid_types_type_arguments_and_unused_errors() {
    for call in [
        "std::len(1)",
        "std::len()",
        "std::lines_unique(true)",
        "std::lines_unique(\"a\",\"b\")",
        "std::escape_terminal::<Int>(\"x\")",
        "std::escape_terminal(false)",
    ] {
        let source = format!("fn main(){{}}fn unused(){{{call};}}");
        assert!(
            compile_generic_string_source(&source, "main", GenericPropagationSourceLimits::DEFAULT)
                .is_err(),
            "{call}"
        );
    }
}
