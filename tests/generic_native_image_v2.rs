//! Generic image admission, feature isolation, closed entry and fixed runtime policy.
use sovra::compiler::v2::*;
fn raw(source: &str) -> Vec<u8> {
    let mut bytes = b"SVRG2\0".to_vec();
    bytes.extend_from_slice(&[1, 2]);
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&(source.len() as u32).to_le_bytes());
    bytes.extend_from_slice(source.as_bytes());
    bytes
}
fn image(source: &str) -> Vec<u8> {
    compile_generic_source(source, "main", GenericSourceLimits::DEFAULT)
        .unwrap()
        .encode_generic_native_image()
        .unwrap()
}
#[test]
fn complete_generic_program_round_trips_and_executes_without_hosts() {
    let source = include_str!("../docs/examples/generic-source.svr");
    let bytes = image(source);
    assert!(is_generic_native_image(&bytes));
    assert!(!is_native_image(&bytes));
    let decoded = decode_generic_native_image(&bytes).unwrap();
    assert_eq!(decoded.source(), source);
    assert_eq!(decoded.encode_generic_native_image().unwrap(), bytes);
    let outcome = execute_generic_native_image(&bytes).unwrap();
    assert!(outcome.execution().value().is_unit());
    assert!(
        outcome
            .execution()
            .remaining()
            .execution
            .max_instructions
            .unwrap()
            < 1_000_000
    );
}
#[test]
fn all_prefixes_and_invalid_header_bytes_refuse() {
    let bytes = image("fn main(){}");
    for end in 0..bytes.len() {
        assert!(decode_generic_native_image(&bytes[..end]).is_err(), "{end}");
    }
    for at in 0..16 {
        let mut bad = bytes.clone();
        bad[at] ^= 255;
        assert!(decode_generic_native_image(&bad).is_err(), "{at}");
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert!(decode_generic_native_image(&trailing).is_err());
}
#[test]
fn declared_lengths_utf8_and_unsupported_features_are_not_trusted() {
    for count in [0, 1, u32::MAX] {
        let mut bad = image("fn main(){}");
        bad[12..16].copy_from_slice(&count.to_le_bytes());
        assert!(decode_generic_native_image(&bad).is_err());
    }
    for features in [0u32, 2, 4, u32::MAX] {
        let mut bad = image("fn main(){}");
        bad[8..12].copy_from_slice(&features.to_le_bytes());
        assert!(decode_generic_native_image(&bad).is_err());
    }
    let mut bad = image("fn main(){}");
    bad[16] = 255;
    assert!(decode_generic_native_image(&bad).is_err());
}
#[test]
fn every_source_body_and_entry_is_rechecked_at_decode() {
    for source in [
        "fn main(){} fn unused(){let bad:Bool=1;}",
        "fn main(){} struct Invalid<T>{value:Unknown}",
        "fn main(){let x=unknown();}",
        "fn other(){}",
        "fn main<T>(){}",
        "fn main(x:Int){}",
        "fn main()->Int{return 1;}",
        "fn main(){} trailing",
    ] {
        assert!(
            decode_generic_native_image(&raw(source)).is_err(),
            "{source}"
        );
    }
    let mut bytes = image("fn main(){let value=1;let copy=value;}");
    let at = std::str::from_utf8(&bytes[16..])
        .unwrap()
        .rfind("value")
        .unwrap()
        + 16;
    bytes[at..at + 5].copy_from_slice(b"other");
    assert!(decode_generic_native_image(&bytes).is_err());
}
#[test]
fn library_noncommand_roots_cannot_be_encoded() {
    for (source, entry) in [
        ("fn main()->Int{return 1;}", "main"),
        ("fn other(){}", "other"),
    ] {
        let checked = compile_generic_source(source, entry, GenericSourceLimits::DEFAULT).unwrap();
        assert!(checked.encode_generic_native_image().is_err());
    }
}
#[test]
fn exact_source_byte_ceiling_admits_and_next_byte_refuses_both_directions() {
    let mut source = "fn main(){}//".to_owned();
    source.push_str(&"x".repeat(GENERIC_NATIVE_SOURCE_LIMIT - source.len()));
    let bytes = image(&source);
    assert_eq!(bytes.len(), GENERIC_NATIVE_SOURCE_LIMIT + 16);
    decode_generic_native_image(&bytes).unwrap();
    source.push('x');
    let checked = compile_generic_source(&source, "main", GenericSourceLimits::DEFAULT).unwrap();
    assert!(checked.encode_generic_native_image().is_err());
    assert!(decode_generic_native_image(&raw(&source)).is_err());
}
#[test]
fn runtime_failures_and_infinite_loops_do_not_return_partial_success() {
    for source in [
        "fn main(){let x=1/0;}",
        "fn main(){while true{}}",
        "fn main(){let x:[Int]=[1];let y=x[2];}",
    ] {
        assert!(execute_generic_native_image(&image(source)).is_err());
    }
}
#[test]
fn all_host_calls_and_imports_are_refused_at_rechecking() {
    for source in [
        "fn main(){print(1);}",
        "fn main(){std::read_line();}",
        "fn main(){std::write_text(\"marker\",\"bad\");}",
        "import other;fn main(){}",
    ] {
        assert!(execute_generic_native_image(&raw(source)).is_err());
    }
}
#[test]
fn legacy_and_plain_v2_families_remain_distinct() {
    let old = sovra::compiler::native_image::encode(&sovra::compiler::ir::IrProgram {
        functions: vec![],
    })
    .unwrap();
    let plain = check_program_source("fixture", "fn main(){}", ProgramSourceLimits::DEFAULT)
        .unwrap()
        .encode_native_image()
        .unwrap();
    for bytes in [&old, &plain] {
        assert!(!is_generic_native_image(bytes));
        assert!(decode_generic_native_image(bytes).is_err());
    }
    let generic = image("fn main(){}");
    assert!(decode_native_image(&generic).is_err());
    assert!(sovra::compiler::native_image::decode(&generic).is_err());
    assert!(decode_native_image(&plain).is_ok());
    assert!(sovra::compiler::native_image::decode(&old).is_ok());
}
#[test]
fn command_policy_is_finite_and_host_free() {
    let policy = generic_command_runtime_policy();
    assert_eq!(policy.runtime.execution.max_instructions, Some(1_000_000));
    assert_eq!(policy.runtime.execution.max_output_bytes, Some(0));
    assert_eq!(
        policy.control.max_elapsed,
        Some(std::time::Duration::from_secs(30))
    );
    assert_eq!(policy.max_call_depth, 256);
}
