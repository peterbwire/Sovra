//! Exact wire response sizing, no-content semantics and header injection refusal.
use sovra::compiler::v2::*;
fn render(
    status: u16,
    context: HttpResponseContext,
    fields: &[HttpResponseHeader<'_>],
    body: &[u8],
) -> Vec<u8> {
    render_http_response(status, context, fields, body, HttpResponseLimits::DEFAULT).unwrap()
}
fn parts(wire: &[u8]) -> (&str, &[u8]) {
    let end = wire
        .windows(4)
        .position(|part| part == b"\r\n\r\n")
        .unwrap()
        + 4;
    (std::str::from_utf8(&wire[..end]).unwrap(), &wire[end..])
}
#[test]
fn binary_body_and_generated_framing_have_exact_wire_bytes() {
    let fields = [
        HttpResponseHeader {
            name: "Content-Type",
            value: b"application/octet-stream",
        },
        HttpResponseHeader {
            name: "X-Empty",
            value: b"",
        },
    ];
    let wire = render(200, HttpResponseContext::Standard, &fields, b"\0\xff\n");
    assert_eq!(wire,b"HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nX-Empty: \r\nContent-Length: 3\r\nConnection: close\r\n\r\n\0\xff\n");
    let wire = render(404, HttpResponseContext::Standard, &[], b"missing");
    let (head, body) = parts(&wire);
    assert!(head.starts_with("HTTP/1.1 404 Not Found\r\n"));
    assert!(head.contains("Content-Length: 7\r\n"));
    assert_eq!(body, b"missing");
}
#[test]
fn head_uses_representation_length_without_emitting_representation_bytes() {
    let representation = b"\0\xffdata";
    let normal = render(200, HttpResponseContext::Standard, &[], representation);
    let head = render(200, HttpResponseContext::Head, &[], representation);
    assert_eq!(parts(&normal).0, parts(&head).0);
    assert!(parts(&head).1.is_empty());
    assert!(parts(&head).0.contains("Content-Length: 6\r\n"));
    let limits = HttpResponseLimits {
        output_bytes: head.len(),
        ..HttpResponseLimits::DEFAULT
    };
    assert!(
        render_http_response(200, HttpResponseContext::Head, &[], representation, limits).is_ok()
    );
    assert_eq!(
        render_http_response(
            200,
            HttpResponseContext::Standard,
            &[],
            representation,
            limits
        )
        .unwrap_err()
        .code,
        "limit"
    );
}
#[test]
fn no_content_and_not_modified_never_emit_forbidden_lengths_or_body() {
    for status in [204, 205, 304] {
        for context in [HttpResponseContext::Standard, HttpResponseContext::Head] {
            let wire = render(status, context, &[], b"");
            let (head, body) = parts(&wire);
            assert!(body.is_empty());
            assert!(head.contains("Connection: close\r\n"));
            assert_eq!(head.contains("Content-Length:"), status == 205);
            if status == 205 {
                assert!(head.contains("Content-Length: 0\r\n"));
            }
            assert_eq!(
                render_http_response(status, context, &[], b"x", HttpResponseLimits::DEFAULT)
                    .unwrap_err()
                    .code,
                "body"
            );
        }
    }
}
#[test]
fn final_status_range_and_static_reasons_cannot_inject_a_status_line() {
    for status in [0, 99, 600, 999, u16::MAX] {
        assert_eq!(
            render_http_response(
                status,
                HttpResponseContext::Standard,
                &[],
                b"",
                HttpResponseLimits::DEFAULT
            )
            .unwrap_err()
            .code,
            "status"
        );
    }
    for status in [100, 101, 103, 199] {
        assert_eq!(
            render_http_response(
                status,
                HttpResponseContext::Standard,
                &[],
                b"",
                HttpResponseLimits::DEFAULT
            )
            .unwrap_err()
            .code,
            "unsupported"
        );
    }
    for status in 200..=599 {
        let wire = render(status, HttpResponseContext::Standard, &[], b"");
        assert!(parts(&wire).0.starts_with(&format!("HTTP/1.1 {status} ")));
    }
    assert!(parts(&render(499, HttpResponseContext::Standard, &[], b""))
        .0
        .starts_with("HTTP/1.1 499 Status\r\n"));
}
#[test]
fn framing_headers_are_reserved_case_insensitively() {
    for name in [
        "Content-Length",
        "content-length",
        "cOnNeCtIoN",
        "Transfer-Encoding",
        "Trailer",
        "Upgrade",
        "Keep-Alive",
        "Proxy-Connection",
    ] {
        let error = render_http_response(
            200,
            HttpResponseContext::Standard,
            &[HttpResponseHeader { name, value: b"0" }],
            b"",
            HttpResponseLimits::DEFAULT,
        )
        .unwrap_err();
        assert_eq!(error.code, "framing");
        assert_eq!(error.header, Some(0));
    }
}
#[test]
fn header_name_and_value_injection_controls_and_surrounding_ows_refuse() {
    for name in [
        "",
        "bad name",
        "bad\tname",
        "x:y",
        "x\r\nInjected",
        "[bad]",
        "λ",
    ] {
        assert_eq!(
            render_http_response(
                200,
                HttpResponseContext::Standard,
                &[HttpResponseHeader { name, value: b"ok" }],
                b"",
                HttpResponseLimits::DEFAULT
            )
            .unwrap_err()
            .code,
            "syntax"
        );
    }
    for value in [
        b"\r\nContent-Length: 0".as_slice(),
        b"x\ny",
        b"x\0y",
        b"x\x7fy",
        b"\x01",
        b" leading",
        b"trailing ",
        b"\tleading",
        b"trailing\t",
    ] {
        let error = render_http_response(
            200,
            HttpResponseContext::Standard,
            &[HttpResponseHeader { name: "X", value }],
            b"",
            HttpResponseLimits::DEFAULT,
        )
        .unwrap_err();
        assert_eq!(error.code, "syntax");
        assert_eq!(error.header, Some(0));
        assert!(!error.message.contains("Content-Length"));
    }
    let wire = render(
        200,
        HttpResponseContext::Standard,
        &[HttpResponseHeader {
            name: "X-Bytes",
            value: b"\xff\x80\tinside",
        }],
        b"",
    );
    assert!(wire
        .windows(b"\xff\x80\tinside\r\n".len())
        .any(|w| w == b"\xff\x80\tinside\r\n"));
}
#[test]
fn set_cookie_repetitions_preserve_order_other_duplicates_refuse() {
    let fields = [
        HttpResponseHeader {
            name: "Set-Cookie",
            value: b"a=1; HttpOnly",
        },
        HttpResponseHeader {
            name: "set-cookie",
            value: b"b=2; Secure",
        },
    ];
    let wire = render(200, HttpResponseContext::Standard, &fields, b"");
    assert!(parts(&wire)
        .0
        .contains("Set-Cookie: a=1; HttpOnly\r\nset-cookie: b=2; Secure\r\n"));
    let duplicate = [
        HttpResponseHeader {
            name: "Content-Type",
            value: b"application/json",
        },
        HttpResponseHeader {
            name: "content-type",
            value: b"text/html",
        },
    ];
    let error = render_http_response(
        200,
        HttpResponseContext::Standard,
        &duplicate,
        b"",
        HttpResponseLimits::DEFAULT,
    )
    .unwrap_err();
    assert_eq!(error.code, "framing");
    assert_eq!(error.header, Some(1));
}
#[test]
fn exact_head_line_count_body_output_and_work_caps_are_enforced() {
    let wire = render(200, HttpResponseContext::Standard, &[], b"abc");
    let head = parts(&wire).0.len();
    let exact = HttpResponseLimits {
        head_bytes: head,
        line_bytes: 17,
        headers: 2,
        body_bytes: 3,
        output_bytes: wire.len(),
        work: wire.len(),
    };
    assert_eq!(
        render_http_response(200, HttpResponseContext::Standard, &[], b"abc", exact).unwrap(),
        wire
    );
    for limits in [
        HttpResponseLimits {
            head_bytes: head - 1,
            ..exact
        },
        HttpResponseLimits {
            line_bytes: 16,
            ..exact
        },
        HttpResponseLimits {
            headers: 1,
            ..exact
        },
        HttpResponseLimits {
            body_bytes: 2,
            ..exact
        },
        HttpResponseLimits {
            output_bytes: wire.len() - 1,
            ..exact
        },
        HttpResponseLimits {
            work: wire.len() - 1,
            ..exact
        },
    ] {
        assert_eq!(
            render_http_response(200, HttpResponseContext::Standard, &[], b"abc", limits)
                .unwrap_err()
                .code,
            "limit"
        );
    }
}
#[test]
fn widened_limits_refuse_before_rendering() {
    let d = HttpResponseLimits::DEFAULT;
    for limits in [
        HttpResponseLimits {
            head_bytes: d.head_bytes + 1,
            ..d
        },
        HttpResponseLimits {
            line_bytes: d.line_bytes + 1,
            ..d
        },
        HttpResponseLimits {
            headers: d.headers + 1,
            ..d
        },
        HttpResponseLimits {
            body_bytes: d.body_bytes + 1,
            ..d
        },
        HttpResponseLimits {
            output_bytes: d.output_bytes + 1,
            ..d
        },
        HttpResponseLimits {
            work: d.work + 1,
            ..d
        },
    ] {
        assert_eq!(
            render_http_response(200, HttpResponseContext::Standard, &[], b"", limits)
                .unwrap_err()
                .code,
            "policy"
        );
    }
}
#[test]
fn malformed_value_byte_mutations_and_maximum_representation_remain_bounded() {
    for byte in 0..=255 {
        let value = [b'a', byte, b'b'];
        let result = render_http_response(
            200,
            HttpResponseContext::Standard,
            &[HttpResponseHeader {
                name: "X",
                value: &value,
            }],
            b"",
            HttpResponseLimits::DEFAULT,
        );
        if (byte < 32 && byte != b'\t') || byte == 127 {
            assert_eq!(result.unwrap_err().code, "syntax");
        } else {
            assert!(result.is_ok());
        }
    }
    let body = vec![255; 1024 * 1024];
    let wire = render(200, HttpResponseContext::Standard, &[], &body);
    assert_eq!(parts(&wire).1, body);
    assert!(parts(&wire).0.contains("Content-Length: 1048576\r\n"));
    assert_eq!(
        render_http_response(
            200,
            HttpResponseContext::Head,
            &[],
            &vec![0; 1024 * 1024 + 1],
            HttpResponseLimits::DEFAULT
        )
        .unwrap_err()
        .code,
        "limit"
    );
}

#[test]
fn custom_field_and_generated_field_boundaries_are_counted_together() {
    let fields = vec![
        HttpResponseHeader {
            name: "Set-Cookie",
            value: b"a=1"
        };
        62
    ];
    assert!(render_http_response(
        200,
        HttpResponseContext::Standard,
        &fields,
        b"",
        HttpResponseLimits::DEFAULT
    )
    .is_ok());
    let fields = vec![
        HttpResponseHeader {
            name: "Set-Cookie",
            value: b"a=1"
        };
        63
    ];
    assert_eq!(
        render_http_response(
            200,
            HttpResponseContext::Standard,
            &fields,
            b"",
            HttpResponseLimits::DEFAULT
        )
        .unwrap_err()
        .code,
        "limit"
    );
    assert!(render_http_response(
        204,
        HttpResponseContext::Standard,
        &fields,
        b"",
        HttpResponseLimits::DEFAULT
    )
    .is_ok());
    let value = vec![b'a'; 8189];
    let fields = [HttpResponseHeader {
        name: "X",
        value: &value,
    }];
    let wire = render(200, HttpResponseContext::Standard, &fields, b"");
    let limits = HttpResponseLimits {
        head_bytes: wire.len(),
        headers: 3,
        body_bytes: 0,
        output_bytes: wire.len(),
        ..HttpResponseLimits::DEFAULT
    };
    assert_eq!(
        render_http_response(200, HttpResponseContext::Standard, &fields, b"", limits).unwrap(),
        wire
    );
    assert_eq!(
        render_http_response(
            200,
            HttpResponseContext::Standard,
            &fields,
            b"",
            HttpResponseLimits {
                head_bytes: wire.len() - 1,
                ..limits
            }
        )
        .unwrap_err()
        .code,
        "limit"
    );
    assert_eq!(
        render_http_response(
            200,
            HttpResponseContext::Standard,
            &fields,
            b"",
            HttpResponseLimits {
                line_bytes: 8191,
                ..limits
            }
        )
        .unwrap_err()
        .code,
        "limit"
    );
}
