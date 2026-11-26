//! Explicit discarded-trailer decoding: grammar, limits, fragmentation and refusal.
use sovra::compiler::v2::{
    HttpChunkedDecoder as Decoder, HttpChunkedLimits as Body, HttpTrailerLimits as Trailers,
};
fn decoder() -> Decoder {
    Decoder::with_discarded_trailers(Body::DEFAULT, Trailers::DEFAULT).unwrap()
}
#[test]
fn every_split_preserves_body_and_discards_trailers() {
    let wire =
        b"2\r\n\0\xff\r\n0\r\nX-Trace: one\r\nContent-Digest: opaque\r\nX-Trace:\t\xff\x80\r\n\r\n";
    for at in 0..=wire.len() {
        let mut d = decoder();
        assert_eq!(d.feed(&wire[..at]).unwrap(), at == wire.len());
        assert!(d.feed(&wire[at..]).unwrap());
        assert_eq!(d.finish().unwrap(), b"\0\xff");
    }
    let mut d = decoder();
    for byte in wire {
        d.feed(&[*byte]).unwrap();
    }
    assert_eq!(d.finish().unwrap(), b"\0\xff");
}
#[test]
fn every_truncated_trailer_prefix_refuses_body_publication() {
    let wire = b"0\r\nX-Trace: value\r\n\r\n";
    for at in 0..wire.len() {
        let mut d = decoder();
        assert!(!d.feed(&wire[..at]).unwrap());
        let e = d.finish().unwrap_err();
        assert_eq!(e.code, "truncated");
        assert_eq!(e.offset, at);
    }
}
#[test]
fn malformed_fields_and_known_security_fields_are_sticky_errors() {
    for field in [
        " Content: x",
        "X : x",
        ": x",
        "X",
        "X: a\0b",
        "X: a\u{7f}",
        "Host: other",
        "cOnTeNt-LeNgTh: 2",
        "Transfer-Encoding: chunked",
        "Authorization: secret",
        "Cookie: auth",
        "Content-Type: text/plain",
        "Connection: keep-alive",
        "Trailer: X",
        "Range: bytes=0-1",
    ] {
        let mut d = decoder();
        let wire = format!("0\r\n{field}\r\n\r\n");
        let first = d.feed(wire.as_bytes()).unwrap_err();
        assert!(
            matches!(first.code, "syntax" | "unsupported"),
            "{field:?}: {first:?}"
        );
        assert!(first.offset <= wire.len());
        assert_eq!(d.feed(b"").unwrap_err(), first);
        assert_eq!(d.finish().unwrap_err(), first);
    }
    for wire in [
        b"0\r\nX: a\nb\r\n\r\n".as_slice(),
        b"0\r\nX: a\rX",
        b"0\r\n\r\nx",
    ] {
        let mut d = decoder();
        assert!(d.feed(wire).is_err());
        assert!(d.finish().is_err());
    }
}
#[test]
fn exact_and_lowered_section_line_field_and_work_limits() {
    let wire = b"0\r\nX: y\r\n\r\n";
    let body = Body {
        work: wire.len() + 4 + 4 * 32,
        ..Body::DEFAULT
    };
    let trailers = Trailers {
        bytes: 8,
        line_bytes: 4,
        fields: 1,
    };
    let mut d = Decoder::with_discarded_trailers(body, trailers).unwrap();
    assert!(d.feed(wire).unwrap());
    assert!(d.finish().unwrap().is_empty());
    for (b, t) in [
        (
            body,
            Trailers {
                bytes: 7,
                ..trailers
            },
        ),
        (
            body,
            Trailers {
                line_bytes: 3,
                ..trailers
            },
        ),
        (
            body,
            Trailers {
                fields: 0,
                ..trailers
            },
        ),
        (
            Body {
                work: body.work - 1,
                ..body
            },
            trailers,
        ),
    ] {
        let mut d = Decoder::with_discarded_trailers(b, t).unwrap();
        assert_eq!(d.feed(wire).unwrap_err().code, "limit");
        assert_eq!(d.finish().unwrap_err().code, "limit");
    }
    for t in [
        Trailers {
            bytes: 16385,
            ..trailers
        },
        Trailers {
            line_bytes: 8193,
            ..trailers
        },
        Trailers {
            fields: 65,
            ..trailers
        },
    ] {
        assert_eq!(
            Decoder::with_discarded_trailers(body, t).unwrap_err().code,
            "policy"
        );
    }
    let mut d = Decoder::with_discarded_trailers(
        Body::DEFAULT,
        Trailers {
            bytes: 2,
            line_bytes: 0,
            fields: 0,
        },
    )
    .unwrap();
    assert!(d.feed(b"0\r\n\r\n").unwrap());
}
#[test]
fn trailer_work_and_offsets_do_not_reset_with_feeds() {
    let wire = b"1\r\nx\r\n0\r\nX-Trace: y\r\n\r\n";
    for work in 0..450 {
        let b = Body {
            work,
            ..Body::DEFAULT
        };
        let mut whole = Decoder::with_discarded_trailers(b, Trailers::DEFAULT).unwrap();
        let expected = whole.feed(wire).err();
        let mut parts = Decoder::with_discarded_trailers(b, Trailers::DEFAULT).unwrap();
        let mut got = None;
        for byte in wire {
            if let Err(e) = parts.feed(&[*byte]) {
                got = Some(e);
                break;
            }
        }
        assert_eq!(expected, got);
    }
}
#[test]
fn deterministic_mutations_cannot_publish_partial_or_unbounded_body() {
    let wire = b"1\r\nx\r\n0\r\nX: value\r\n\r\n";
    for at in 0..wire.len() {
        for byte in 0..=255 {
            let mut changed = wire.to_vec();
            changed[at] = byte;
            let mut d = decoder();
            if d.feed(&changed).is_ok() {
                if let Ok(body) = d.finish() {
                    assert!(body.len() <= changed.len());
                }
            }
        }
    }
}
