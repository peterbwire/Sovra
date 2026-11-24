//! Distinct bytes, full immutable copies, strict RFC 4648 and UTF-8 refusal.
use sovra::compiler::{
    execution::{CancellationToken, ExecutionControl, MaterializationLimits as Limits},
    v2::*,
};
fn fuel(n: u64, t: u64) -> RuntimeValueFuel {
    RuntimeValueFuel::new(Limits {
        max_value_nodes: Some(n),
        max_text_bytes: Some(t),
    })
}
fn unlimited() -> RuntimeValueFuel {
    RuntimeValueFuel::new(Limits::UNLIMITED)
}
fn work() -> RuntimeDataWork {
    RuntimeDataWork::new(100_000_000)
}
fn bytes(data: &[u8]) -> RuntimeBytesV2 {
    RuntimeBytesV2::from_slice(
        data,
        RuntimeBytesLimits::DEFAULT,
        &mut unlimited(),
        &mut work(),
    )
    .unwrap()
}
const L: RuntimeBytesLimits = RuntimeBytesLimits::DEFAULT;
#[test]
fn byte_is_a_checked_distinct_scalar_with_explicit_node_charges() {
    let mut f = fuel(3, 0);
    let b = RuntimeByteV2::from_int(255, &mut f).unwrap();
    assert_eq!(b.value(), 255);
    let copied = b.snapshot(&mut f).unwrap();
    assert_eq!(copied.to_int(&mut f).unwrap(), 255);
    assert_eq!(f.remaining().max_value_nodes, Some(0));
    assert!(b.snapshot(&mut f).is_err());
    for value in [i64::MIN, -1, 256, i64::MAX] {
        let mut f = fuel(1, 0);
        let e = RuntimeByteV2::from_int(value, &mut f).unwrap_err();
        assert_eq!(e.kind, RuntimeDataErrorKind::ByteRange);
        assert_eq!(e.offset, None);
        assert_eq!(f.remaining().max_value_nodes, Some(1));
    }
    assert_eq!(
        RuntimeByteV2::from_int(0, &mut unlimited())
            .unwrap()
            .value(),
        0
    );
}
#[test]
fn bytes_charge_one_buffer_plus_each_byte_and_no_binary_text() {
    let mut f = fuel(4, 0);
    let mut source = vec![0, 128, 255];
    let b = RuntimeBytesV2::from_slice(&source, L, &mut f, &mut work()).unwrap();
    source[0] = 99;
    assert_eq!(b.as_slice(), [0, 128, 255]);
    assert_eq!(b.footprint(), (4, 0));
    assert_eq!(f.remaining().max_value_nodes, Some(0));
    assert_eq!(f.remaining().max_text_bytes, Some(0));
    assert!(!b.is_empty());
    assert_eq!(b.len(), 3);
    let mut f = fuel(3, 17);
    let e = RuntimeBytesV2::from_slice(&source, L, &mut f, &mut work()).unwrap_err();
    assert_eq!(
        e.kind,
        RuntimeDataErrorKind::Materialization(RuntimeValueError::NodeLimit)
    );
    assert_eq!(f.remaining().max_value_nodes, Some(3));
    assert_eq!(f.remaining().max_text_bytes, Some(17));
    let empty = RuntimeBytesV2::from_slice(&[], L, &mut fuel(1, 0), &mut work()).unwrap();
    assert!(empty.is_empty());
    assert_eq!(empty.footprint(), (1, 0));
}
#[test]
fn snapshots_arrays_slices_concat_and_indexing_are_complete_independent_copies() {
    let original = bytes(&[0, 1, 2, 255]);
    let copy = original.snapshot(L, &mut fuel(5, 0), &mut work()).unwrap();
    assert_ne!(original.as_slice().as_ptr(), copy.as_slice().as_ptr());
    let slice = original
        .slice(1, 3, L, &mut fuel(3, 0), &mut work())
        .unwrap();
    assert_eq!(slice.as_slice(), [1, 2]);
    assert_eq!(original.as_slice(), [0, 1, 2, 255]);
    let joined = slice
        .concat(&bytes(&[7]), L, &mut fuel(4, 0), &mut work())
        .unwrap();
    assert_eq!(joined.as_slice(), [1, 2, 7]);
    assert_eq!(
        original
            .get(3, &mut fuel(1, 0), &mut work())
            .unwrap()
            .value(),
        255
    );
    let array = original.to_array(&mut fuel(5, 0), &mut work()).unwrap();
    let rebuilt = RuntimeBytesV2::from_array(&array, L, &mut fuel(5, 0), &mut work()).unwrap();
    assert!(original.equal(&rebuilt, &mut work()).unwrap());
    assert!(original
        .slice(4, 4, L, &mut fuel(1, 0), &mut work())
        .unwrap()
        .is_empty());
    for (a, b, offset) in [(3, 2, 3), (0, 5, 5), (5, 5, 5)] {
        let mut f = fuel(100, 100);
        let e = original.slice(a, b, L, &mut f, &mut work()).unwrap_err();
        assert_eq!(e.kind, RuntimeDataErrorKind::Bounds);
        assert_eq!(e.offset, Some(offset));
        assert_eq!(f.remaining().max_value_nodes, Some(100));
    }
    assert_eq!(
        original
            .get(4, &mut unlimited(), &mut work())
            .unwrap_err()
            .offset,
        Some(4)
    );
}
#[test]
fn hex_has_lowercase_output_accepts_both_cases_and_reports_original_byte_offsets() {
    let b = bytes(&[0, 15, 16, 127, 128, 255]);
    assert_eq!(
        b.to_hex(L, &mut fuel(1, 12), &mut work()).unwrap(),
        "000f107f80ff"
    );
    assert_eq!(
        RuntimeBytesV2::from_hex("000F107f80Ff", L, &mut fuel(7, 0), &mut work())
            .unwrap()
            .as_slice(),
        b.as_slice()
    );
    for (text, offset) in [
        ("0", 0),
        ("abc", 2),
        ("0x", 1),
        ("00 1", 2),
        ("λ0", 0),
        ("ff\n", 2),
    ] {
        let mut f = fuel(100, 100);
        let e = RuntimeBytesV2::from_hex(text, L, &mut f, &mut work()).unwrap_err();
        assert_eq!(e.kind, RuntimeDataErrorKind::Hex, "{text}");
        assert_eq!(e.offset, Some(offset), "{text}");
        assert_eq!(f.remaining().max_value_nodes, Some(100));
        assert_eq!(f.remaining().max_text_bytes, Some(100));
    }
}
#[test]
fn base64_matches_rfc4648_vectors_and_standard_alphabet() {
    for (plain, encoded) in [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ] {
        let b = bytes(plain.as_bytes());
        assert_eq!(
            b.to_base64(L, &mut fuel(1, encoded.len() as u64), &mut work())
                .unwrap(),
            encoded
        );
        assert_eq!(
            RuntimeBytesV2::from_base64(
                encoded,
                L,
                &mut fuel(plain.len() as u64 + 1, 0),
                &mut work()
            )
            .unwrap()
            .as_slice(),
            plain.as_bytes()
        );
    }
    assert_eq!(
        bytes(&[251, 255, 255])
            .to_base64(L, &mut unlimited(), &mut work())
            .unwrap(),
        "+///"
    );
    assert_eq!(
        RuntimeBytesV2::from_base64("+///", L, &mut unlimited(), &mut work())
            .unwrap()
            .as_slice(),
        [251, 255, 255]
    );
}
#[test]
fn base64_rejects_noncanonical_bits_padding_whitespace_and_url_alphabet() {
    for (text, offset) in [
        ("Z", 1),
        ("Zg", 2),
        ("Zg=", 3),
        ("Zh==", 1),
        ("Zm9=", 2),
        ("Zg==\n", 4),
        ("Zg==A", 4),
        ("Zg==Zg==", 4),
        ("A===", 1),
        ("====", 0),
        ("AA=A", 3),
        ("_AAA", 0),
        ("-AAA", 0),
        ("AA A", 2),
        ("λ==", 0),
    ] {
        let mut f = fuel(100, 100);
        let e = RuntimeBytesV2::from_base64(text, L, &mut f, &mut work()).unwrap_err();
        assert_eq!(e.kind, RuntimeDataErrorKind::Base64, "{text}");
        assert_eq!(e.offset, Some(offset), "{text}");
        assert_eq!(f.remaining().max_value_nodes, Some(100));
        assert_eq!(f.remaining().max_text_bytes, Some(100));
    }
}
#[test]
fn utf8_is_strict_preserves_bom_nul_and_unicode_without_normalization() {
    for text in ["", "λ😀", "\u{feff}text\0", "é", "e\u{301}"] {
        let b =
            RuntimeBytesV2::from_text(text, L, &mut fuel(text.len() as u64 + 1, 0), &mut work())
                .unwrap();
        assert_eq!(
            b.to_utf8(L, &mut fuel(1, text.len() as u64), &mut work())
                .unwrap(),
            text
        );
    }
    for raw in [
        &[128][..],
        &[192, 128],
        &[224, 128, 128],
        &[237, 160, 128],
        &[244, 144, 128, 128],
        &[240],
        &[226, 130],
        &[b'a', 255, b'b'],
    ] {
        let b = bytes(raw);
        let mut f = fuel(100, 100);
        let e = b.to_utf8(L, &mut f, &mut work()).unwrap_err();
        assert_eq!(e.kind, RuntimeDataErrorKind::Utf8);
        assert_eq!(e.offset, Some(if raw[0] == b'a' { 1 } else { 0 }));
        assert_eq!(f.remaining().max_value_nodes, Some(100));
        assert_eq!(f.remaining().max_text_bytes, Some(100));
    }
}
#[test]
fn full_result_node_text_and_work_boundaries_are_atomic() {
    let b = bytes(b"foo");
    let mut w = RuntimeDataWork::new(7);
    b.snapshot(L, &mut fuel(4, 0), &mut w).unwrap();
    assert_eq!(w.remaining(), 0);
    let mut f = fuel(4, 9);
    let e = b
        .snapshot(L, &mut f, &mut RuntimeDataWork::new(6))
        .unwrap_err();
    assert_eq!(e.kind, RuntimeDataErrorKind::WorkLimit);
    assert_eq!(f.remaining().max_value_nodes, Some(4));
    assert_eq!(f.remaining().max_text_bytes, Some(9));
    for text in [5, 6] {
        let mut f = fuel(1, text);
        let result = b.to_hex(L, &mut f, &mut work());
        assert_eq!(result.is_ok(), text == 6);
        if text == 5 {
            assert_eq!(f.remaining().max_value_nodes, Some(1));
            assert_eq!(f.remaining().max_text_bytes, Some(5));
        }
    }
    let mut w = RuntimeDataWork::new(13);
    RuntimeBytesV2::from_hex("666f6f", L, &mut fuel(4, 0), &mut w).unwrap();
    assert_eq!(w.remaining(), 0);
    let mut f = fuel(4, 0);
    assert!(RuntimeBytesV2::from_hex("666f6f", L, &mut f, &mut RuntimeDataWork::new(12)).is_err());
    assert_eq!(f.remaining().max_value_nodes, Some(4));
    let mut w = RuntimeDataWork::new(11);
    RuntimeBytesV2::from_base64("Zm9v", L, &mut fuel(4, 0), &mut w).unwrap();
    assert_eq!(w.remaining(), 0);
    let mut w = RuntimeDataWork::new(10);
    b.to_utf8(L, &mut fuel(1, 3), &mut w).unwrap();
    assert_eq!(w.remaining(), 0);
    let mut w = RuntimeDataWork::new(7);
    assert!(b.equal(&b, &mut w).unwrap());
    assert_eq!(w.remaining(), 0);
}
#[test]
fn finite_caller_caps_apply_before_growth_and_cannot_overflow() {
    let b = bytes(&[1, 2, 3]);
    let small = RuntimeBytesLimits {
        max_bytes: 2,
        max_text_bytes: 4,
    };
    let mut f = fuel(100, 100);
    assert_eq!(
        b.snapshot(small, &mut f, &mut work()).unwrap_err().kind,
        RuntimeDataErrorKind::Capacity
    );
    assert_eq!(f.remaining().max_value_nodes, Some(100));
    if usize::BITS > 32 {
        assert!(RuntimeBytesV2::from_slice(
            &[],
            RuntimeBytesLimits {
                max_bytes: usize::MAX,
                max_text_bytes: usize::MAX
            },
            &mut f,
            &mut work()
        )
        .is_err());
    }
    assert!(bytes(&[1, 2])
        .concat(&bytes(&[3]), small, &mut f, &mut work())
        .is_err());
    assert!(RuntimeBytesV2::from_hex("000102", small, &mut f, &mut work()).is_err());
    assert!(b.to_base64(small, &mut f, &mut work()).is_err());
    let zero = RuntimeBytesLimits {
        max_bytes: 0,
        max_text_bytes: 0,
    };
    assert!(
        RuntimeBytesV2::from_hex("", zero, &mut fuel(1, 0), &mut work())
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        bytes(&[])
            .to_base64(zero, &mut fuel(1, 0), &mut work())
            .unwrap(),
        ""
    );
}
#[test]
fn cancellation_and_zero_elapsed_refuse_without_partial_results() {
    let token = CancellationToken::new();
    token.cancel();
    let mut w = RuntimeDataWork::with_control(
        100,
        ExecutionControl {
            cancellation: Some(token),
            max_elapsed: None,
        },
    );
    let mut f = fuel(4, 0);
    let e = RuntimeBytesV2::from_slice(b"foo", L, &mut f, &mut w).unwrap_err();
    assert!(matches!(e.kind, RuntimeDataErrorKind::Control(_)));
    assert_eq!(w.remaining(), 100);
    assert_eq!(f.remaining().max_value_nodes, Some(4));
    let mut w = RuntimeDataWork::with_control(
        100,
        ExecutionControl {
            cancellation: None,
            max_elapsed: Some(std::time::Duration::ZERO),
        },
    );
    assert!(matches!(
        RuntimeBytesV2::from_slice(&[], L, &mut f, &mut w)
            .unwrap_err()
            .kind,
        RuntimeDataErrorKind::Control(_)
    ));
}
#[test]
fn every_one_and_two_byte_payload_roundtrips_hex_and_base64() {
    for n in 0..=u16::MAX {
        let data = n.to_be_bytes();
        let b = bytes(&data);
        let h = b.to_hex(L, &mut unlimited(), &mut work()).unwrap();
        assert_eq!(h, format!("{:04x}", n));
        assert_eq!(
            RuntimeBytesV2::from_hex(&h, L, &mut unlimited(), &mut work())
                .unwrap()
                .as_slice(),
            data
        );
        let text = b.to_base64(L, &mut unlimited(), &mut work()).unwrap();
        assert_eq!(
            RuntimeBytesV2::from_base64(&text, L, &mut unlimited(), &mut work())
                .unwrap()
                .as_slice(),
            data
        );
    }
    for n in 0..=u8::MAX {
        let b = bytes(&[n]);
        let text = b.to_base64(L, &mut unlimited(), &mut work()).unwrap();
        assert_eq!(
            RuntimeBytesV2::from_base64(&text, L, &mut unlimited(), &mut work())
                .unwrap()
                .as_slice(),
            [n]
        );
    }
}
#[test]
fn data_foundation_does_not_silently_add_source_types_or_builtin_registry_entries() {
    assert!(compile_generic_array_source(
        "fn unused(value:std::Bytes){}fn main(){}",
        "main",
        GenericPropagationSourceLimits::DEFAULT
    )
    .is_err());
    for name in [
        "std::byte_from_int",
        "std::bytes_from_hex",
        "std::bytes_from_base64",
    ] {
        assert!(sovra::compiler::stdlib::lookup(name).is_none());
    }
}

#[test]
fn accepted_short_codec_inputs_are_canonical_and_malformed_inputs_never_panic() {
    for length in 0..=4u32 {
        let alphabet = ["A", "B", "/", "=", " ", "-", "λ"];
        for mut code in 0..alphabet.len().pow(length) {
            let mut text = String::new();
            for _ in 0..length {
                text.push_str(alphabet[code % alphabet.len()]);
                code /= alphabet.len();
            }
            if let Ok(value) = RuntimeBytesV2::from_base64(&text, L, &mut unlimited(), &mut work())
            {
                assert_eq!(
                    value.to_base64(L, &mut unlimited(), &mut work()).unwrap(),
                    text
                );
            }
        }
        let alphabet = ["0", "f", "A", "g", "λ", " "];
        for mut code in 0..alphabet.len().pow(length) {
            let mut text = String::new();
            for _ in 0..length {
                text.push_str(alphabet[code % alphabet.len()]);
                code /= alphabet.len();
            }
            if let Ok(value) = RuntimeBytesV2::from_hex(&text, L, &mut unlimited(), &mut work()) {
                assert_eq!(
                    value.to_hex(L, &mut unlimited(), &mut work()).unwrap(),
                    text.to_ascii_lowercase()
                );
            }
        }
    }
    assert_eq!(
        bytes(&[0xce, 0xbb, 255])
            .to_utf8(L, &mut unlimited(), &mut work())
            .unwrap_err()
            .offset,
        Some(2)
    );
    assert_eq!(
        RuntimeBytesV2::from_hex("00λ", L, &mut unlimited(), &mut work())
            .unwrap_err()
            .offset,
        Some(2)
    );
}
#[test]
fn large_buffer_copy_boundaries_preserve_every_byte() {
    for n in [65_535, 65_536, 65_537] {
        let source = (0..n).map(|i| (i % 256) as u8).collect::<Vec<_>>();
        let b = bytes(&source);
        let copy = b
            .snapshot(L, &mut fuel(n as u64 + 1, 0), &mut work())
            .unwrap();
        assert_eq!(copy.as_slice(), source);
        let hex = copy
            .to_hex(L, &mut fuel(1, n as u64 * 2), &mut work())
            .unwrap();
        assert_eq!(
            RuntimeBytesV2::from_hex(&hex, L, &mut fuel(n as u64 + 1, 0), &mut work())
                .unwrap()
                .as_slice(),
            source
        );
    }
}
