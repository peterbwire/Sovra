//! Aggregate HTTP attempt admission before parsing/verifier work, with scoped cleanup.
#![cfg(target_os = "linux")]
use sovra::compiler::v2::*;
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, TcpStream},
    time::Duration,
};
fn j(s: &str) -> JsonValue {
    parse_json(s.as_bytes(), JsonLimits::DEFAULT).unwrap()
}
fn call(h: &mut NetworkHostV4, s: &str) -> Result<JsonValue, HostError> {
    h.call(&j(s), || Ok::<_, ()>(())).unwrap()
}
fn number(j: JsonValue) -> i64 {
    let JsonValue::Int(n) = j else { panic!("Int") };
    n
}
fn receive(n: i64) -> String {
    format!(r#"{{"version":4,"op":"http.receive","handle":{n}}}"#)
}
fn listener(h: &mut NetworkHostV4) -> (i64, u16) {
    let l = number(call(h, r#"{"version":4,"op":"net.listen","port":0}"#).unwrap());
    let p = number(
        call(
            h,
            &format!(r#"{{"version":4,"op":"net.port","handle":{l}}}"#),
        )
        .unwrap(),
    );
    (l, p as u16)
}
fn connect(h: &mut NetworkHostV4, l: i64, p: u16) -> (i64, TcpStream) {
    let peer = TcpStream::connect((Ipv4Addr::LOCALHOST, p)).unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    let n = number(
        call(
            h,
            &format!(r#"{{"version":4,"op":"net.accept","handle":{l}}}"#),
        )
        .unwrap(),
    );
    (n, peer)
}
fn host(requests: usize) -> NetworkHostV4 {
    let mut h = NetworkHostV4::new(NetworkLimits::DEFAULT).unwrap();
    h.set_http_admission(HttpAdmissionPolicy {
        requests,
        milliseconds: 60000,
    })
    .unwrap();
    h.grant_port(0).unwrap();
    h
}
#[test]
fn external_policy_is_optional_exact_and_bounded() {
    for policy in [
        r#"{"version":4}"#,
        r#"{"version":4,"http_admission":{"requests":0,"milliseconds":1}}"#,
        r#"{"version":4,"http_admission":{"requests":1024,"milliseconds":60000}}"#,
    ] {
        assert!(DataHostPolicy::from_policy(policy.as_bytes()).is_ok());
    }
    for inner in [
        r#"{}"#,
        r#"{"requests":1}"#,
        r#"{"requests":1,"milliseconds":0}"#,
        r#"{"requests":1025,"milliseconds":1}"#,
        r#"{"requests":1,"milliseconds":60001}"#,
        r#"{"requests":-1,"milliseconds":1}"#,
        r#"{"requests":true,"milliseconds":1}"#,
        r#"{"requests":1,"milliseconds":1,"unknown":0}"#,
        r#"{"requests":1,"requests":2,"milliseconds":1}"#,
    ] {
        assert!(DataHostPolicy::from_policy(
            format!(r#"{{"version":4,"http_admission":{inner}}}"#).as_bytes()
        )
        .is_err());
    }
}
#[test]
fn malformed_successful_and_cross_listener_attempts_share_one_budget() {
    let mut h = host(2);
    let (a, pa) = listener(&mut h);
    let (b, pb) = listener(&mut h);
    let (n, mut peer) = connect(&mut h, a, pa);
    peer.write_all(b"BAD\r\n\r\n").unwrap();
    assert_eq!(call(&mut h, &receive(n)).unwrap_err().code, "syntax");
    let (n, mut peer) = connect(&mut h, b, pb);
    peer.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    call(&mut h, &receive(n)).unwrap();
    call(
        &mut h,
        &format!(r#"{{"version":4,"op":"net.close","handle":{n}}}"#),
    )
    .unwrap();
    // Closing/recreating listeners does not replenish the domain's admission state.
    h.close_all().unwrap();
    let (l, p) = listener(&mut h);
    let (n, mut peer) = connect(&mut h, l, p);
    assert_eq!(call(&mut h, &receive(n)).unwrap_err().code, "admission");
    let mut wire = Vec::new();
    peer.read_to_end(&mut wire).unwrap();
    assert!(wire.starts_with(b"HTTP/1.1 429 Too Many Requests\r\n"));
    assert!(wire
        .windows(b"Cache-Control: no-store\r\n".len())
        .any(|w| w == b"Cache-Control: no-store\r\n"));
    assert_eq!(h.usage(), (1, 0));
    h.close_all().unwrap();
}
struct NeverVerify;
impl HttpBearerVerifier for NeverVerify {
    fn verify(&mut self, _: HttpBearerRequest<'_>) -> HttpAuthenticationDecision {
        panic!("admission must precede verifier")
    }
}
#[test]
fn zero_quota_denies_before_continue_or_verifier_and_respects_output_bounds() {
    for output in [HttpResponseLimits::DEFAULT.output_bytes, 0] {
        let mut h = host(0);
        h.set_http_limits(HttpExchangeLimits {
            response: HttpResponseLimits {
                output_bytes: output,
                ..HttpResponseLimits::DEFAULT
            },
            ..HttpExchangeLimits::DEFAULT
        })
        .unwrap();
        h.set_http_access(HttpAccessPolicy::new(&["localhost"], &[], true).unwrap())
            .unwrap();
        h.set_http_bearer_verifier(Box::new(NeverVerify)).unwrap();
        let (l, p) = listener(&mut h);
        let (n, mut peer) = connect(&mut h, l, p);
        assert_eq!(
            h.set_http_admission(HttpAdmissionPolicy {
                requests: 1,
                milliseconds: 1
            })
            .unwrap_err()
            .code,
            "policy"
        );
        peer.write_all(b"POST / HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer fixture\r\nContent-Length: 10\r\nExpect: 100-continue\r\n\r\n").unwrap();
        assert_eq!(call(&mut h, &receive(n)).unwrap_err().code, "admission");
        let mut wire = Vec::new();
        let _ = peer.read_to_end(&mut wire);
        if output == 0 {
            assert!(wire.is_empty());
        } else {
            assert!(wire.starts_with(b"HTTP/1.1 429 Too Many Requests\r\n"));
        }
        assert_eq!(h.usage(), (1, 0));
        assert!(call(&mut h, &receive(n)).is_err());
        h.close_all().unwrap();
    }
}
#[test]
fn preclaim_cancellation_does_not_charge_but_claimed_failure_does() {
    let mut h = host(1);
    let (l, p) = listener(&mut h);
    let (n, mut peer) = connect(&mut h, l, p);
    peer.write_all(b"G").unwrap();
    assert_eq!(
        h.call(&j(&receive(n)), || Err("cancel before claim"))
            .unwrap_err(),
        "cancel before claim"
    );
    assert_eq!(h.usage(), (1, 1));
    let mut ticks = 0;
    assert_eq!(
        h.call(&j(&receive(n)), || {
            ticks += 1;
            if ticks >= 2 {
                Err("cancel claimed read")
            } else {
                Ok(())
            }
        })
        .unwrap_err(),
        "cancel claimed read"
    );
    assert_eq!(h.usage(), (1, 0));
    let (n, mut peer) = connect(&mut h, l, p);
    assert_eq!(call(&mut h, &receive(n)).unwrap_err().code, "admission");
    let mut wire = Vec::new();
    peer.read_to_end(&mut wire).unwrap();
    assert!(wire.starts_with(b"HTTP/1.1 429"));
    h.close_all().unwrap();
}

#[test]
fn head_polls_charge_admission_once_and_receive_does_not_recharge() {
    let mut h = host(1);
    let (l, p) = listener(&mut h);
    let (n, mut peer) = connect(&mut h, l, p);
    let poll = format!(r#"{{"version":4,"op":"http.poll_head","handle":{n}}}"#);
    assert_eq!(call(&mut h, &poll).unwrap(), JsonValue::Bool(false));
    peer.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    let mut ready = false;
    for _ in 0..100 {
        ready = call(&mut h, &poll).unwrap() == JsonValue::Bool(true);
        if ready {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(ready);
    call(&mut h, &receive(n)).unwrap();
    let (next, _peer) = connect(&mut h, l, p);
    assert_eq!(
        call(
            &mut h,
            &format!(r#"{{"version":4,"op":"http.poll_head","handle":{next}}}"#)
        )
        .unwrap_err()
        .code,
        "admission"
    );
    h.close_all().unwrap();
}
