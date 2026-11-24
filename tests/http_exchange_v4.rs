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
            "Transfer-Encoding: chunked\r\nTrailer: X-Trailer\r\n",
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
