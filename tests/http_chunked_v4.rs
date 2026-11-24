//! Incremental chunk grammar, refusal, fragmentation and aggregate admission regressions.
use sovra::compiler::v2::{HttpChunkedDecoder as Decoder, HttpChunkedLimits as Limits};
fn decode(wire: &[u8]) -> Result<Vec<u8>, &'static str> {
    let mut d = Decoder::new(Limits::DEFAULT).unwrap();
    d.feed(wire).map_err(|e| e.code)?;
    d.finish().map_err(|e| e.code)
}
#[test]
fn binary_chunks_extensions_and_zero_forms() {
    assert_eq!(decode(b"3;name=value; flag; quoted=\"a\\\"b\"\r\na\0\xff\r\n2 \t; x = token\r\n\r\n\r\n000;end\r\n\r\n").unwrap(), b"a\0\xff\r\n");
    assert_eq!(decode(b"0\r\n\r\n").unwrap(), b"");
    assert_eq!(
        decode(b"A\r\n0123456789\r\n0\r\n\r\n").unwrap(),
        b"0123456789"
    );
    assert_eq!(
        decode(b"1;q=\"\xff\t\\\x80\"\r\nx\r\n0\r\n\r\n").unwrap(),
        b"x"
    );
}
#[test]
fn every_split_and_one_byte_feeds_preserve_output() {
    let wire = b"3;x=\"test\"\r\na\0b\r\n2\r\ncd\r\n0\r\n\r\n";
    for split in 0..=wire.len() {
        let mut d = Decoder::new(Limits::DEFAULT).unwrap();
        assert_eq!(d.feed(&wire[..split]).unwrap(), split == wire.len());
        assert!(d.feed(&wire[split..]).unwrap());
        assert_eq!(d.finish().unwrap(), b"a\0bcd");
    }
    let mut d = Decoder::new(Limits::DEFAULT).unwrap();
    for (i, byte) in wire.iter().enumerate() {
        assert_eq!(d.feed(&[*byte]).unwrap(), i + 1 == wire.len());
    }
    assert_eq!(d.finish().unwrap(), b"a\0bcd");
}
#[test]
fn every_incomplete_prefix_is_truncated() {
    let wire = b"2;foo=bar\r\nab\r\n0\r\n\r\n";
    for end in 0..wire.len() {
        let mut d = Decoder::new(Limits::DEFAULT).unwrap();
        assert!(!d.feed(&wire[..end]).unwrap());
        let e = d.finish().unwrap_err();
        assert_eq!(e.code, "truncated");
        assert_eq!(e.offset, end);
    }
}
#[test]
fn malformed_sizes_extensions_and_delimiters_refuse() {
    for wire in [
        &b"\r\n"[..],
        b"+1\r\nx\r\n0\r\n\r\n",
        b"0x1\r\n",
        b" 1\r\n",
        b"1 \r\n",
        b"1;\r\n",
        b"1;x=\r\n",
        b"1;x=\"\r\n",
        b"1;x=\"a\x7f\"\r\n",
        b"1;x=\"a\\\0\"\r\n",
        b"1;x=a \r\n",
        b"1;x=\"a\"z\r\n",
        b"1\nx",
        b"1\rx",
        b"1\r\nxX",
        b"1\r\nx\rX",
        b"0\r\n\rX",
        b"FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF\r\n",
    ] {
        assert!(decode(wire).is_err(), "{wire:?}");
    }
}
#[test]
fn trailers_and_trailing_bytes_never_become_body_or_second_message() {
    assert_eq!(
        decode(b"0\r\nContent-Length: 9\r\n\r\n"),
        Err("unsupported")
    );
    assert_eq!(decode(b"0\r\n\r\nx"), Err("framing"));
    let mut d = Decoder::new(Limits::DEFAULT).unwrap();
    assert!(d.feed(b"1\r\nx\r\n0\r\n\r\n").unwrap());
    let first = d.feed(b"GET / HTTP/1.1\r\n").unwrap_err();
    assert_eq!(first.code, "framing");
    assert_eq!(d.feed(b"").unwrap_err(), first);
    assert_eq!(d.finish().unwrap_err(), first);
}
#[test]
fn exact_limits_and_one_below_refuse_before_publication() {
    let wire = b"1\r\na\r\n0\r\n\r\n";
    // Work: each wire byte plus four times each size/extension line length.
    let exact = Limits {
        wire_bytes: wire.len(),
        body_bytes: 1,
        line_bytes: 1,
        chunks: 1,
        work: wire.len() + 8,
    };
    let mut d = Decoder::new(exact).unwrap();
    assert!(d.feed(wire).unwrap());
    assert_eq!(d.finish().unwrap(), b"a");
    for limits in [
        Limits {
            wire_bytes: exact.wire_bytes - 1,
            ..exact
        },
        Limits {
            body_bytes: 0,
            ..exact
        },
        Limits {
            line_bytes: 0,
            ..exact
        },
        Limits { chunks: 0, ..exact },
        Limits {
            work: exact.work - 1,
            ..exact
        },
    ] {
        let mut d = Decoder::new(limits).unwrap();
        assert_eq!(d.feed(wire).unwrap_err().code, "limit");
        assert_eq!(d.finish().unwrap_err().code, "limit");
    }
    for limits in [
        Limits {
            wire_bytes: Limits::DEFAULT.wire_bytes + 1,
            ..Limits::DEFAULT
        },
        Limits {
            body_bytes: Limits::DEFAULT.body_bytes + 1,
            ..Limits::DEFAULT
        },
        Limits {
            line_bytes: Limits::DEFAULT.line_bytes + 1,
            ..Limits::DEFAULT
        },
        Limits {
            chunks: Limits::DEFAULT.chunks + 1,
            ..Limits::DEFAULT
        },
        Limits {
            work: Limits::DEFAULT.work + 1,
            ..Limits::DEFAULT
        },
    ] {
        assert_eq!(Decoder::new(limits).unwrap_err().code, "policy");
    }
}
#[test]
fn limits_and_errors_are_fragmentation_independent() {
    let wire = b"1;one=two\r\nx\r\n1\r\ny\r\n0\r\n\r\n";
    for work in 0..100 {
        let limits = Limits {
            work,
            ..Limits::DEFAULT
        };
        let mut whole = Decoder::new(limits).unwrap();
        let expected = whole.feed(wire).err();
        let mut split = Decoder::new(limits).unwrap();
        let mut got = None;
        for b in wire {
            if let Err(e) = split.feed(&[*b]) {
                got = Some(e);
                break;
            }
        }
        assert_eq!(got, expected);
    }
}
#[test]
fn many_tiny_chunks_count_and_aggregate_body_admission() {
    let mut wire = b"1\r\nx\r\n".repeat(4096);
    wire.extend_from_slice(b"0\r\n\r\n");
    assert_eq!(decode(&wire).unwrap(), vec![b'x'; 4096]);
    wire.splice(0..0, b"1\r\nx\r\n".iter().copied());
    assert_eq!(decode(&wire), Err("limit"));
    let mut d = Decoder::new(Limits {
        body_bytes: 1,
        ..Limits::DEFAULT
    })
    .unwrap();
    assert_eq!(d.feed(b"1\r\na\r\n1\r\n").unwrap_err().code, "limit");
}
#[test]
fn deterministic_byte_mutations_never_panic_or_publish_unbounded_body() {
    let wire = b"2;ext=\"v\"\r\nab\r\n0\r\n\r\n";
    for at in 0..wire.len() {
        for byte in 0..=255 {
            let mut changed = wire.to_vec();
            changed[at] = byte;
            let mut d = Decoder::new(Limits::DEFAULT).unwrap();
            if d.feed(&changed).is_ok() {
                if let Ok(body) = d.finish() {
                    assert!(body.len() <= changed.len());
                }
            }
        }
    }
}
