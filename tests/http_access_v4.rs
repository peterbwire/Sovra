//! Explicit HTTP access policy grammar and pre-body socket refusal.
use sovra::compiler::v2::*;
#[test]
fn exact_grants_are_bounded_and_no_url_normalization_is_implied() {
    HttpAccessPolicy::new(
        &["LOCALHOST:80", "[::1]:90"],
        &["https://example.test", "http://localhost:80"],
        false,
    )
    .unwrap();
    HttpAccessPolicy::new(&[], &[], false).unwrap();
    for hosts in [
        vec![""],
        vec!["*.example.test"],
        vec!["localhost/path"],
        vec!["localhost", "LOCALHOST"],
        vec!["a"; 17],
    ] {
        assert_eq!(
            HttpAccessPolicy::new(&hosts, &[], true).unwrap_err().code,
            "policy"
        );
    }
    for origins in [
        vec!["null"],
        vec!["*"],
        vec!["https://x/"],
        vec!["https://user@x"],
        vec!["http://x?a"],
        vec!["https://x https://y"],
        vec!["https://x", "https://x"],
        vec!["https://x"; 17],
    ] {
        assert_eq!(
            HttpAccessPolicy::new(&["localhost"], &origins, true)
                .unwrap_err()
                .code,
            "policy"
        );
    }
    let long = "a".repeat(257);
    assert!(HttpAccessPolicy::new(&[&long], &[], true).is_err());
    assert!(HttpAccessPolicy::new(&[], &[&format!("https://{long}")], true).is_err());
}
#[test]
fn external_policy_has_exact_required_fields_types_and_no_mixed_versions() {
    let valid = r#"{"version":4,"http_access":{"hosts":["localhost"],"origins":[],"allow_missing_origin":true}}"#;
    assert!(DataHostPolicy::from_policy(valid.as_bytes()).is_ok());
    assert!(DataHostPolicy::from_policy(br#"{"version":4}"#).is_ok());
    for policy in [
        valid.replace("\"hosts\":[\"localhost\"]", "\"hosts\":[]"),
        valid.replace("true", "false"),
    ] {
        assert!(DataHostPolicy::from_policy(policy.as_bytes()).is_ok());
    }
    for policy in [
        valid.replace("\"hosts\":[\"localhost\"],", ""),
        valid.replace("\"origins\":[]", "\"origins\":[\"null\"]"),
        valid.replace(
            "\"allow_missing_origin\":true",
            "\"allow_missing_origin\":1",
        ),
        valid.replace("\"hosts\":[\"localhost\"]", "\"hosts\":[1]"),
        valid.replace(
            "\"hosts\":[\"localhost\"]",
            "\"hosts\":[\"localhost\",\"LOCALHOST\"]",
        ),
        valid.replace("\"origins\":[]", "\"unknown\":[]"),
        valid.replace("\"version\":4", "\"version\":3"),
        valid.replace("\"origins\":[]", "\"origins\":[],\"origins\":[]"),
    ] {
        assert!(
            DataHostPolicy::from_policy(policy.as_bytes()).is_err(),
            "{policy}"
        );
    }
}

#[test]
fn request_grants_are_bounded_exact_and_preserve_method_case() {
    let make = || HttpAccessPolicy::new(&["localhost"], &[], true).unwrap();
    make()
        .with_requests(&["GET", "get", "OPTIONS"], &["/safe", "/a%2Fb?x=1", "*"])
        .unwrap();
    make().with_requests(&[], &[]).unwrap();
    for methods in [
        vec![""],
        vec!["GET GET"],
        vec!["GET", "GET"],
        vec!["GET"; 17],
    ] {
        assert_eq!(
            make().with_requests(&methods, &["/"]).unwrap_err().code,
            "policy"
        );
    }
    let long = "A".repeat(33);
    assert!(make().with_requests(&[&long], &["/"]).is_err());
    for targets in [
        vec![""],
        vec!["http://localhost/"],
        vec!["/bad%"],
        vec!["/#fragment"],
        vec!["/a b"],
        vec!["/same", "/same"],
        vec!["/"; 17],
    ] {
        assert_eq!(
            make().with_requests(&["GET"], &targets).unwrap_err().code,
            "policy"
        );
    }
    let long = format!("/{}", "a".repeat(256));
    assert!(make().with_requests(&["GET"], &[&long]).is_err());
}
#[test]
fn external_request_grants_require_both_lists_without_changing_legacy_access() {
    let base = r#"{"version":4,"http_access":{"hosts":["localhost"],"origins":[],"allow_missing_origin":true}}"#;
    assert!(DataHostPolicy::from_policy(base.as_bytes()).is_ok());
    for extra in [
        r#", "methods":["GET"], "targets":["/safe"]"#,
        r#", "methods":[], "targets":[]"#,
    ] {
        let text = base.replace("true}", &format!("true{extra}}}"));
        assert!(
            DataHostPolicy::from_policy(text.as_bytes()).is_ok(),
            "{text}"
        );
    }
    for extra in [
        r#", "methods":["GET"]"#,
        r#", "targets":["/safe"]"#,
        r#", "methods":["GET"], "targets":[1]"#,
        r#", "methods":["GET"], "targets":["/safe"], "unknown":0"#,
    ] {
        let text = base.replace("true}", &format!("true{extra}}}"));
        assert!(
            DataHostPolicy::from_policy(text.as_bytes()).is_err(),
            "{text}"
        );
    }
}
#[cfg(target_os = "linux")]
mod sockets {
    use super::*;
    use std::{
        io::{Read, Write},
        net::{Ipv4Addr, TcpStream},
        time::Duration,
    };
    fn call(host: &mut NetworkHostV4, value: JsonValue) -> Result<JsonValue, HostError> {
        host.call(&value, || Ok::<_, ()>(())).unwrap()
    }
    fn json(s: &str) -> JsonValue {
        parse_json(s.as_bytes(), JsonLimits::DEFAULT).unwrap()
    }
    fn number(j: JsonValue) -> i64 {
        let JsonValue::Int(n) = j else { panic!("Int") };
        n
    }
    fn setup(policy: HttpAccessPolicy) -> (NetworkHostV4, i64, TcpStream) {
        setup_limits(policy, HttpExchangeLimits::DEFAULT)
    }
    fn setup_limits(
        policy: HttpAccessPolicy,
        limits: HttpExchangeLimits,
    ) -> (NetworkHostV4, i64, TcpStream) {
        let mut host = NetworkHostV4::new(NetworkLimits::DEFAULT).unwrap();
        host.set_http_access(policy).unwrap();
        host.set_http_limits(limits).unwrap();
        host.grant_port(0).unwrap();
        let listener = number(
            call(
                &mut host,
                json(r#"{"version":4,"op":"net.listen","port":0}"#),
            )
            .unwrap(),
        );
        let port = number(
            call(
                &mut host,
                json(&format!(
                    r#"{{"version":4,"op":"net.port","handle":{listener}}}"#
                )),
            )
            .unwrap(),
        );
        let peer = TcpStream::connect((Ipv4Addr::LOCALHOST, port as u16)).unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let stream = number(
            call(
                &mut host,
                json(&format!(
                    r#"{{"version":4,"op":"net.accept","handle":{listener}}}"#
                )),
            )
            .unwrap(),
        );
        (host, stream, peer)
    }
    #[test]
    fn granted_hosts_origins_and_explicit_absence_publish_only_complete_requests() {
        for (origin, missing) in [("Origin: https://client.test\r\n", false), ("", true)] {
            let policy =
                HttpAccessPolicy::new(&["LOCALHOST:80"], &["https://client.test"], missing)
                    .unwrap();
            let (mut host, handle, mut peer) = setup(policy);
            peer.write_all(
                format!("GET / HTTP/1.1\r\nHost: localhost:80\r\n{origin}\r\n").as_bytes(),
            )
            .unwrap();
            call(
                &mut host,
                json(&format!(
                    r#"{{"version":4,"op":"http.receive","handle":{handle}}}"#
                )),
            )
            .unwrap();
            assert_eq!(host.usage(), (1, 1));
            host.close_all().unwrap();
            assert_eq!(host.usage(), (0, 0));
        }
    }
    #[test]
    fn ungranted_or_duplicate_origins_and_hosts_refuse_before_continue_or_body() {
        for (authority, origin, missing) in [
            ("evil.test:80", "", true),
            ("localhost:81", "", true),
            ("localhost:80", "", false),
            ("localhost:80", "Origin: null\r\n", true),
            ("localhost:80", "Origin: https://evil.test\r\n", true),
            ("localhost:80", "Origin: https://client.test/\r\n", true),
            ("localhost:80", "Origin: https://client.test:443\r\n", true),
            (
                "localhost:80",
                "Origin: https://client.test https://evil.test\r\n",
                true,
            ),
            (
                "localhost:80",
                "Origin: https://client.test\r\nOrigin: https://client.test\r\n",
                true,
            ),
        ] {
            let policy =
                HttpAccessPolicy::new(&["localhost:80"], &["https://client.test"], missing)
                    .unwrap();
            let (mut host, handle, mut peer) = setup(policy);
            peer.write_all(format!("POST / HTTP/1.1\r\nHost: {authority}\r\n{origin}Content-Length: 999\r\nExpect: 100-continue\r\n\r\n").as_bytes()).unwrap();
            assert_eq!(
                call(
                    &mut host,
                    json(&format!(
                        r#"{{"version":4,"op":"http.receive","handle":{handle}}}"#
                    ))
                )
                .unwrap_err()
                .code,
                "authority"
            );
            let mut wire = Vec::new();
            peer.read_to_end(&mut wire).unwrap();
            assert!(wire.starts_with(b"HTTP/1.1 403 Forbidden\r\n"));
            assert_eq!(host.usage(), (1, 0));
            host.close_all().unwrap();
        }
    }
    #[test]
    fn deny_all_and_live_owner_policy_mutation_are_explicit() {
        let (mut host, handle, mut peer) = setup(HttpAccessPolicy::new(&[], &[], true).unwrap());
        assert_eq!(
            host.set_http_access(HttpAccessPolicy::new(&["localhost"], &[], true).unwrap())
                .unwrap_err()
                .code,
            "policy"
        );
        peer.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        assert_eq!(
            call(
                &mut host,
                json(&format!(
                    r#"{{"version":4,"op":"http.receive","handle":{handle}}}"#
                ))
            )
            .unwrap_err()
            .code,
            "authority"
        );
        host.close_all().unwrap();
    }
    #[test]
    fn access_checks_share_head_and_decoder_work_without_sending_continue() {
        let policy = HttpAccessPolicy::new(&["localhost"], &["https://client.test"], true).unwrap();
        let (mut host, handle, mut peer) = setup_limits(
            policy,
            HttpExchangeLimits {
                request: HttpRequestLimits {
                    work: 800,
                    ..HttpRequestLimits::DEFAULT
                },
                ..HttpExchangeLimits::DEFAULT
            },
        );
        peer.write_all(b"POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 1\r\nExpect: 100-continue\r\n\r\n").unwrap();
        assert_eq!(
            call(
                &mut host,
                json(&format!(
                    r#"{{"version":4,"op":"http.receive","handle":{handle}}}"#
                ))
            )
            .unwrap_err()
            .code,
            "limit"
        );
        let mut wire = Vec::new();
        peer.read_to_end(&mut wire).unwrap();
        assert!(wire.starts_with(b"HTTP/1.1 413"));
        assert_eq!(host.usage(), (1, 0));
        host.close_all().unwrap();
    }
    #[test]
    fn exact_methods_and_raw_targets_cannot_be_bypassed_by_case_decoding_or_queries() {
        for (method, target, allowed) in [
            ("GET", "/safe", true),
            ("POST", "/safe", true),
            ("OPTIONS", "*", true),
            ("GET", "/raw%2Fvalue?x=1", true),
            ("get", "/safe", false),
            ("DELETE", "/safe", false),
            ("GET", "/safe?x=1", false),
            ("GET", "/safe/", false),
            ("GET", "/raw/value?x=1", false),
            ("GET", "/raw%2fvalue?x=1", false),
        ] {
            let policy = HttpAccessPolicy::new(&["localhost"], &[], true)
                .unwrap()
                .with_requests(
                    &["GET", "POST", "OPTIONS"],
                    &["/safe", "/raw%2Fvalue?x=1", "*"],
                )
                .unwrap();
            let (mut host, handle, mut peer) = setup(policy);
            let payload = if allowed { "x" } else { "" };
            peer.write_all(format!("{method} {target} HTTP/1.1\r\nHost: localhost\r\nContent-Length: 1\r\nExpect: 100-continue\r\n\r\n{payload}").as_bytes()).unwrap();
            let result = call(
                &mut host,
                json(&format!(
                    r#"{{"version":4,"op":"http.receive","handle":{handle}}}"#
                )),
            );
            if allowed {
                result.unwrap();
                assert_eq!(host.usage(), (1, 1));
            } else {
                assert_eq!(result.unwrap_err().code, "authority");
                let mut wire = Vec::new();
                peer.read_to_end(&mut wire).unwrap();
                assert!(wire.starts_with(b"HTTP/1.1 403 Forbidden\r\n"));
                assert_eq!(host.usage(), (1, 0));
            }
            host.close_all().unwrap();
        }
    }
    #[test]
    fn empty_request_lists_deny_all_and_comparison_cost_uses_shared_work() {
        for (methods, targets, work, code) in [
            (vec![], vec!["/safe"], 2_000_000, "authority"),
            (vec!["GET"], vec![], 2_000_000, "authority"),
            (vec!["GET"], vec!["/safe"], 1100, "limit"),
        ] {
            let policy = HttpAccessPolicy::new(&["localhost"], &[], true)
                .unwrap()
                .with_requests(&methods, &targets)
                .unwrap();
            let (mut host, handle, mut peer) = setup_limits(
                policy,
                HttpExchangeLimits {
                    request: HttpRequestLimits {
                        work,
                        ..HttpRequestLimits::DEFAULT
                    },
                    ..HttpExchangeLimits::DEFAULT
                },
            );
            peer.write_all(b"GET /safe HTTP/1.1\r\nHost: localhost\r\nContent-Length: 1\r\nExpect: 100-continue\r\n\r\n").unwrap();
            assert_eq!(
                call(
                    &mut host,
                    json(&format!(
                        r#"{{"version":4,"op":"http.receive","handle":{handle}}}"#
                    ))
                )
                .unwrap_err()
                .code,
                code
            );
            assert_eq!(host.usage(), (1, 0));
            host.close_all().unwrap();
        }
    }
}
