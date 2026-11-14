//! Source-backed native image admission and closed host policy.
use sovra::compiler::v2::*;
fn image(source: &str) -> Vec<u8> {
    check_program_source("fixture", source, ProgramSourceLimits::DEFAULT)
        .unwrap()
        .encode_native_image()
        .unwrap()
}
#[test]
fn enum_linked_calls_round_trip_with_arguments_status_and_finite_shared_policy() {
    let source = "enum E{A(Int),End} fn get()->E{return E::A(7);} fn main(){let e=get();match e{E::A(n)=>{print(n);},E::End()=>{}}print(std::arg(0));std::set_exit_code(7);}";
    let bytes = image(source);
    assert!(is_native_image(&bytes));
    assert_eq!(decode_native_image(&bytes).unwrap().source(), source);
    let outcome = execute_native_image(&bytes, &["λ😀".into()]).unwrap();
    assert_eq!(outcome.execution.output, ["7", "λ😀"]);
    assert_eq!(outcome.exit_code, 7);
    assert!(
        outcome
            .execution
            .remaining
            .execution
            .max_instructions
            .unwrap()
            < 1_000_000
    );
}
#[test]
fn all_truncations_trailing_bytes_versions_features_models_and_utf8_refuse() {
    let bytes = image("fn main(){}");
    for n in 0..bytes.len() {
        assert!(decode_native_image(&bytes[..n]).is_err(), "{n}");
    }
    for index in [0, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16] {
        let mut bad = bytes.clone();
        bad[index] = 255;
        assert!(decode_native_image(&bad).is_err(), "{index}");
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert!(decode_native_image(&trailing).is_err());
}
#[test]
fn semantically_invalid_embedded_source_is_rechecked_not_trusted() {
    let mut bytes = image("fn main(){let value=1;print(value);}");
    let source = std::str::from_utf8(&bytes[16..]).unwrap();
    let offset = source.rfind("value").unwrap() + 16;
    bytes[offset..offset + 5].copy_from_slice(b"other");
    assert!(decode_native_image(&bytes).is_err());
}
#[test]
fn missing_or_invalid_main_never_encodes_an_executable() {
    for source in [
        "fn helper(){}",
        "fn main(x:Int){}",
        "fn main()->Int{return 1;}",
    ] {
        if let Ok(checked) = check_program_source("fixture", source, ProgramSourceLimits::DEFAULT) {
            assert!(checked.encode_native_image().is_err());
        }
    }
}
#[test]
fn exact_source_limit_is_admitted_and_the_next_byte_is_refused() {
    let mut source = "fn main(){}//".to_owned();
    source.push_str(&"x".repeat(NATIVE_SOURCE_LIMIT - source.len()));
    let bytes = image(&source);
    assert_eq!(bytes.len(), NATIVE_SOURCE_LIMIT + 16);
    assert!(decode_native_image(&bytes).is_ok());
    let mut bad = bytes;
    bad.push(b'x');
    assert!(decode_native_image(&bad).is_err());
}
#[test]
fn native_invocations_never_grant_input_or_files_and_fail_without_output() {
    for call in [
        "std::read_line();",
        "std::read_text(\"missing\");",
        "std::write_text(\"marker\",\"bad\");",
    ] {
        let bytes = image(&format!("fn main(){{print(\"pending\");{call}}}"));
        let error = execute_native_image(&bytes, &[]).unwrap_err();
        assert!(error.contains("UnsupportedStandard"), "{error}");
    }
    let bytes = image("fn main(){while true{}}");
    assert!(execute_native_image(&bytes, &[])
        .unwrap_err()
        .contains("execution instruction allowance exceeded"));
}
#[test]
fn argument_limits_accept_exact_counts_and_text_and_refuse_next() {
    let bytes = image("fn main(){}");
    assert!(execute_native_image(&bytes, &vec![String::new(); 1024]).is_ok());
    assert!(execute_native_image(&bytes, &vec![String::new(); 1025]).is_err());
    assert!(execute_native_image(&bytes, &["λ".repeat(32768)]).is_ok());
    assert!(execute_native_image(&bytes, &["λ".repeat(32769)]).is_err());
}
#[test]
fn v1_image_encoding_and_decoding_remain_separate() {
    let old = sovra::compiler::native_image::encode(&sovra::compiler::ir::IrProgram {
        functions: vec![],
    })
    .unwrap();
    assert!(!is_native_image(&old));
    assert!(decode_native_image(&old).is_err());
    let new = image("fn main(){}");
    assert!(sovra::compiler::native_image::decode(&new).is_err());
    assert!(sovra::compiler::native_image::decode(&old).is_ok());
}

#[test]
fn source_images_do_not_promote_raw_module_function_syntax_to_executable_code() {
    let source = "mod m{fn unused(){missing();}}fn main(){}";
    assert!(parse_program_syntax(source, EntrySourceLimits::DEFAULT, 128).is_ok());
    let mut image = b"SVRV2\0".to_vec();
    image.extend_from_slice(&[1, 2]);
    image.extend_from_slice(&1u32.to_le_bytes());
    image.extend_from_slice(&(source.len() as u32).to_le_bytes());
    image.extend_from_slice(source.as_bytes());
    assert!(decode_native_image(&image).is_err());
    assert!(execute_native_image(&image, &[]).is_err());
}
