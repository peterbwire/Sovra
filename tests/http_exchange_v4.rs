//! Real sockets verify one-request HTTP custody, bounded framing and cancellation.
#![cfg(target_os = "linux")]
use sovra::compiler::v2::*;
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, Shutdown, TcpStream},
    time::Duration,
};
fn request(text: &str) -> JsonValue {
    parse_json(text.as_bytes(), JsonLimits::DEFAULT).unwrap()
}
fn call(host: &mut NetworkHostV4, text: &str) -> Result<JsonValue, HostError> {
    host.call(&request(text), || Ok::<_, ()>(())).unwrap()
}
fn integer(value: JsonValue) -> i64 {
    if let JsonValue::Int(n) = value {
        n
    } else {
        panic!("expected Int")
    }
}
fn connect(bytes: usize, http: HttpExchangeLimits) -> (NetworkHostV4, i64, TcpStream) {
    let mut host = NetworkHostV4::new(NetworkLimits {
        bytes,
        milliseconds: 1000,
        ..NetworkLimits::DEFAULT
    })
    .unwrap();
    host.set_http_limits(http).unwrap();
    host.grant_port(0).unwrap();
    let listener = integer(call(&mut host, r#"{"version":4,"op":"net.listen","port":0}"#).unwrap());
    let port = integer(
        call(
            &mut host,
            &format!(r#"{{"version":4,"op":"net.port","handle":{listener}}}"#),
        )
        .unwrap(),
    );
    let peer = TcpStream::connect((Ipv4Addr::LOCALHOST, port as u16)).unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    peer.set_write_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let stream = integer(
        call(
            &mut host,
            &format!(r#"{{"version":4,"op":"net.accept","handle":{listener}}}"#),
        )
        .unwrap(),
    );
    (host, stream, peer)
}
fn receive(handle: i64) -> String {
    format!(r#"{{"version":4,"op":"http.receive","handle":{handle}}}"#)
}
fn response(handle: i64) -> String {
    format!(
        r#"{{"version":4,"op":"http.respond","handle":{handle},"status":200,"headers":[["Content-Type","application/json"]],"body":"e30="}}"#
    )
}
fn field<'a>(value: &'a JsonValue, key: &str) -> &'a JsonValue {
    if let JsonValue::Object(fields) = value {
        &fields.iter().find(|(k, _)| k == key).unwrap().1
    } else {
        panic!("expected Object")
    }
}
#[test]
fn incremental_http_claim_blocks_raw_io_and_closes_after_one_response() {
    let (mut host, stream, mut peer) = connect(1, HttpExchangeLimits::DEFAULT);
    peer.write_all(b"POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: 2\r\n\r\n\0\xff")
        .unwrap();
    let parsed = call(&mut host, &receive(stream)).unwrap();
    assert_eq!(field(&parsed, "method"), &JsonValue::String("POST".into()));
    assert_eq!(field(&parsed, "target"), &JsonValue::String("/echo".into()));
    assert_eq!(field(&parsed, "body"), &JsonValue::String("AP8=".into()));
    assert_eq!(
        call(
            &mut host,
            &format!(r#"{{"version":4,"op":"net.read","handle":{stream},"maximum":1}}"#)
        )
        .unwrap_err()
        .code,
        "input"
    );
    assert_eq!(call(&mut host, &receive(stream)).unwrap_err().code, "input");
    call(&mut host, &response(stream)).unwrap();
    let mut wire = Vec::new();
    peer.read_to_end(&mut wire).unwrap();
    assert_eq!(wire,b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}");
    assert_eq!(host.usage(), (1, 0));
    assert!(call(&mut host, &response(stream)).is_err());
    host.close_all().unwrap();
}
#[test]
fn head_context_cannot_be_overridden_by_response_body() {
    let (mut host, stream, mut peer) = connect(128, HttpExchangeLimits::DEFAULT);
    peer.write_all(b"HEAD /health HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    call(&mut host, &receive(stream)).unwrap();
    call(&mut host, &response(stream)).unwrap();
    let mut wire = Vec::new();
    peer.read_to_end(&mut wire).unwrap();
    assert!(wire.ends_with(b"Content-Length: 2\r\nConnection: close\r\n\r\n"));
    assert!(!wire.ends_with(b"{}"));
    host.close_all().unwrap();
}
#[test]
fn invalid_framing_is_never_published_and_receives_bounded_automatic_error() {
    for input in [
        b"GET / HTTP/1.1\r\n\r\n".as_slice(),
        b"POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 1\r\nContent-Length: 1\r\n\r\na",
        b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\njunk",
    ] {
        let (mut host, stream, mut peer) = connect(1024, HttpExchangeLimits::DEFAULT);
        peer.write_all(input).unwrap();
        assert!(call(&mut host, &receive(stream)).is_err());
        let mut wire = Vec::new();
        peer.read_to_end(&mut wire).unwrap();
        assert_eq!(
            wire,
            b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        assert_eq!(host.usage(), (1, 0));
        assert!(call(&mut host, &receive(stream)).is_err());
        host.close_all().unwrap();
    }
}
#[test]
fn cancellation_deadline_and_io_exhaustion_release_the_claimed_stream() {
    let (mut host, stream, _peer) = connect(64, HttpExchangeLimits::DEFAULT);
    let result = host.call(&request(&receive(stream)), || Err("cancelled"));
    assert_eq!(result.unwrap_err(), "cancelled");
    assert_eq!(host.usage(), (1, 1)); // pre-request cancellation leaves ownership untouched
    let mut checks = 0;
    let result = host.call(&request(&receive(stream)), || {
        checks += 1;
        if checks > 1 {
            Err("cancelled receive")
        } else {
            Ok(())
        }
    });
    assert_eq!(result.unwrap_err(), "cancelled receive");
    assert_eq!(host.usage(), (1, 0));
    host.close_all().unwrap();
    let (mut host, stream, _peer) = connect(
        64,
        HttpExchangeLimits {
            milliseconds: 0,
            ..HttpExchangeLimits::DEFAULT
        },
    );
    assert_eq!(
        call(&mut host, &receive(stream)).unwrap_err().code,
        "timeout"
    );
    assert_eq!(host.usage(), (1, 0));
    host.close_all().unwrap();
    let (mut host, stream, mut peer) = connect(
        1,
        HttpExchangeLimits {
            io_operations: 1,
            ..HttpExchangeLimits::DEFAULT
        },
    );
    peer.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    assert_eq!(call(&mut host, &receive(stream)).unwrap_err().code, "limit");
    assert_eq!(host.usage(), (1, 0));
    host.close_all().unwrap();
}
#[test]
fn handler_time_counts_and_invalid_response_revokes_http_owner() {
    let (mut host, stream, mut peer) = connect(
        1024,
        HttpExchangeLimits {
            milliseconds: 20,
            ..HttpExchangeLimits::DEFAULT
        },
    );
    peer.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    call(&mut host, &receive(stream)).unwrap();
    std::thread::sleep(Duration::from_millis(30));
    assert_eq!(
        call(&mut host, &response(stream)).unwrap_err().code,
        "timeout"
    );
    let mut wire = Vec::new();
    peer.read_to_end(&mut wire).unwrap();
    assert!(wire.is_empty());
    assert_eq!(host.usage(), (1, 0));
    host.close_all().unwrap();
    let (mut host, stream, mut peer) = connect(1024, HttpExchangeLimits::DEFAULT);
    peer.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    call(&mut host, &receive(stream)).unwrap();
    let invalid = response(stream).replace("Content-Type", "Content-Length");
    assert_eq!(call(&mut host, &invalid).unwrap_err().code, "framing");
    let mut wire = Vec::new();
    peer.read_to_end(&mut wire).unwrap();
    assert!(wire.is_empty());
    assert_eq!(host.usage(), (1, 0));
    host.close_all().unwrap();
}
#[test]
fn truncated_and_unsupported_requests_refuse_before_handler_publication() {
    for (wire, code, status) in [
        (
            b"POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 2\r\n\r\nx".as_slice(),
            "truncated",
            b"400".as_slice(),
        ),
        (
            b"POST / HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: gzip, chunked\r\n\r\n",
            "unsupported",
            b"501",
        ),
    ] {
        let (mut host, stream, mut peer) = connect(1024, HttpExchangeLimits::DEFAULT);
        peer.write_all(wire).unwrap();
        peer.shutdown(Shutdown::Write).unwrap();
        assert_eq!(call(&mut host, &receive(stream)).unwrap_err().code, code);
        let mut reply = Vec::new();
        peer.read_to_end(&mut reply).unwrap();
        assert_eq!(&reply[9..12], status);
        assert_eq!(host.usage(), (1, 0));
        host.close_all().unwrap();
    }
}

#[test]
fn slow_partial_input_cannot_reset_the_aggregate_deadline() {
    let (mut host, stream, mut peer) = connect(
        1,
        HttpExchangeLimits {
            milliseconds: 30,
            ..HttpExchangeLimits::DEFAULT
        },
    );
    peer.write_all(b"G").unwrap();
    let sender = std::thread::spawn(move || {
        for byte in b"ET / HTTP/1.1\r\nHost: localhost\r\n\r\n" {
            std::thread::sleep(Duration::from_millis(5));
            if peer.write_all(&[*byte]).is_err() {
                break;
            }
        }
    });
    assert_eq!(
        call(&mut host, &receive(stream)).unwrap_err().code,
        "timeout"
    );
    assert_eq!(host.usage(), (1, 0));
    host.close_all().unwrap();
    sender.join().unwrap();
}

#[test]
fn exchange_limits_cannot_widen_or_mutate_after_socket_publication() {
    for limits in [
        HttpExchangeLimits {
            milliseconds: 30001,
            ..HttpExchangeLimits::DEFAULT
        },
        HttpExchangeLimits {
            io_operations: 4097,
            ..HttpExchangeLimits::DEFAULT
        },
        HttpExchangeLimits {
            request: HttpRequestLimits {
                body_bytes: 1024 * 1024 + 1,
                ..HttpRequestLimits::DEFAULT
            },
            ..HttpExchangeLimits::DEFAULT
        },
    ] {
        let mut host = NetworkHostV4::new(NetworkLimits::DEFAULT).unwrap();
        assert_eq!(host.set_http_limits(limits).unwrap_err().code, "policy");
    }
    let (mut host, _, _peer) = connect(1024, HttpExchangeLimits::DEFAULT);
    assert_eq!(
        host.set_http_limits(HttpExchangeLimits {
            milliseconds: 0,
            ..HttpExchangeLimits::DEFAULT
        })
        .unwrap_err()
        .code,
        "policy"
    );
    host.close_all().unwrap();
    for policy in [
        r#"{"version":4,"http":{"milliseconds":30001}}"#,
        r#"{"version":4,"http":{"io_operations":4097}}"#,
        r#"{"version":4,"http":{"head_bytes":16385}}"#,
        r#"{"version":4,"http":{"line_bytes":8193}}"#,
        r#"{"version":4,"http":{"headers":65}}"#,
        r#"{"version":4,"http":{"body_bytes":1048577}}"#,
        r#"{"version":4,"http":{"response_bytes":1064961}}"#,
        r#"{"version":4,"http":{"work":2097153}}"#,
        r#"{"version":4,"http":{"unknown":0}}"#,
        r#"{"version":4,"http":{"body_bytes":-1}}"#,
    ] {
        assert!(
            DataHostPolicy::from_policy(policy.as_bytes()).is_err(),
            "{policy}"
        );
    }
    assert!(DataHostPolicy::from_policy(br#"{"version":4,"http":{"milliseconds":0,"io_operations":0,"head_bytes":0,"line_bytes":0,"headers":0,"body_bytes":0,"response_bytes":0,"work":0}}"#).is_ok());
}

#[test]
fn execution_scope_cleanup_closes_an_unanswered_http_owner() {
    let (mut host, stream, mut peer) = connect(1024, HttpExchangeLimits::DEFAULT);
    peer.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    call(&mut host, &receive(stream)).unwrap();
    assert_eq!(host.usage(), (1, 1));
    host.close_all().unwrap();
    assert_eq!(host.usage(), (0, 0));
    let mut wire = Vec::new();
    peer.read_to_end(&mut wire).unwrap();
    assert!(wire.is_empty());
    assert!(call(&mut host, &response(stream)).is_err());
}

#[test]
fn chunked_binary_and_extensions_are_decoded_across_one_byte_reads() {
    for coding in ["chunked", "cHuNkEd"] {
        let (mut host, stream, mut peer) = connect(1, HttpExchangeLimits::DEFAULT);
        peer.write_all(
            format!(
                "POST /echo HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: {coding}\r\n\r\n"
            )
            .as_bytes(),
        )
        .unwrap();
        peer.write_all(b"1;x=\"v\\\"\"\r\n\0\r\n1;ignored\r\n\xff\r\n000\r\n\r\n")
            .unwrap();
        let value = call(&mut host, &receive(stream)).unwrap();
        assert_eq!(field(&value, "body"), &JsonValue::String("AP8=".into()));
        assert_eq!(field(&value, "target"), &JsonValue::String("/echo".into()));
        assert_eq!(call(&mut host, &receive(stream)).unwrap_err().code, "input");
        call(&mut host, &response(stream)).unwrap();
        let mut reply = Vec::new();
        peer.read_to_end(&mut reply).unwrap();
        assert!(reply.starts_with(b"HTTP/1.1 200"));
        assert_eq!(host.usage(), (1, 0));
        host.close_all().unwrap();
    }
}
#[test]
fn chunked_ambiguity_codings_trailers_and_bad_frames_never_publish() {
    for (fields, body, code) in [
        (
            "Transfer-Encoding: chunked\r\nContent-Length: 0\r\n",
            "0\r\n\r\n",
            "framing",
        ),
        (
            "Transfer-Encoding: chunked\r\nTransfer-Encoding: chunked\r\n",
            "0\r\n\r\n",
            "framing",
        ),
        (
            "Transfer-Encoding: chunked, chunked\r\n",
            "0\r\n\r\n",
            "unsupported",
        ),
        (
            "Transfer-Encoding: chunked, gzip\r\n",
            "0\r\n\r\n",
            "unsupported",
        ),
        (
            "Transfer-Encoding: gzip, chunked\r\n",
            "0\r\n\r\n",
            "unsupported",
        ),
        (
            "Transfer-Encoding: chunked;q=1\r\n",
            "0\r\n\r\n",
            "unsupported",
        ),
        ("Transfer-Encoding: \r\n", "0\r\n\r\n", "unsupported"),
        (
            "Transfer-Encoding: chunked\r\nTrailer: Host\r\n",
            "0\r\n\r\n",
            "unsupported",
        ),
        (
            "Transfer-Encoding: chunked\r\n",
            "0\r\nHost: other\r\n\r\n",
            "unsupported",
        ),
        ("Transfer-Encoding: chunked\r\n", "1\r\nxX", "framing"),
        (
            "Transfer-Encoding: chunked\r\n",
            "1;=bad\r\nx\r\n0\r\n\r\n",
            "syntax",
        ),
        ("Transfer-Encoding: chunked\r\n", "0\r\n\r\nx", "framing"),
        ("Transfer-Encoding: chunked\r\n", "2\r\nx", "truncated"),
    ] {
        let (mut host, stream, mut peer) = connect(4096, HttpExchangeLimits::DEFAULT);
        peer.write_all(
            format!("POST / HTTP/1.1\r\nHost: localhost\r\n{fields}\r\n{body}").as_bytes(),
        )
        .unwrap();
        peer.shutdown(Shutdown::Write).unwrap();
        assert_eq!(
            call(&mut host, &receive(stream)).unwrap_err().code,
            code,
            "{fields:?} {body:?}"
        );
        assert!(call(&mut host, &response(stream)).is_err());
        assert_eq!(host.usage(), (1, 0));
        host.close_all().unwrap();
    }
}
#[test]
fn chunked_body_line_and_aggregate_work_limits_close_claimed_owner() {
    for (request_limits, body) in [
        (
            HttpRequestLimits {
                body_bytes: 1,
                ..HttpRequestLimits::DEFAULT
            },
            "2\r\nab\r\n0\r\n\r\n".to_string(),
        ),
        (
            HttpRequestLimits {
                line_bytes: 64,
                ..HttpRequestLimits::DEFAULT
            },
            format!("1;x={}\r\nx\r\n0\r\n\r\n", "a".repeat(65)),
        ),
        (
            HttpRequestLimits {
                work: 1000,
                ..HttpRequestLimits::DEFAULT
            },
            format!("1;x={}\r\nx\r\n0\r\n\r\n", "a".repeat(120)),
        ),
    ] {
        let (mut host, stream, mut peer) = connect(
            4096,
            HttpExchangeLimits {
                request: request_limits,
                ..HttpExchangeLimits::DEFAULT
            },
        );
        peer.write_all(
            format!(
                "POST / HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n{body}"
            )
            .as_bytes(),
        )
        .unwrap();
        assert_eq!(call(&mut host, &receive(stream)).unwrap_err().code, "limit");
        let mut reply = Vec::new();
        peer.read_to_end(&mut reply).unwrap();
        assert_eq!(&reply[9..12], b"413");
        assert_eq!(host.usage(), (1, 0));
        host.close_all().unwrap();
    }
}
#[test]
fn chunked_body_slow_progress_obeys_shared_deadline_and_cleanup() {
    let (mut host, stream, mut peer) = connect(
        4096,
        HttpExchangeLimits {
            milliseconds: 30,
            ..HttpExchangeLimits::DEFAULT
        },
    );
    peer.write_all(
        b"POST / HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n20\r\nx",
    )
    .unwrap();
    let sender = std::thread::spawn(move || {
        for _ in 0..30 {
            std::thread::sleep(Duration::from_millis(5));
            if peer.write_all(b"x").is_err() {
                break;
            }
        }
    });
    assert_eq!(
        call(&mut host, &receive(stream)).unwrap_err().code,
        "timeout"
    );
    sender.join().unwrap();
    assert_eq!(host.usage(), (1, 0));
    host.close_all().unwrap();
}

#[test]
fn chunked_mid_body_cancellation_and_io_exhaustion_revoke_owner() {
    let partial = b"POST / HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n2\r\nx";
    let (mut host, stream, mut peer) = connect(4096, HttpExchangeLimits::DEFAULT);
    peer.write_all(partial).unwrap();
    let mut checks = 0;
    let result = host.call(&request(&receive(stream)), || {
        checks += 1;
        if checks >= 3 {
            Err("cancel body wait")
        } else {
            Ok(())
        }
    });
    assert_eq!(result.unwrap_err(), "cancel body wait");
    assert_eq!(host.usage(), (1, 0));
    assert!(call(&mut host, &response(stream)).is_err());
    host.close_all().unwrap();
    let (mut host, stream, mut peer) = connect(
        4096,
        HttpExchangeLimits {
            io_operations: 1,
            ..HttpExchangeLimits::DEFAULT
        },
    );
    peer.write_all(partial).unwrap();
    assert_eq!(call(&mut host, &receive(stream)).unwrap_err().code, "limit");
    assert_eq!(host.usage(), (1, 0));
    host.close_all().unwrap();
}

#[test]
fn trailers_are_validated_discarded_and_never_merged_into_request_headers() {
    for declaration in ["", "Trailer: X-Trace, Content-Digest\r\n"] {
        let (mut host, stream, mut peer) = connect(1, HttpExchangeLimits::DEFAULT);
        peer.write_all(format!("POST / HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\nX-Trace: head\r\n{declaration}\r\n1\r\nx\r\n0\r\nX-Trace: tail\r\nX-Other: discarded\r\nContent-Digest: not-verified\r\n\r\n").as_bytes()).unwrap();
        let value = call(&mut host, &receive(stream)).unwrap();
        assert_eq!(field(&value, "body"), &JsonValue::String("eA==".into()));
        let JsonValue::Array(headers) = field(&value, "headers") else {
            panic!("headers")
        };
        let trace: Vec<_> = headers
            .iter()
            .filter(|h| field(h, "name") == &JsonValue::String("X-Trace".into()))
            .collect();
        assert_eq!(trace.len(), 1);
        assert_eq!(
            field(trace[0], "value"),
            &JsonValue::String("aGVhZA==".into())
        );
        assert!(!headers
            .iter()
            .any(|h| field(h, "name") == &JsonValue::String("X-Other".into())));
        call(&mut host, &response(stream)).unwrap();
        host.close_all().unwrap();
    }
}
#[test]
fn trailer_declarations_and_fields_refuse_malformed_or_security_sensitive_data() {
    for (extra, tail, code) in [
        ("Trailer: Host\r\n", "", "unsupported"),
        ("Trailer: Authorization\r\n", "", "unsupported"),
        ("Trailer: X,\r\n", "", "syntax"),
        ("Trailer: \r\n", "", "syntax"),
        ("Trailer: X\r\nTrailer: Y\r\n", "", "framing"),
        ("Trailer: X Y\r\n", "", "syntax"),
        ("", "X : value\r\n", "syntax"),
        ("", " X: value\r\n", "syntax"),
        ("", "Content-Length: 9\r\n", "unsupported"),
        ("", "Authorization: auth\r\n", "unsupported"),
        ("", "X: value\r\n", "truncated"),
    ] {
        let (mut host, stream, mut peer) = connect(4096, HttpExchangeLimits::DEFAULT);
        let end = if code == "truncated" { "" } else { "\r\n" };
        peer.write_all(format!("POST / HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n{extra}\r\n0\r\n{tail}{end}").as_bytes()).unwrap();
        peer.shutdown(Shutdown::Write).unwrap();
        assert_eq!(
            call(&mut host, &receive(stream)).unwrap_err().code,
            code,
            "{extra:?} {tail:?}"
        );
        assert_eq!(host.usage(), (1, 0));
        assert!(call(&mut host, &response(stream)).is_err());
        host.close_all().unwrap();
    }
    let (mut host, stream, mut peer) = connect(4096, HttpExchangeLimits::DEFAULT);
    peer.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nTrailer: X\r\n\r\n")
        .unwrap();
    assert_eq!(
        call(&mut host, &receive(stream)).unwrap_err().code,
        "unsupported"
    );
}
#[test]
fn trailer_byte_count_line_count_and_shared_work_limits_revoke_owner() {
    for (limits, tail) in [
        (
            HttpRequestLimits {
                head_bytes: 100,
                ..HttpRequestLimits::DEFAULT
            },
            format!("X: {}\r\n", "a".repeat(100)),
        ),
        (
            HttpRequestLimits {
                line_bytes: 32,
                ..HttpRequestLimits::DEFAULT
            },
            format!("X: {}\r\n", "a".repeat(33)),
        ),
        (
            HttpRequestLimits {
                headers: 2,
                ..HttpRequestLimits::DEFAULT
            },
            "X: a\r\nY: b\r\nZ: c\r\n".into(),
        ),
        (
            HttpRequestLimits {
                work: 1000,
                ..HttpRequestLimits::DEFAULT
            },
            format!("X: {}\r\n", "a".repeat(20)),
        ),
    ] {
        let (mut host, stream, mut peer) = connect(
            4096,
            HttpExchangeLimits {
                request: limits,
                ..HttpExchangeLimits::DEFAULT
            },
        );
        peer.write_all(format!("POST / HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n{tail}\r\n").as_bytes()).unwrap();
        assert_eq!(call(&mut host, &receive(stream)).unwrap_err().code, "limit");
        assert_eq!(host.usage(), (1, 0));
        host.close_all().unwrap();
    }
}

#[test]
fn unfinished_trailers_share_cancellation_and_deadline_cleanup() {
    let partial =
        b"POST / HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n0\r\nX-Trace: ";
    let (mut host, stream, mut peer) = connect(4096, HttpExchangeLimits::DEFAULT);
    peer.write_all(partial).unwrap();
    let mut checks = 0;
    assert_eq!(
        host.call(&request(&receive(stream)), || {
            checks += 1;
            if checks >= 3 {
                Err("cancel trailer")
            } else {
                Ok(())
            }
        })
        .unwrap_err(),
        "cancel trailer"
    );
    assert_eq!(host.usage(), (1, 0));
    host.close_all().unwrap();
    let (mut host, stream, mut peer) = connect(
        4096,
        HttpExchangeLimits {
            milliseconds: 30,
            ..HttpExchangeLimits::DEFAULT
        },
    );
    peer.write_all(partial).unwrap();
    let sender = std::thread::spawn(move || {
        for _ in 0..30 {
            std::thread::sleep(Duration::from_millis(5));
            if peer.write_all(b"x").is_err() {
                break;
            }
        }
    });
    assert_eq!(
        call(&mut host, &receive(stream)).unwrap_err().code,
        "timeout"
    );
    sender.join().unwrap();
    assert_eq!(host.usage(), (1, 0));
    host.close_all().unwrap();
}

#[test]
fn continue_handshake_precedes_body_for_fixed_and_chunked_framing() {
    for (framing, body) in [
        ("Content-Length: 2", b"\0\xff".as_slice()),
        (
            "Transfer-Encoding: chunked",
            b"2\r\n\0\xff\r\n0\r\n\r\n".as_slice(),
        ),
    ] {
        let (mut host, stream, mut peer) = connect(1, HttpExchangeLimits::DEFAULT);
        let head = format!(
            "POST / HTTP/1.1\r\nHost: localhost\r\n{framing}\r\nExpect: 100-CoNtInUe\r\n\r\n"
        );
        let body = body.to_vec();
        let client = std::thread::spawn(move || {
            peer.write_all(head.as_bytes()).unwrap();
            let mut interim = [0u8; 25];
            peer.read_exact(&mut interim).unwrap();
            assert_eq!(&interim, b"HTTP/1.1 100 Continue\r\n\r\n");
            peer.write_all(&body).unwrap();
            let mut final_wire = Vec::new();
            peer.read_to_end(&mut final_wire).unwrap();
            assert!(final_wire.starts_with(b"HTTP/1.1 200 OK\r\n"));
        });
        let parsed = call(&mut host, &receive(stream)).unwrap();
        assert_eq!(field(&parsed, "body"), &JsonValue::String("AP8=".into()));
        call(&mut host, &response(stream)).unwrap();
        client.join().unwrap();
        assert_eq!(host.usage(), (1, 0));
        host.close_all().unwrap();
    }
}
#[test]
fn complete_buffered_or_empty_body_omits_interim_response() {
    for suffix in [
        "Content-Length: 1\r\n\r\nx",
        "Content-Length: 0\r\n\r\n",
        "Transfer-Encoding: chunked\r\n\r\n1\r\nx\r\n0\r\n\r\n",
    ] {
        let (mut host, stream, mut peer) = connect(4096, HttpExchangeLimits::DEFAULT);
        peer.write_all(
            format!("POST / HTTP/1.1\r\nHost: localhost\r\nExpect: 100-continue\r\n{suffix}")
                .as_bytes(),
        )
        .unwrap();
        call(&mut host, &receive(stream)).unwrap();
        call(&mut host, &response(stream)).unwrap();
        let mut wire = Vec::new();
        peer.read_to_end(&mut wire).unwrap();
        assert!(wire.starts_with(b"HTTP/1.1 200 OK\r\n"));
        host.close_all().unwrap();
    }
}
#[test]
fn expectation_and_head_refusals_send_final_without_waiting_for_body() {
    for (fields, code, status) in [
        (
            "Content-Length: 1\r\nExpect: unknown\r\n",
            "expectation",
            b"417",
        ),
        ("Content-Length: 1\r\nExpect: \r\n", "expectation", b"417"),
        (
            "Content-Length: 1\r\nExpect: 100-continue, other\r\n",
            "expectation",
            b"417",
        ),
        (
            "Content-Length: 1\r\nExpect: 100-continue\r\nExpect: 100-continue\r\n",
            "framing",
            b"400",
        ),
        (
            "Content-Length: 1\r\nTransfer-Encoding: chunked\r\nExpect: 100-continue\r\n",
            "framing",
            b"400",
        ),
        (
            "Content-Length: 1048577\r\nExpect: 100-continue\r\n",
            "limit",
            b"413",
        ),
    ] {
        let (mut host, stream, mut peer) = connect(4096, HttpExchangeLimits::DEFAULT);
        peer.write_all(format!("POST / HTTP/1.1\r\nHost: localhost\r\n{fields}\r\n").as_bytes())
            .unwrap();
        assert_eq!(call(&mut host, &receive(stream)).unwrap_err().code, code);
        let mut wire = Vec::new();
        peer.read_to_end(&mut wire).unwrap();
        assert_eq!(&wire[9..12], status);
        assert_eq!(host.usage(), (1, 0));
        assert!(call(&mut host, &response(stream)).is_err());
        host.close_all().unwrap();
    }
}
#[test]
fn continue_response_uses_same_io_budget_and_cancellation_control() {
    let head =
        b"POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 1\r\nExpect: 100-continue\r\n\r\n";
    for operations in [1, 2] {
        let (mut host, stream, mut peer) = connect(
            4096,
            HttpExchangeLimits {
                io_operations: operations,
                ..HttpExchangeLimits::DEFAULT
            },
        );
        peer.write_all(head).unwrap();
        assert_eq!(call(&mut host, &receive(stream)).unwrap_err().code, "limit");
        let mut wire = Vec::new();
        peer.read_to_end(&mut wire).unwrap();
        assert_eq!(
            wire,
            if operations == 1 {
                b"".as_slice()
            } else {
                b"HTTP/1.1 100 Continue\r\n\r\n".as_slice()
            }
        );
        assert_eq!(host.usage(), (1, 0));
        host.close_all().unwrap();
    }
    let (mut host, stream, mut peer) = connect(4096, HttpExchangeLimits::DEFAULT);
    peer.write_all(head).unwrap();
    peer.set_nonblocking(true).unwrap();
    assert_eq!(
        host.call(&request(&receive(stream)), || {
            let mut observed = [0u8; 25];
            match peer.peek(&mut observed) {
                Ok(n) if n > 0 => Err("cancel after interim"),
                Ok(_) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(()),
                Err(e) => panic!("interim observation failed: {e}"),
            }
        })
        .unwrap_err(),
        "cancel after interim"
    );
    peer.set_nonblocking(false).unwrap();
    let mut wire = Vec::new();
    peer.read_to_end(&mut wire).unwrap();
    assert_eq!(wire, b"HTTP/1.1 100 Continue\r\n\r\n");
    assert_eq!(host.usage(), (1, 0));
    host.close_all().unwrap();
}
#[test]
fn continue_does_not_restart_accept_to_response_deadline() {
    let (mut host, stream, mut peer) = connect(
        4096,
        HttpExchangeLimits {
            milliseconds: 30,
            ..HttpExchangeLimits::DEFAULT
        },
    );
    peer.write_all(
        b"POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 1\r\nExpect: 100-continue\r\n\r\n",
    )
    .unwrap();
    assert_eq!(
        call(&mut host, &receive(stream)).unwrap_err().code,
        "timeout"
    );
    let mut wire = Vec::new();
    peer.read_to_end(&mut wire).unwrap();
    assert_eq!(wire, b"HTTP/1.1 100 Continue\r\n\r\n");
    assert_eq!(host.usage(), (1, 0));
    host.close_all().unwrap();
}

#[test]
fn interim_output_obeys_lowered_response_limits() {
    for response_limits in [
        HttpResponseLimits {
            output_bytes: 0,
            ..HttpResponseLimits::DEFAULT
        },
        HttpResponseLimits {
            head_bytes: 24,
            ..HttpResponseLimits::DEFAULT
        },
        HttpResponseLimits {
            line_bytes: 20,
            ..HttpResponseLimits::DEFAULT
        },
        HttpResponseLimits {
            work: 24,
            ..HttpResponseLimits::DEFAULT
        },
    ] {
        let (mut host, stream, mut peer) = connect(
            4096,
            HttpExchangeLimits {
                response: response_limits,
                ..HttpExchangeLimits::DEFAULT
            },
        );
        peer.write_all(b"POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 1\r\nExpect: 100-continue\r\n\r\n").unwrap();
        assert_eq!(call(&mut host, &receive(stream)).unwrap_err().code, "limit");
        let mut wire = Vec::new();
        peer.read_to_end(&mut wire).unwrap();
        assert!(wire.is_empty());
        assert_eq!(host.usage(), (1, 0));
        host.close_all().unwrap();
    }
}

#[test]
fn yielding_head_preserves_fragments_custody_and_single_attempt_charge() {
    let (mut h, n, mut peer) = connect(65536, HttpExchangeLimits::DEFAULT);
    let poll = format!(r#"{{"version":4,"op":"http.poll_head","handle":{n}}}"#);
    assert_eq!(call(&mut h, &poll).unwrap(), JsonValue::Bool(false));
    assert_eq!(h.usage(), (1, 1));
    peer.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n")
        .unwrap();
    assert_eq!(call(&mut h, &poll).unwrap(), JsonValue::Bool(false));
    // Raw I/O and response operations cannot bypass the pending HTTP state.
    assert_eq!(
        call(
            &mut h,
            &format!(r#"{{"version":4,"op":"net.read","handle":{n},"maximum":1}}"#)
        )
        .unwrap_err()
        .code,
        "input"
    );
    peer.write_all(b"\r\n").unwrap();
    let mut ready = false;
    for _ in 0..100 {
        ready = call(&mut h, &poll).unwrap() == JsonValue::Bool(true);
        if ready {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(ready);
    assert_eq!(call(&mut h, &poll).unwrap(), JsonValue::Bool(true));
    let received = call(
        &mut h,
        &format!(r#"{{"version":4,"op":"http.receive","handle":{n}}}"#),
    )
    .unwrap();
    assert!(matches!(received, JsonValue::Object(_)));
    assert_eq!(call(&mut h, &poll).unwrap_err().code, "input");
    h.close_all().unwrap();
    assert_eq!(h.usage(), (0, 0));
}

#[test]
fn yielding_head_idle_attempts_share_io_limit_and_cancelled_claim_drops_owner() {
    let (mut h, n, _peer) = connect(
        65536,
        HttpExchangeLimits {
            io_operations: 1,
            ..HttpExchangeLimits::DEFAULT
        },
    );
    let poll = format!(r#"{{"version":4,"op":"http.poll_head","handle":{n}}}"#);
    assert_eq!(call(&mut h, &poll).unwrap(), JsonValue::Bool(false));
    assert_eq!(call(&mut h, &poll).unwrap_err().code, "limit");
    assert_eq!(h.usage(), (1, 0));
    let (mut h, n, _peer) = connect(65536, HttpExchangeLimits::DEFAULT);
    let poll = request(&format!(
        r#"{{"version":4,"op":"http.poll_head","handle":{n}}}"#
    ));
    let mut ticks = 0;
    assert_eq!(
        h.call(&poll, || {
            ticks += 1;
            if ticks == 2 {
                Err("cancel claimed head")
            } else {
                Ok(())
            }
        })
        .unwrap_err(),
        "cancel claimed head"
    );
    assert_eq!(h.usage(), (1, 0));
}

#[test]
fn yielding_head_readiness_is_not_validation_and_eof_is_rejected_on_receive() {
    for wire in [b"BAD\r\n\r\n".as_slice(), b"GET / HTTP/1.1\r\nHost:"] {
        let (mut h, n, mut peer) = connect(65536, HttpExchangeLimits::DEFAULT);
        peer.write_all(wire).unwrap();
        peer.shutdown(Shutdown::Write).unwrap();
        let poll = format!(r#"{{"version":4,"op":"http.poll_head","handle":{n}}}"#);
        let mut ready = false;
        for _ in 0..100 {
            ready = call(&mut h, &poll).unwrap() == JsonValue::Bool(true);
            if ready {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(ready);
        assert!(call(
            &mut h,
            &format!(r#"{{"version":4,"op":"http.receive","handle":{n}}}"#)
        )
        .is_err());
        assert_eq!(h.usage(), (1, 0));
    }
}

#[test]
fn idle_head_yields_while_other_connection_reaches_handler() {
    let (mut h, slow, _slow_peer) = connect(65536, HttpExchangeLimits::DEFAULT);
    let poll = format!(r#"{{"version":4,"op":"http.poll_head","handle":{slow}}}"#);
    assert_eq!(call(&mut h, &poll).unwrap(), JsonValue::Bool(false));
    let l = integer(call(&mut h, r#"{"version":4,"op":"net.listen","port":0}"#).unwrap());
    let p = integer(
        call(
            &mut h,
            &format!(r#"{{"version":4,"op":"net.port","handle":{l}}}"#),
        )
        .unwrap(),
    );
    let mut peer = TcpStream::connect((Ipv4Addr::LOCALHOST, p as u16)).unwrap();
    let fast = integer(
        call(
            &mut h,
            &format!(r#"{{"version":4,"op":"net.accept","handle":{l}}}"#),
        )
        .unwrap(),
    );
    peer.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    assert!(matches!(
        call(&mut h, &receive(fast)).unwrap(),
        JsonValue::Object(_)
    ));
    assert_eq!(call(&mut h, &poll).unwrap(), JsonValue::Bool(false));
    h.close_all().unwrap();
}

#[test]
fn prepared_head_cannot_publish_after_aggregate_deadline_expires() {
    let (mut h, n, mut peer) = connect(65536, HttpExchangeLimits::DEFAULT);
    peer.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    let poll = format!(r#"{{"version":4,"op":"http.poll_head","handle":{n}}}"#);
    let mut ready = false;
    for _ in 0..100 {
        ready = call(&mut h, &poll).unwrap() == JsonValue::Bool(true);
        if ready {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(ready);
    std::thread::sleep(Duration::from_millis(1100));
    assert_eq!(call(&mut h, &receive(n)).unwrap_err().code, "timeout");
    assert_eq!(h.usage(), (1, 0));
}

#[test]
fn body_poll_handles_fixed_and_chunked_fragments_without_publishing_partial_data() {
    for (framing, first, last) in [
        ("Content-Length: 3", b"a".as_slice(), b"bc".as_slice()),
        (
            "Transfer-Encoding: chunked",
            b"3\r\na".as_slice(),
            b"bc\r\n0\r\n\r\n".as_slice(),
        ),
    ] {
        let (mut h, n, mut peer) = connect(65536, HttpExchangeLimits::DEFAULT);
        let body_poll = format!(r#"{{"version":4,"op":"http.poll_body","handle":{n}}}"#);
        assert_eq!(call(&mut h, &body_poll).unwrap_err().code, "input");
        peer.write_all(
            format!("POST / HTTP/1.1\r\nHost: localhost\r\n{framing}\r\n\r\n").as_bytes(),
        )
        .unwrap();
        let head_poll = format!(r#"{{"version":4,"op":"http.poll_head","handle":{n}}}"#);
        let mut ready = false;
        for _ in 0..100 {
            ready = call(&mut h, &head_poll).unwrap() == JsonValue::Bool(true);
            if ready {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(ready);
        assert_eq!(call(&mut h, &body_poll).unwrap(), JsonValue::Bool(false));
        peer.write_all(first).unwrap();
        assert_eq!(call(&mut h, &body_poll).unwrap(), JsonValue::Bool(false));
        assert_eq!(
            call(
                &mut h,
                &format!(r#"{{"version":4,"op":"net.read","handle":{n},"maximum":1}}"#)
            )
            .unwrap_err()
            .code,
            "input"
        );
        peer.write_all(last).unwrap();
        for _ in 0..100 {
            ready = call(&mut h, &body_poll).unwrap() == JsonValue::Bool(true);
            if ready {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(ready);
        let result = call(&mut h, &receive(n)).unwrap();
        assert_eq!(field(&result, "body"), &JsonValue::String("YWJj".into()));
        assert_eq!(call(&mut h, &body_poll).unwrap_err().code, "input");
        h.close_all().unwrap();
    }
}

#[test]
fn body_poll_truncated_eof_drops_pending_owner() {
    let (mut h, n, mut peer) = connect(65536, HttpExchangeLimits::DEFAULT);
    peer.write_all(b"POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 3\r\n\r\n")
        .unwrap();
    let head_poll = format!(r#"{{"version":4,"op":"http.poll_head","handle":{n}}}"#);
    let mut ready = false;
    for _ in 0..100 {
        ready = call(&mut h, &head_poll).unwrap() == JsonValue::Bool(true);
        if ready {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(ready);
    peer.shutdown(Shutdown::Write).unwrap();
    let body_poll = format!(r#"{{"version":4,"op":"http.poll_body","handle":{n}}}"#);
    assert_eq!(call(&mut h, &body_poll).unwrap_err().code, "truncated");
    assert_eq!(h.usage(), (1, 0));
}

#[test]
fn pending_body_cancellation_and_deadline_release_private_state() {
    for expire in [false, true] {
        let (mut h, n, mut peer) = connect(65536, HttpExchangeLimits::DEFAULT);
        peer.write_all(b"POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 3\r\n\r\n")
            .unwrap();
        let head_poll = format!(r#"{{"version":4,"op":"http.poll_head","handle":{n}}}"#);
        let mut ready = false;
        for _ in 0..100 {
            ready = call(&mut h, &head_poll).unwrap() == JsonValue::Bool(true);
            if ready {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(ready);
        let body_poll = format!(r#"{{"version":4,"op":"http.poll_body","handle":{n}}}"#);
        assert_eq!(call(&mut h, &body_poll).unwrap(), JsonValue::Bool(false));
        if expire {
            std::thread::sleep(Duration::from_millis(1100));
            assert_eq!(call(&mut h, &body_poll).unwrap_err().code, "timeout");
        } else {
            let mut ticks = 0;
            assert_eq!(
                h.call(&request(&body_poll), || {
                    ticks += 1;
                    if ticks == 2 {
                        Err("cancel body")
                    } else {
                        Ok(())
                    }
                })
                .unwrap_err(),
                "cancel body"
            );
        }
        assert_eq!(h.usage(), (1, 0));
        h.close_all().unwrap();
    }
}

#[test]
fn cooperative_response_preserves_exact_wire_across_partial_writes_and_head_context() {
    for method in ["GET", "HEAD"] {
        let (mut h, n, mut peer) = connect(1, HttpExchangeLimits::DEFAULT);
        peer.write_all(format!("{method} / HTTP/1.1\r\nHost: localhost\r\n\r\n").as_bytes())
            .unwrap();
        call(&mut h, &receive(n)).unwrap();
        let start = response(n).replace("http.respond", "http.start_response");
        assert_eq!(call(&mut h, &start).unwrap(), JsonValue::Null);
        assert_eq!(h.usage(), (1, 1));
        assert_eq!(call(&mut h, &start).unwrap_err().code, "input");
        assert_eq!(
            call(
                &mut h,
                &format!(r#"{{"version":4,"op":"net.write","handle":{n},"data":"eA=="}}"#)
            )
            .unwrap_err()
            .code,
            "input"
        );
        let poll = format!(r#"{{"version":4,"op":"http.poll_response","handle":{n}}}"#);
        assert_eq!(call(&mut h, &poll).unwrap(), JsonValue::Bool(false));
        let mut complete = false;
        for _ in 0..1024 {
            complete = call(&mut h, &poll).unwrap() == JsonValue::Bool(true);
            if complete {
                break;
            }
        }
        assert!(complete);
        assert_eq!(h.usage(), (1, 0));
        assert_eq!(call(&mut h, &poll).unwrap_err().code, "input");
        let mut wire = Vec::new();
        peer.read_to_end(&mut wire).unwrap();
        let expected = render_http_response(
            200,
            if method == "HEAD" {
                HttpResponseContext::Head
            } else {
                HttpResponseContext::Standard
            },
            &[HttpResponseHeader {
                name: "Content-Type",
                value: b"application/json",
            }],
            b"{}",
            HttpResponseLimits::DEFAULT,
        )
        .unwrap();
        assert_eq!(wire, expected);
        h.close_all().unwrap();
    }
}

#[test]
fn cooperative_response_cancellation_and_io_exhaustion_send_no_prefix() {
    for cancel in [false, true] {
        let (mut h, n, mut peer) = connect(
            65536,
            HttpExchangeLimits {
                io_operations: 1,
                ..HttpExchangeLimits::DEFAULT
            },
        );
        peer.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        call(&mut h, &receive(n)).unwrap();
        call(
            &mut h,
            &response(n).replace("http.respond", "http.start_response"),
        )
        .unwrap();
        let poll = format!(r#"{{"version":4,"op":"http.poll_response","handle":{n}}}"#);
        if cancel {
            let mut ticks = 0;
            assert_eq!(
                h.call(&request(&poll), || {
                    ticks += 1;
                    if ticks == 2 {
                        Err("cancel output")
                    } else {
                        Ok(())
                    }
                })
                .unwrap_err(),
                "cancel output"
            );
        } else {
            assert_eq!(call(&mut h, &poll).unwrap_err().code, "limit");
        }
        assert_eq!(h.usage(), (1, 0));
        let mut wire = Vec::new();
        peer.read_to_end(&mut wire).unwrap();
        assert!(wire.is_empty());
        h.close_all().unwrap();
    }
}

#[test]
fn cooperative_response_deadline_and_invalid_framing_drop_pending_output() {
    for expire in [false, true] {
        let (mut h, n, mut peer) = connect(65536, HttpExchangeLimits::DEFAULT);
        peer.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        call(&mut h, &receive(n)).unwrap();
        let start = response(n).replace("http.respond", "http.start_response");
        if expire {
            call(&mut h, &start).unwrap();
            assert!(!format!("{h:?}").contains("application/json"));
            std::thread::sleep(Duration::from_millis(1100));
            assert_eq!(
                call(
                    &mut h,
                    &format!(r#"{{"version":4,"op":"http.poll_response","handle":{n}}}"#)
                )
                .unwrap_err()
                .code,
                "timeout"
            );
        } else {
            assert!(call(&mut h, &start.replace("Content-Type", "Content-Length")).is_err());
        }
        assert_eq!(h.usage(), (1, 0));
        let mut wire = Vec::new();
        peer.read_to_end(&mut wire).unwrap();
        assert!(wire.is_empty());
        h.close_all().unwrap();
    }
}
