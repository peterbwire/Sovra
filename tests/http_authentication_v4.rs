//! Trusted verifier boundary tests. Fixture decisions are not a production credential backend.
#![cfg(target_os = "linux")]
use sovra::compiler::{project::packages::*, v2::*};
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, TcpStream},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
const TOKEN: &str = "non-production-fixture-token";
struct FixtureVerifier {
    calls: Arc<AtomicUsize>,
    mode: u8,
    late: bool,
}
impl HttpBearerVerifier for FixtureVerifier {
    fn verify(&mut self, input: HttpBearerRequest<'_>) -> HttpAuthenticationDecision {
        self.calls.fetch_add(1, Ordering::Relaxed);
        assert_eq!(input.method(), "POST");
        assert_eq!(input.target(), "/safe");
        assert_eq!(input.credential(), TOKEN.as_bytes());
        assert!(!format!("{input:?}").contains(TOKEN));
        if self.late {
            std::thread::sleep(Duration::from_millis(30));
        }
        match self.mode {
            0 => HttpAuthenticationDecision::Granted(HttpPrincipal::new("operator-1").unwrap()),
            1 => HttpAuthenticationDecision::Unauthenticated,
            2 => HttpAuthenticationDecision::Forbidden,
            _ => HttpAuthenticationDecision::Unavailable,
        }
    }
}
fn json(s: &str) -> JsonValue {
    parse_json(s.as_bytes(), JsonLimits::DEFAULT).unwrap()
}
fn call(h: &mut NetworkHostV4, s: &str) -> Result<JsonValue, HostError> {
    h.call(&json(s), || Ok::<_, ()>(())).unwrap()
}
fn int(j: JsonValue) -> i64 {
    let JsonValue::Int(n) = j else { panic!("Int") };
    n
}
fn receive(h: &mut NetworkHostV4, n: i64) -> Result<JsonValue, HostError> {
    call(
        h,
        &format!(r#"{{"version":4,"op":"http.receive","handle":{n}}}"#),
    )
}
fn setup(
    mode: u8,
    late: bool,
    limits: HttpExchangeLimits,
) -> (NetworkHostV4, i64, TcpStream, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut h = NetworkHostV4::new(NetworkLimits::DEFAULT).unwrap();
    h.set_http_limits(limits).unwrap();
    h.set_http_access(HttpAccessPolicy::new(&["localhost"], &[], true).unwrap())
        .unwrap();
    h.set_http_bearer_verifier(Box::new(FixtureVerifier {
        calls: calls.clone(),
        mode,
        late,
    }))
    .unwrap();
    h.grant_port(0).unwrap();
    let listener = int(call(&mut h, r#"{"version":4,"op":"net.listen","port":0}"#).unwrap());
    let port = int(call(
        &mut h,
        &format!(r#"{{"version":4,"op":"net.port","handle":{listener}}}"#),
    )
    .unwrap());
    let peer = TcpStream::connect((Ipv4Addr::LOCALHOST, port as u16)).unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    let handle = int(call(
        &mut h,
        &format!(r#"{{"version":4,"op":"net.accept","handle":{listener}}}"#),
    )
    .unwrap());
    (h, handle, peer, calls)
}
fn head(fields: &str) -> String {
    format!("POST /safe HTTP/1.1\r\nHost: localhost\r\nContent-Length: 1\r\n{fields}\r\n")
}
fn credential() -> String {
    format!("Authorization: Bearer {TOKEN}\r\n")
}
fn field<'a>(j: &'a JsonValue, name: &str) -> &'a JsonValue {
    let JsonValue::Object(v) = j else {
        panic!("Object")
    };
    &v.iter().find(|(k, _)| k == name).unwrap().1
}
#[test]
fn principal_names_and_registration_are_bounded_and_private() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<NetworkHostV4>();
    for name in ["", "bad name", "token\n", "用户"] {
        assert!(HttpPrincipal::new(name).is_err());
    }
    assert!(HttpPrincipal::new(&"a".repeat(64)).is_ok());
    assert!(HttpPrincipal::new(&"a".repeat(65)).is_err());
    let mut h = NetworkHostV4::new(NetworkLimits::DEFAULT).unwrap();
    let make = || {
        Box::new(FixtureVerifier {
            calls: Arc::new(AtomicUsize::new(0)),
            mode: 0,
            late: false,
        })
    };
    assert_eq!(
        h.set_http_bearer_verifier(make()).unwrap_err().code,
        "policy"
    );
    let (mut h, _, _, _) = setup(0, false, HttpExchangeLimits::DEFAULT);
    assert_eq!(
        h.set_http_bearer_verifier(make()).unwrap_err().code,
        "policy"
    );
    assert!(format!("{h:?}").contains("[opaque]"));
    assert!(!format!("{h:?}").contains(TOKEN));
    assert!(DataHostPolicy::from_policy(br#"{"version":4,"http_authentication":{}}"#).is_err());
    h.close_all().unwrap();
}
#[test]
fn absent_malformed_or_duplicate_credentials_never_call_provider_or_send_continue() {
    for fields in [
        "".to_string(),
        "Authorization: Basic abc\r\n".into(),
        "Authorization: Bearer a b\r\n".into(),
        "Authorization: Bearer ===\r\n".into(),
        format!("Authorization: Bearer {}\r\n", "a".repeat(257)),
        format!("{}{}", credential(), credential()),
    ] {
        let (mut h, n, mut peer, calls) = setup(0, false, HttpExchangeLimits::DEFAULT);
        peer.write_all(head(&(fields + "Expect: 100-continue\r\n")).as_bytes())
            .unwrap();
        assert_eq!(receive(&mut h, n).unwrap_err().code, "authentication");
        let mut wire = Vec::new();
        peer.read_to_end(&mut wire).unwrap();
        let text = String::from_utf8(wire).unwrap();
        assert!(text.starts_with("HTTP/1.1 401 Unauthorized\r\n"));
        assert!(text.contains("WWW-Authenticate: Bearer realm=\"Sovra\"\r\n"));
        assert!(!text.contains(TOKEN));
        assert_eq!(calls.load(Ordering::Relaxed), 0);
        assert_eq!(h.usage(), (1, 0));
        h.close_all().unwrap();
    }
}
#[test]
fn provider_refusals_are_fixed_final_responses_without_body_wait() {
    for (mode, code, status) in [
        (1, "authentication", "401"),
        (2, "authority", "403"),
        (3, "authentication_unavailable", "503"),
    ] {
        let (mut h, n, mut peer, calls) = setup(mode, false, HttpExchangeLimits::DEFAULT);
        peer.write_all(head(&(credential() + "Expect: 100-continue\r\n")).as_bytes())
            .unwrap();
        assert_eq!(receive(&mut h, n).unwrap_err().code, code);
        let mut wire = Vec::new();
        peer.read_to_end(&mut wire).unwrap();
        assert_eq!(&wire[9..12], status.as_bytes());
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        assert_eq!(h.usage(), (1, 0));
        h.close_all().unwrap();
    }
}
#[test]
fn authenticated_result_contains_public_principal_without_authorization_header() {
    let (mut h, n, mut peer, calls) = setup(0, false, HttpExchangeLimits::DEFAULT);
    peer.write_all((head(&(credential() + "X-Principal: forged\r\n")) + "x").as_bytes())
        .unwrap();
    let result = receive(&mut h, n).unwrap();
    assert_eq!(
        field(&result, "principal"),
        &JsonValue::String("operator-1".into())
    );
    let text = serialize_json(&result, JsonLimits::DEFAULT).unwrap();
    assert!(!text.to_ascii_lowercase().contains("authorization"));
    assert!(!text.contains(TOKEN));
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    h.close_all().unwrap();
    let mut wire = Vec::new();
    peer.read_to_end(&mut wire).unwrap();
    assert!(wire.is_empty());
}
#[test]
fn authenticated_continue_waits_for_verified_head_and_complete_body() {
    let (mut h, n, mut peer, calls) = setup(0, false, HttpExchangeLimits::DEFAULT);
    let head = head(&(credential() + "Expect: 100-continue\r\n"));
    let client = std::thread::spawn(move || {
        peer.write_all(head.as_bytes()).unwrap();
        let mut wire = [0; 25];
        peer.read_exact(&mut wire).unwrap();
        assert_eq!(&wire, b"HTTP/1.1 100 Continue\r\n\r\n");
        peer.write_all(b"x").unwrap();
        let mut remaining = Vec::new();
        peer.read_to_end(&mut remaining).unwrap();
    });
    let result = receive(&mut h, n).unwrap();
    assert_eq!(field(&result, "body"), &JsonValue::String("eA==".into()));
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    h.close_all().unwrap();
    client.join().unwrap();
}
#[test]
fn access_and_shared_work_refusals_precede_provider_calls() {
    for (work, authority, code) in [
        (2_000_000, "evil.test", "authority"),
        (1900, "localhost", "limit"),
    ] {
        let (mut h, n, mut peer, calls) = setup(
            0,
            false,
            HttpExchangeLimits {
                request: HttpRequestLimits {
                    work,
                    ..HttpRequestLimits::DEFAULT
                },
                ..HttpExchangeLimits::DEFAULT
            },
        );
        peer.write_all(
            head(&credential())
                .replace("Host: localhost", &format!("Host: {authority}"))
                .as_bytes(),
        )
        .unwrap();
        assert_eq!(receive(&mut h, n).unwrap_err().code, code);
        assert_eq!(calls.load(Ordering::Relaxed), 0);
        assert_eq!(h.usage(), (1, 0));
        h.close_all().unwrap();
    }
}
#[test]
fn post_provider_cancellation_and_timeout_never_publish_identity_or_continue() {
    let (mut h, n, mut peer, calls) = setup(0, false, HttpExchangeLimits::DEFAULT);
    peer.write_all(head(&(credential() + "Expect: 100-continue\r\n")).as_bytes())
        .unwrap();
    assert_eq!(
        h.call(
            &json(&format!(
                r#"{{"version":4,"op":"http.receive","handle":{n}}}"#
            )),
            || if calls.load(Ordering::Relaxed) > 0 {
                Err("cancel after provider")
            } else {
                Ok(())
            }
        )
        .unwrap_err(),
        "cancel after provider"
    );
    assert_eq!(h.usage(), (1, 0));
    let mut wire = Vec::new();
    peer.read_to_end(&mut wire).unwrap();
    assert!(wire.is_empty());
    h.close_all().unwrap();
    let (mut h, n, mut peer, calls) = setup(
        0,
        true,
        HttpExchangeLimits {
            milliseconds: 20,
            ..HttpExchangeLimits::DEFAULT
        },
    );
    peer.write_all(head(&(credential() + "Expect: 100-continue\r\n")).as_bytes())
        .unwrap();
    assert_eq!(receive(&mut h, n).unwrap_err().code, "timeout");
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    let mut wire = Vec::new();
    peer.read_to_end(&mut wire).unwrap();
    assert!(wire.is_empty());
    assert_eq!(h.usage(), (1, 0));
    h.close_all().unwrap();
}
#[test]
fn authenticated_principal_reaches_actual_sovra_execution_without_credentials() {
    let (h, n, mut peer, _) = setup(0, false, HttpExchangeLimits::DEFAULT);
    peer.write_all((head(&credential()) + "x").as_bytes())
        .unwrap();
    let root = std::env::temp_dir().join(format!("svr-auth-source-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("sovra.toml"),
        "[project]\nname=\"auth\"\nentry=\"main.svr\"\n",
    )
    .unwrap();
    let request =
        format!(r#"{{"version":4,"op":"http.receive","handle":{n}}}"#).replace('"', "\\\"");
    let source = format!(
        r#"fn main() -> std::Result<String, std::Error> {{
let input = std::json_parse("{request}")?;
let request = std::host_call(input)?;
return std::json_stringify(request);
}}"#
    );
    std::fs::write(root.join("main.svr"), source).unwrap();
    let program = compile_generic_data_package(
        capture_package_sources(&root, PackageCaptureLimits::DEFAULT).unwrap(),
        "main",
        GenericPackageCompilationLimits::DEFAULT,
    )
    .unwrap();
    let outcome =
        execute_generic_data_package_with_network_host(&program, GenericVmPolicy::DEFAULT, h)
            .unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    let text = format!("{outcome:?}");
    assert!(text.contains("operator-1"));
    assert!(!text.contains(TOKEN));
    assert!(!text.to_ascii_lowercase().contains("authorization"));
    let mut wire = Vec::new();
    peer.read_to_end(&mut wire).unwrap();
    assert!(wire.is_empty());
}
