//! Strict JSON syntax, numeric-kind, Unicode and resource contracts.
use sovra::compiler::v2::*;
#[test]
fn strict_json_numeric_kinds_and_order_roundtrip() {
    for input in [
        "null",
        "true",
        "false",
        "-9223372036854775808",
        "9223372036854775807",
        "1.0",
        "-0.0",
        "1e2",
        "[null,true,2,2.0,\"λ\"]",
        "{\"z\":1,\"a\":2.0}",
    ] {
        let value = parse_json(input.as_bytes(), JsonLimits::DEFAULT).unwrap();
        let text = serialize_json(&value, JsonLimits::DEFAULT).unwrap();
        assert_eq!(
            parse_json(text.as_bytes(), JsonLimits::DEFAULT).unwrap(),
            value
        );
    }
    assert!(matches!(
        parse_json(b"1.0", JsonLimits::DEFAULT),
        Ok(JsonValue::Float(_))
    ));
}
#[test]
fn strict_json_rejects_duplicates_invalid_numbers_unicode_and_trailing_data() {
    for input in [
        "{\"a\":1,\"\\u0061\":2}",
        "9223372036854775808",
        "1e400",
        "01",
        "+1",
        "NaN",
        "[1,]",
        "{\"a\":}",
        "true false",
        "\"\\ud800\"",
        "\"\\udc00\"",
        "\"\\ud800\\u0041\"",
        "\"\n\"",
    ] {
        assert!(
            parse_json(input.as_bytes(), JsonLimits::DEFAULT).is_err(),
            "{input}"
        );
    }
    assert_eq!(
        parse_json(&[b'"', 0xff, b'"'], JsonLimits::DEFAULT)
            .unwrap_err()
            .offset,
        1
    );
}
#[test]
fn strict_json_surrogate_pairs_bom_nul_and_decoded_duplicates() {
    assert_eq!(
        parse_json(br#""\ud83e\udd80""#, JsonLimits::DEFAULT).unwrap(),
        JsonValue::String("🦀".into())
    );
    assert_eq!(
        parse_json(br#""a\u0000b""#, JsonLimits::DEFAULT).unwrap(),
        JsonValue::String("a\0b".into())
    );
    let err = parse_json("{\"λ\":1,\"λ\":2}".as_bytes(), JsonLimits::DEFAULT).unwrap_err();
    assert_eq!(err.code, "duplicate");
    assert_eq!(err.offset, 8);
}
#[test]
fn strict_json_exact_resource_limits_and_invalid_policy() {
    let mut limits = JsonLimits::DEFAULT;
    limits.bytes = 4;
    limits.values = 1;
    limits.depth = 0;
    limits.text = 0;
    assert!(parse_json(b"null", limits).is_ok());
    limits.bytes = 3;
    assert!(parse_json(b"null", limits).is_err());
    limits = JsonLimits::DEFAULT;
    limits.text = 2;
    assert!(parse_json("\"λ\"".as_bytes(), limits).is_ok());
    limits.text = 1;
    assert!(parse_json("\"λ\"".as_bytes(), limits).is_err());
    limits = JsonLimits::DEFAULT;
    limits.depth = 129;
    assert!(parse_json(b"null", limits).is_err());
    let deep = format!("{}null{}", "[".repeat(129), "]".repeat(129));
    assert!(parse_json(deep.as_bytes(), JsonLimits::DEFAULT).is_err());
}
#[test]
fn strict_json_serializer_refuses_nonfinite_duplicate_and_excess_output() {
    assert!(serialize_json(&JsonValue::Float(f64::NAN), JsonLimits::DEFAULT).is_err());
    assert!(serialize_json(
        &JsonValue::Object(vec![
            ("a".into(), JsonValue::Null),
            ("a".into(), JsonValue::Null)
        ]),
        JsonLimits::DEFAULT
    )
    .is_err());
    let mut limits = JsonLimits::DEFAULT;
    limits.bytes = 3;
    assert!(serialize_json(&JsonValue::Null, limits).is_err());
    assert_eq!(
        serialize_json(&JsonValue::Float(-0.0), JsonLimits::DEFAULT).unwrap(),
        "-0.0"
    );
}
