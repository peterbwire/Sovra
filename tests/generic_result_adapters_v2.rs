//! Explicit standard-result schemas and reusable Sovra source adapters.
use sovra::compiler::v2::*;
fn program(body: &str) -> CompiledGenericSource {
    let source = format!("{STANDARD_RESULT_ADAPTER_SOURCE}\n{body}");
    compile_generic_adapter_source(&source, "main", GenericPropagationSourceLimits::DEFAULT)
        .unwrap()
}
fn run(body: &str) -> GenericSourceOutcome {
    execute_generic_source(&program(body), GenericVmPolicy::DEFAULT).unwrap()
}
#[test]
fn parsed_integer_and_input_line_preserve_flags_and_empty_successes() {
    let out = run(
        "fn main()->std::Option<Int>{return adapt_parsed_int(std::ParsedInt{ok:true,value:42});}",
    );
    assert_eq!(
        out.execution().value().payload().unwrap().as_int(),
        Some(42)
    );
    let out = run(
        "fn main()->std::Option<Int>{return adapt_parsed_int(std::ParsedInt{ok:false,value:42});}",
    );
    assert!(out.execution().value().payload().is_none());
    for (eof, expected) in [(false, Some("")), (true, None)] {
        let out=run(&format!("fn main()->std::Option<String>{{return adapt_input_line(std::InputLine{{eof:{eof},text:\"\"}});}}"));
        assert_eq!(
            out.execution().value().payload().and_then(|p| p.as_str()),
            expected
        );
    }
}
#[test]
fn split_once_keeps_both_parts_and_distinguishes_absence() {
    let out=run("fn main()->std::Option<[String]>{return adapt_split_once(std::SplitOnce{found:true,before:\"left\",after:\"\"});}");
    let pair = out.execution().value().payload().unwrap();
    assert_eq!(pair.array_len(), Some(2));
    assert_eq!(pair.array_item(0).unwrap().as_str(), Some("left"));
    assert_eq!(pair.array_item(1).unwrap().as_str(), Some(""));
    assert!(run("fn main()->std::Option<[String]>{return adapt_split_once(std::SplitOnce{found:false,before:\"ignored\",after:\"ignored\"});}").execution().value().payload().is_none());
}
#[test]
fn text_read_and_write_preserve_success_or_explicit_error_message() {
    let out=run("fn main()->std::Result<String,std::Error>{return adapt_text_read(std::TextRead{ok:true,text:\"\",error:\"ignored\"});}");
    assert_eq!(out.execution().value().enum_tag(), Some(0));
    assert_eq!(
        out.execution().value().payload().unwrap().as_str(),
        Some("")
    );
    let out=run("fn main()->std::Result<Unit,std::Error>{return adapt_text_write(std::TextWrite{ok:true,error:\"ignored\"});}");
    assert!(out.execution().value().payload().unwrap().is_unit());
    for (adapter, ty, fields, code) in [
        (
            "adapt_text_read",
            "std::TextRead",
            "ok:false,text:\"ignored\",error:\"denied\"",
            "text_read_failed",
        ),
        (
            "adapt_text_write",
            "std::TextWrite",
            "ok:false,error:\"denied\"",
            "text_write_failed",
        ),
    ] {
        let success = if adapter == "adapt_text_read" {
            "String"
        } else {
            "Unit"
        };
        let out = run(&format!(
            "fn main()->std::Result<{success},std::Error>{{return {adapter}({ty}{{{fields}}});}}"
        ));
        assert_eq!(out.execution().value().enum_tag(), Some(1));
        let e = out.execution().value().payload().unwrap();
        assert_eq!(e.field(0).unwrap().as_str(), Some(code));
        assert_eq!(e.field(1).unwrap().as_str(), Some("denied"));
        assert!(e.field(2).unwrap().payload().is_none());
    }
}
#[test]
fn adapters_compose_with_question_and_are_native_profile_isolated() {
    let p=program("fn main(){let result=helper();match result{std::Option::Some(n)=>{let bad=1/0;},std::Option::None()=>{}}}fn helper()->std::Option<Int>{let n=adapt_parsed_int(std::ParsedInt{ok:false,value:1})?;return std::Option::<Int>::Some(n+1);}");
    assert!(p.has_standard_results());
    assert!(execute_generic_source(&p, GenericVmPolicy::DEFAULT).is_ok());
    let image = p.encode_generic_native_image().unwrap();
    assert_eq!(&image[8..12], &31u32.to_le_bytes());
    assert!(execute_generic_native_image(&image).is_ok());
    for word in [1u32, 3, 7, 15, 16, 17, 23, 27, 29, 30, u32::MAX] {
        let mut bad = image.clone();
        bad[8..12].copy_from_slice(&word.to_le_bytes());
        assert!(decode_generic_native_image(&bad).is_err(), "{word}");
    }
    assert!(compile_generic_propagation_source(
        p.source(),
        "main",
        GenericPropagationSourceLimits::DEFAULT
    )
    .is_err());
}
#[test]
fn standard_names_are_qualified_and_cannot_be_replaced_by_user_lookalikes() {
    let p=program("struct ParsedInt<T>{other:Bool}fn main()->std::Option<Int>{let x=ParsedInt::<Int>{other:true};return adapt_parsed_int(std::ParsedInt{ok:true,value:3});}");
    assert!(execute_generic_source(&p, GenericVmPolicy::DEFAULT).is_ok());
    let bad=format!("{STANDARD_RESULT_ADAPTER_SOURCE}struct ParsedInt<T>{{ok:Bool,value:Int}}fn main()->std::Option<Int>{{return adapt_parsed_int(ParsedInt::<Int>{{ok:true,value:1}});}}");
    assert!(
        compile_generic_adapter_source(&bad, "main", GenericPropagationSourceLimits::DEFAULT)
            .is_err()
    );
}

#[test]
fn adapters_retain_shared_resource_limits_and_do_not_enable_host_calls() {
    let p=program("fn main()->std::Result<String,std::Error>{return adapt_text_read(std::TextRead{ok:false,text:\"ignored\",error:\"denied\"});}");
    let out = execute_generic_source(&p, GenericVmPolicy::DEFAULT).unwrap();
    assert!(execute_generic_source(
        &p,
        GenericVmPolicy {
            max_work: out.execution().work(),
            ..GenericVmPolicy::DEFAULT
        }
    )
    .is_ok());
    assert!(execute_generic_source(
        &p,
        GenericVmPolicy {
            max_work: out.execution().work() - 1,
            ..GenericVmPolicy::DEFAULT
        }
    )
    .is_err());
    assert!(compile_generic_adapter_source(
        "fn main(){let value=std::parse_int(\"1\");}",
        "main",
        GenericPropagationSourceLimits::DEFAULT
    )
    .is_err());
    for source in [
        "fn main(){std::ParsedInt::<Int>{ok:true,value:1};}",
        "fn main(){std::ParsedInt{ok:true,value:false};}",
        "fn main(){std::ParsedInt{ok:true};}",
    ] {
        assert!(compile_generic_adapter_source(
            source,
            "main",
            GenericPropagationSourceLimits::DEFAULT
        )
        .is_err());
    }
}
