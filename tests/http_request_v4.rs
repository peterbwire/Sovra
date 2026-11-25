//! Byte-exact initial HTTP framing, malformed refusal and finite admission.
use sovra::compiler::v2::*;
fn parse(input: &[u8]) -> HttpRequest<'_> {
    match parse_http_request(input, HttpRequestLimits::DEFAULT, true).unwrap() {
        HttpRequestParse::Complete(request) => request,
        _ => panic!("incomplete request"),
    }
}
fn refuses(input: &[u8]) -> HttpRequestError {
    parse_http_request(input, HttpRequestLimits::DEFAULT, true).unwrap_err()
}
#[test]
fn borrowed_binary_body_headers_and_raw_target_preserve_original_bytes() {
    let wire=b"POST /a%2Fb?x=%00 HTTP/1.1\r\nHost: LocalHost:8080\r\nContent-Length: 4\r\nX-Bytes: \t\xff\x80 \t\r\nX-Repeat: one\r\nx-repeat: two\r\n\r\n\0\xff\n\r";
    let request = parse(wire);
    assert_eq!(request.method(), "POST");
    assert_eq!(request.target(), "/a%2Fb?x=%00");
    assert_eq!(request.body(), b"\0\xff\n\r");
    assert_eq!(request.body().as_ptr(), wire[wire.len() - 4..].as_ptr());
    assert_eq!(request.headers()[0].name(), "Host");
    assert_eq!(request.headers()[0].value(), b"LocalHost:8080");
    assert_eq!(request.headers()[2].value(), b"\xff\x80");
    assert_eq!(request.headers().len(), 5);
    assert_eq!(
        parse(b"OPTIONS * HTTP/1.1\r\nHost: [::1]:80\r\n\r\n").target(),
        "*"
    );
    assert!(parse(b"GET / HTTP/1.1\r\nHost: example.org.\r\n\r\n")
        .body()
        .is_empty());
}
#[test]
fn every_partial_boundary_is_incomplete_and_eof_is_truncated() {
    let wire = b"POST / HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer private-head-fixture\r\nContent-Length: 3\r\n\r\nabc";
    let head = wire.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
    for end in 0..wire.len() {
        match parse_http_request(&wire[..end], HttpRequestLimits::DEFAULT, false).unwrap() {
            HttpRequestParse::Incomplete { minimum_bytes } => assert_eq!(
                minimum_bytes,
                if end >= head { Some(wire.len()) } else { None }
            ),
            _ => panic!("published partial request at {end}"),
        }
        let error = refuses(&wire[..end]);
        assert_eq!(error.code, "truncated", "{end}: {error:?}");
        assert!(error.offset <= end);
    }
    assert_eq!(parse(wire).body(), b"abc");
}
#[test]
fn ambiguous_lengths_and_pipelined_or_unframed_bodies_refuse() {
    for wire in [
 "POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 1\r\nContent-Length: 1\r\n\r\na",
 "POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 1\r\ncOnTeNt-LeNgTh: 2\r\n\r\na",
 "POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 1, 1\r\n\r\na",
 "POST / HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\nContent-Length: 0\r\n\r\n",
 "POST / HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\ntransfer-encoding: identity\r\n\r\n",
 "GET / HTTP/1.1\r\nHost: localhost\r\n\r\nx",
 "GET / HTTP/1.1\r\nHost: localhost\r\n\r\nGET / HTTP/1.1\r\nHost: localhost\r\n\r\n",
 ] {assert_eq!(refuses(wire.as_bytes()).code,"framing","{wire:?}");}
    for length in [
        "",
        "+1",
        "-1",
        "1 0",
        "0x10",
        "1844674407370955161600000000000000000000",
    ] {
        assert_eq!(
            refuses(
                format!("POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: {length}\r\n\r\n")
                    .as_bytes()
            )
            .code,
            "framing"
        );
    }
    assert_eq!(
        parse(b"POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0001\r\n\r\na").body(),
        b"a"
    );
}
#[test]
fn whitespace_controls_folding_and_invalid_names_never_normalize_into_valid_fields() {
    for wire in [
        b"GET / HTTP/1.1\nHost: localhost\n\n".as_slice(),
        b"GET / HTTP/1.1\rXHost: localhost\r\n\r\n",
        b"GET  / HTTP/1.1\r\nHost: localhost\r\n\r\n",
        b"GET\t/ HTTP/1.1\r\nHost: localhost\r\n\r\n",
        b"GET / HTTP/1.1 \r\nHost: localhost\r\n\r\n",
        b"\r\nGET / HTTP/1.1\r\nHost: localhost\r\n\r\n",
        b"GET / HTTP/1.1\r\nHost : localhost\r\n\r\n",
        b"GET / HTTP/1.1\r\nHost: localhost\r\n folded: value\r\n\r\n",
        b"GET / HTTP/1.1\r\nHost: localhost\r\nX: a\0b\r\n\r\n",
        b"GET / HTTP/1.1\r\nHost: localhost\r\nX: a\x7fb\r\n\r\n",
        b"GET / HTTP/1.1\r\nHost: localhost\r\nBad(name): value\r\n\r\n",
    ] {
        let error = refuses(wire);
        assert_eq!(error.code, "syntax", "{wire:?}");
        assert!(error.offset <= wire.len());
    }
}
#[test]
fn host_cardinality_and_restricted_authority_syntax_are_checked() {
    assert_eq!(refuses(b"GET / HTTP/1.1\r\n\r\n").code, "framing");
    assert_eq!(
        refuses(b"GET / HTTP/1.1\r\nHost: localhost\r\nhOst: localhost\r\n\r\n").code,
        "framing"
    );
    for host in [
        "",
        "user@localhost",
        "local host",
        "localhost/path",
        "localhost,evil",
        "localhost:",
        "localhost:65536",
        "localhost:-1",
        "a..b",
        "-bad.example",
        "bad-.example",
        "[::1",
        "[xyz]",
        "[::1]x",
        "::1",
        "localhost:1:2",
        "%61.example",
    ] {
        assert_eq!(
            refuses(format!("GET / HTTP/1.1\r\nHost: {host}\r\n\r\n").as_bytes()).code,
            "syntax",
            "{host:?}"
        );
    }
    for host in [
        "localhost",
        "127.0.0.1:0",
        "example.org.",
        "xn--bcher-kva.example",
        "[::1]",
        "[2001:db8::1]:65535",
    ] {
        parse(format!("GET / HTTP/1.1\r\nHost: {host}\r\n\r\n").as_bytes());
    }
}
#[test]
fn unsupported_protocol_contexts_and_invalid_targets_refuse_explicitly() {
    for field in [
        "Transfer-Encoding: chunked",
        "Transfer-Encoding: identity",
        "Expect: 100-continue",
        "Upgrade: websocket",
        "Trailer: Checksum",
    ] {
        assert_eq!(
            refuses(format!("POST / HTTP/1.1\r\nHost: localhost\r\n{field}\r\n\r\n").as_bytes())
                .code,
            "unsupported"
        );
    }
    for request in [
        "GET / HTTP/1.0",
        "GET / HTTP/2.0",
        "CONNECT localhost:80 HTTP/1.1",
        "GET http://localhost/ HTTP/1.1",
        "GET https://localhost/ HTTP/1.1",
    ] {
        assert_eq!(
            refuses(format!("{request}\r\nHost: localhost\r\n\r\n").as_bytes()).code,
            "unsupported"
        );
    }
    for target in [
        "*",
        "/a#fragment",
        "/bad%",
        "/bad%0g",
        "/a\\b",
        "/raw[bracket]",
        "/white\tspace",
        "/λ",
    ] {
        assert_eq!(
            refuses(format!("GET {target} HTTP/1.1\r\nHost: localhost\r\n\r\n").as_bytes()).code,
            "syntax"
        );
    }
}
#[test]
fn lowered_byte_field_body_and_work_limits_are_exact() {
    let wire = b"POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 2\r\n\r\nab";
    let head = wire.len() - 2;
    let exact = HttpRequestLimits {
        head_bytes: head,
        line_bytes: 17,
        headers: 2,
        body_bytes: 2,
        ..HttpRequestLimits::DEFAULT
    };
    assert!(matches!(
        parse_http_request(wire, exact, true).unwrap(),
        HttpRequestParse::Complete(_)
    ));
    for limits in [
        HttpRequestLimits {
            head_bytes: head - 1,
            ..exact
        },
        HttpRequestLimits {
            line_bytes: 16,
            ..exact
        },
        HttpRequestLimits {
            headers: 1,
            ..exact
        },
        HttpRequestLimits {
            body_bytes: 1,
            ..exact
        },
        HttpRequestLimits { work: 1, ..exact },
    ] {
        assert_eq!(
            parse_http_request(wire, limits, true).unwrap_err().code,
            "limit"
        );
    }
    for limits in [
        HttpRequestLimits {
            head_bytes: 16385,
            ..exact
        },
        HttpRequestLimits {
            line_bytes: 8193,
            ..exact
        },
        HttpRequestLimits {
            headers: 65,
            ..exact
        },
        HttpRequestLimits {
            body_bytes: 1024 * 1024 + 1,
            ..exact
        },
        HttpRequestLimits {
            work: 2 * 1024 * 1024 + 1,
            ..exact
        },
    ] {
        assert_eq!(
            parse_http_request(wire, limits, true).unwrap_err().code,
            "policy"
        );
    }
}
#[test]
fn malformed_byte_mutations_and_long_partial_lines_remain_bounded() {
    let source = b"POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 1\r\n\r\nx";
    for at in 0..source.len() {
        for byte in [0, b'\r', b'\n', b' ', b'\t', b':', 127, 128, 255] {
            let mut wire = source.to_vec();
            wire[at] = byte;
            match parse_http_request(&wire, HttpRequestLimits::DEFAULT, true) {
                Ok(HttpRequestParse::Complete(request)) => assert!(request.body().len() <= 1),
                Ok(HttpRequestParse::Incomplete { .. }) => panic!("EOF published incomplete"),
                Err(error) => assert!(error.offset <= wire.len()),
            }
        }
    }
    let partial = vec![b'A'; 8193];
    assert_eq!(
        parse_http_request(&partial, HttpRequestLimits::DEFAULT, false)
            .unwrap_err()
            .code,
        "limit"
    );
}

#[test]
fn default_size_boundaries_and_empty_opaque_fields_are_supported() {
    let target = format!("/{}", "a".repeat(8178));
    let wire = format!("GET {target} HTTP/1.1\r\nHost: localhost\r\nX-Empty: \t \t\r\n\r\n");
    let request = parse(wire.as_bytes());
    assert_eq!(request.target(), target);
    assert_eq!(request.headers()[1].value(), b"");
    let mut wire = b"GET / HTTP/1.1\r\nHost: localhost\r\n".to_vec();
    for _ in 0..63 {
        wire.extend_from_slice(b"X: value\r\n");
    }
    let mut exact = wire.clone();
    exact.extend_from_slice(b"\r\n");
    assert_eq!(parse(&exact).headers().len(), 64);
    wire.extend_from_slice(b"X: too-many\r\n\r\n");
    assert_eq!(refuses(&wire).code, "limit");
    let mut body =
        b"POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 1048576\r\n\r\n".to_vec();
    body.resize(body.len() + 1024 * 1024, 0xff);
    assert_eq!(parse(&body).body().len(), 1024 * 1024);
    assert_eq!(
        refuses(b"POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 1048577\r\n\r\n").code,
        "limit"
    );
}

#[test]
fn resumable_head_all_fragmentations_preserve_body_prefix_and_parser_result() {
    let wire = b"POST / HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer private-head-fixture\r\nContent-Length: 3\r\n\r\nabc";
    for split in 0..wire.len() - 3 {
        let mut pending = HttpHeadBuffer::new(HttpRequestLimits::DEFAULT).unwrap();
        assert!(!pending.feed(&wire[..split]).unwrap());
        assert!(pending.feed(&wire[split..]).unwrap());
        assert!(!format!("{pending:?}").contains("private-head-fixture"));
        let bytes = pending.into_bytes();
        assert_eq!(bytes, wire);
        assert_eq!(parse(&bytes).body(), b"abc");
    }
    let mut pending = HttpHeadBuffer::new(HttpRequestLimits::DEFAULT).unwrap();
    for byte in &wire[..wire.len() - 3] {
        pending.feed(&[*byte]).unwrap();
    }
    assert!(pending.complete());
    assert_eq!(pending.into_bytes(), &wire[..wire.len() - 3]);
}

#[test]
fn resumable_head_limits_and_terminal_errors_are_sticky_without_prefix_copy() {
    let limits = HttpRequestLimits {
        head_bytes: 4,
        ..HttpRequestLimits::DEFAULT
    };
    let mut pending = HttpHeadBuffer::new(limits).unwrap();
    assert!(!pending.feed(b"\r\n").unwrap());
    let error = pending.feed(b"\r\nx").unwrap_err();
    assert_eq!(error.code, "limit");
    assert_eq!(pending.remaining(), 2);
    assert_eq!(pending.feed(b"\r\n").unwrap_err(), error);
    assert_eq!(pending.into_bytes(), b"\r\n");
    let mut pending = HttpHeadBuffer::new(limits).unwrap();
    assert!(pending.feed(b"\r\n\r\n").unwrap());
    assert_eq!(pending.remaining(), 0);
    assert_eq!(pending.feed(b"").unwrap_err().code, "input");
    let mut zero = HttpHeadBuffer::new(HttpRequestLimits {
        head_bytes: 0,
        ..limits
    })
    .unwrap();
    assert!(!zero.feed(b"").unwrap());
    assert_eq!(zero.feed(b"x").unwrap_err().code, "limit");
}

#[test]
fn fixed_body_resumes_all_splits_without_text_conversion_or_debug_disclosure() {
    let wire = b"private-body-fixture\0\xff";
    for split in 0..=wire.len() {
        let mut body = HttpFixedBody::new(wire.len(), HttpRequestLimits::DEFAULT).unwrap();
        assert_eq!(body.feed(&wire[..split]).unwrap(), split == wire.len());
        assert_eq!(body.remaining(), wire.len() - split);
        assert!(body.feed(&wire[split..]).unwrap());
        assert!(!format!("{body:?}").contains("private-body-fixture"));
        assert_eq!(body.finish().unwrap(), wire);
    }
}

#[test]
fn fixed_body_refuses_excess_atomically_and_never_publishes_partial_payload() {
    let mut body = HttpFixedBody::new(3, HttpRequestLimits::DEFAULT).unwrap();
    assert!(!body.feed(b"a").unwrap());
    let error = body.feed(b"bcd").unwrap_err();
    assert_eq!(error.code, "framing");
    assert_eq!(body.remaining(), 2);
    assert_eq!(body.feed(b"bc").unwrap_err(), error);
    assert_eq!(body.finish().unwrap_err(), error);
    let mut truncated = HttpFixedBody::new(3, HttpRequestLimits::DEFAULT).unwrap();
    truncated.feed(b"ab").unwrap();
    assert_eq!(truncated.finish().unwrap_err().code, "truncated");
}

#[test]
fn fixed_body_exact_and_zero_limits_are_independent_of_fragmentation() {
    let limits = HttpRequestLimits {
        body_bytes: 3,
        ..HttpRequestLimits::DEFAULT
    };
    assert_eq!(HttpFixedBody::new(4, limits).unwrap_err().code, "limit");
    let mut body = HttpFixedBody::new(3, limits).unwrap();
    for byte in b"abc" {
        body.feed(&[*byte]).unwrap();
    }
    assert_eq!(body.finish().unwrap(), b"abc");
    let mut empty = HttpFixedBody::new(
        0,
        HttpRequestLimits {
            body_bytes: 0,
            ..limits
        },
    )
    .unwrap();
    assert!(empty.feed(b"").unwrap());
    assert_eq!(empty.finish().unwrap(), b"");
}
