//! Strict network requests, execution-scoped handles and actual Sovra binary echo.
#![cfg(target_os = "linux")]
use sovra::compiler::{diagnostics::Diagnostics, project::packages::*, v2::*};
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, TcpListener, TcpStream},
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
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
fn host() -> NetworkHostV4 {
    let mut host = NetworkHostV4::new(NetworkLimits {
        milliseconds: 1000,
        ..NetworkLimits::DEFAULT
    })
    .unwrap();
    host.grant_port(0).unwrap();
    host
}
fn listener(host: &mut NetworkHostV4) -> (i64, u16) {
    let handle = integer(call(host, r#"{"version":4,"op":"net.listen","port":0}"#).unwrap());
    let port = integer(
        call(
            host,
            &format!(r#"{{"version":4,"op":"net.port","handle":{handle}}}"#),
        )
        .unwrap(),
    );
    (handle, port as u16)
}
struct Fixture(PathBuf);
impl Fixture {
    fn new(source: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "svr network host {} {}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(
            path.join("sovra.toml"),
            "[project]\nname=\"app\"\nentry=\"main.svr\"\n",
        )
        .unwrap();
        std::fs::write(path.join("main.svr"), source).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn program(source: &str) -> CompiledGenericPackage {
    let f = Fixture::new(source);
    compile_generic_data_package(
        capture_package_sources(&f.0, PackageCaptureLimits::DEFAULT).unwrap(),
        "main",
        GenericPackageCompilationLimits::DEFAULT,
    )
    .unwrap()
}
#[test]
fn strict_schema_grants_and_wrong_kind_refusal_have_no_socket_effects() {
    let mut denied = NetworkHostV4::new(NetworkLimits::DEFAULT).unwrap();
    assert_eq!(
        call(&mut denied, r#"{"version":4,"op":"net.listen","port":0}"#)
            .unwrap_err()
            .code,
        "authority"
    );
    assert_eq!(denied.usage(), (0, 0));
    let mut host = host();
    for text in [
        r#"{"version":3,"op":"net.listen","port":0}"#,
        r#"{"version":4,"op":"net.listen","port":65536}"#,
        r#"{"version":4,"op":"net.listen","port":-1}"#,
        r#"{"version":4,"op":"net.listen","port":0,"extra":true}"#,
        r#"{"version":4,"op":"net.listen"}"#,
    ] {
        assert_eq!(call(&mut host, text).unwrap_err().code, "input");
        assert_eq!(host.usage(), (0, 0));
    }
    assert_eq!(
        call(
            &mut host,
            r#"{"version":4,"op":"net.connect","address":"example.com"}"#
        )
        .unwrap_err()
        .code,
        "unsupported"
    );
    let (handle, _) = listener(&mut host);
    assert_eq!(
        call(
            &mut host,
            &format!(r#"{{"version":4,"op":"net.read","handle":{handle},"maximum":1}}"#)
        )
        .unwrap_err()
        .code,
        "input"
    );
    let duplicate = JsonValue::Object(vec![
        ("version".into(), JsonValue::Int(4)),
        ("op".into(), JsonValue::String("net.close".into())),
        ("handle".into(), JsonValue::Int(handle)),
        ("handle".into(), JsonValue::Int(handle)),
    ]);
    assert_eq!(
        host.call(&duplicate, || Ok::<_, ()>(()))
            .unwrap()
            .unwrap_err()
            .code,
        "input"
    );
    assert_eq!(host.usage(), (1, 0));
    host.close_all().unwrap();
    assert_eq!(host.usage(), (0, 0));
}
#[test]
fn foreign_stale_and_copied_tokens_never_duplicate_or_reopen_owners() {
    let mut first = host();
    let mut second = host();
    let (handle, _) = listener(&mut first);
    let (other, _) = listener(&mut second);
    assert_ne!(handle, other);
    assert_eq!(
        call(
            &mut second,
            &format!(r#"{{"version":4,"op":"net.port","handle":{handle}}}"#)
        )
        .unwrap_err()
        .code,
        "input"
    );
    let close = format!(r#"{{"version":4,"op":"net.close","handle":{handle}}}"#);
    assert_eq!(call(&mut first, &close).unwrap(), JsonValue::Null);
    assert_eq!(call(&mut first, &close).unwrap_err().code, "input");
    let (new, _) = listener(&mut first);
    assert_ne!(new, handle);
    assert_eq!(call(&mut first, &close).unwrap_err().code, "input");
    assert_eq!(first.usage(), (1, 0));
    first.close_all().unwrap();
    second.close_all().unwrap();
}
#[test]
fn cancelled_accept_invalid_base64_and_explicit_close_preserve_scope_ownership() {
    let mut host = host();
    let (handle, port) = listener(&mut host);
    let mut client = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let accept = request(&format!(
        r#"{{"version":4,"op":"net.accept","handle":{handle}}}"#
    ));
    let mut checks = 0;
    let cancelled = host.call(&accept, || {
        checks += 1;
        if checks == 2 {
            Err("cancel accept")
        } else {
            Ok(())
        }
    });
    assert_eq!(cancelled.unwrap_err(), "cancel accept");
    assert_eq!(host.usage(), (1, 0));
    let stream = integer(host.call(&accept, || Ok::<_, ()>(())).unwrap().unwrap());
    assert_eq!(
        call(
            &mut host,
            &format!(r#"{{"version":4,"op":"net.write","handle":{stream},"data":"/x=="}}"#)
        )
        .unwrap_err()
        .code,
        "input"
    );
    client.write_all(&[255]).unwrap();
    assert_eq!(
        call(
            &mut host,
            &format!(r#"{{"version":4,"op":"net.read","handle":{stream},"maximum":1}}"#)
        )
        .unwrap(),
        JsonValue::String("/w==".into())
    );
    assert_eq!(
        call(
            &mut host,
            &format!(r#"{{"version":4,"op":"net.write","handle":{stream},"data":"/w=="}}"#)
        )
        .unwrap(),
        JsonValue::Int(1)
    );
    let mut byte = [0];
    client.read_exact(&mut byte).unwrap();
    assert_eq!(byte, [255]);
    call(
        &mut host,
        &format!(r#"{{"version":4,"op":"net.close","handle":{stream}}}"#),
    )
    .unwrap();
    assert_eq!(client.read(&mut byte).unwrap(), 0);
    host.close_all().unwrap();
    assert_eq!(host.usage(), (0, 0));
}
fn execute(source: &str, host: NetworkHostV4) -> Result<GenericVmOutcome, Diagnostics> {
    execute_generic_data_package_with_network_host(&program(source), GenericVmPolicy::DEFAULT, host)
}
#[test]
fn sovra_source_accepts_echoes_binary_and_scope_exit_closes_both_socket_owners() {
    let mut host = host();
    let (handle, port) = listener(&mut host);
    let peer = std::thread::spawn(move || {
        let mut client = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        client
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        client.write_all(&[255]).unwrap();
        let mut byte = [0];
        client.read_exact(&mut byte).unwrap();
        assert_eq!(byte, [255]);
        assert_eq!(client.read(&mut byte).unwrap(), 0);
    });
    let source = format!(
        r#"
 fn call(text:String)->std::Result<std::JsonValue,std::Error>{{let request=std::json_parse(text)?;return std::host_call(request);}}
 fn main()->std::Result<String,std::Error>{{
 let accepted=call("{{\"version\":4,\"op\":\"net.accept\",\"handle\":{handle}}}")?;
 let handle=std::json_stringify(accepted)?;
 let data=call("{{\"version\":4,\"op\":\"net.read\",\"handle\":"+handle+",\"maximum\":1}}")?;
 let encoded=std::json_stringify(data)?;
 let written=call("{{\"version\":4,\"op\":\"net.write\",\"handle\":"+handle+",\"data\":"+encoded+"}}")?;
 return std::json_stringify(written);
 }}"#
    );
    let out = execute(&source, host).unwrap();
    assert_eq!(out.value().enum_tag(), Some(0));
    assert_eq!(out.value().payload().unwrap().as_str(), Some("1"));
    peer.join().unwrap();
    let rebound = TcpListener::bind((Ipv4Addr::LOCALHOST, port)).unwrap();
    drop(rebound);
}
#[test]
fn source_listen_is_explicit_and_vm_failure_still_releases_live_listener() {
    let source = r#"fn main()->std::Result<String,std::Error>{let request=std::json_parse("{\"version\":4,\"op\":\"net.listen\",\"port\":0}")?;let response=std::host_call(request)?;return std::json_stringify(response);}"#;
    let p = program(source);
    let denied = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
    assert_eq!(denied.value().enum_tag(), Some(1));
    let denied = execute(source, NetworkHostV4::new(NetworkLimits::DEFAULT).unwrap()).unwrap();
    assert_eq!(denied.value().enum_tag(), Some(1));
    assert_eq!(
        denied.value().payload().unwrap().field(0).unwrap().as_str(),
        Some("authority")
    );
    let allowed = execute(source, host()).unwrap();
    assert_eq!(allowed.value().enum_tag(), Some(0));
    assert!(
        allowed
            .value()
            .payload()
            .unwrap()
            .as_str()
            .unwrap()
            .parse::<i64>()
            .unwrap()
            > 0
    );
    let mut host = host();
    let (_, port) = listener(&mut host);
    assert!(execute("fn main(){let fail=1/0;}", host).is_err());
    let rebound = TcpListener::bind((Ipv4Addr::LOCALHOST, port)).unwrap();
    drop(rebound);
}

#[test]
fn vm_instruction_refusal_releases_the_owned_network_scope() {
    let mut host = host();
    let (_, port) = listener(&mut host);
    let mut policy = GenericVmPolicy::DEFAULT;
    policy.runtime.execution.max_instructions = Some(0);
    let result =
        execute_generic_data_package_with_network_host(&program("fn main(){}"), policy, host);
    assert!(result.is_err());
    let rebound = TcpListener::bind((Ipv4Addr::LOCALHOST, port)).unwrap();
    drop(rebound);
}

#[test]
fn try_accept_yields_without_owners_and_preserves_queued_peers_at_capacity() {
    let mut h = NetworkHostV4::new(NetworkLimits {
        streams: 1,
        milliseconds: 1000,
        ..NetworkLimits::DEFAULT
    })
    .unwrap();
    h.grant_port(0).unwrap();
    let (l, p) = listener(&mut h);
    let accept = format!(r#"{{"version":4,"op":"net.try_accept","handle":{l}}}"#);
    // Empty attempts must not consume stream slots or globally unique handle IDs.
    for _ in 0..100 {
        assert_eq!(call(&mut h, &accept).unwrap(), JsonValue::Null);
        assert_eq!(h.usage(), (1, 0));
    }
    let mut first = TcpStream::connect((Ipv4Addr::LOCALHOST, p)).unwrap();
    first
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let a = integer(call(&mut h, &accept).unwrap());
    assert!(a > l);
    let mut second = TcpStream::connect((Ipv4Addr::LOCALHOST, p)).unwrap();
    second
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    second.write_all(b"queued").unwrap();
    assert_eq!(call(&mut h, &accept).unwrap_err().code, "limit");
    assert_eq!(h.usage(), (1, 1));
    call(
        &mut h,
        &format!(r#"{{"version":4,"op":"net.close","handle":{a}}}"#),
    )
    .unwrap();
    let b = integer(call(&mut h, &accept).unwrap());
    let received = call(
        &mut h,
        &format!(r#"{{"version":4,"op":"net.read","handle":{b},"maximum":6}}"#),
    )
    .unwrap();
    assert_eq!(received, JsonValue::String("cXVldWVk".into()));
    assert_eq!(
        call(
            &mut h,
            &format!(r#"{{"version":4,"op":"net.try_accept","handle":{b}}}"#)
        )
        .unwrap_err()
        .code,
        "input"
    );
    h.close_all().unwrap();
    let mut bytes = Vec::new();
    first.read_to_end(&mut bytes).unwrap();
    assert!(bytes.is_empty());
    assert_eq!(h.usage(), (0, 0));
}

#[test]
fn try_accept_cancellation_releases_unpublished_stream_and_reservation() {
    let mut h = host();
    let (l, p) = listener(&mut h);
    let mut peer = TcpStream::connect((Ipv4Addr::LOCALHOST, p)).unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    let request = request(&format!(
        r#"{{"version":4,"op":"net.try_accept","handle":{l}}}"#
    ));
    let mut ticks = 0;
    assert_eq!(
        h.call(&request, || {
            ticks += 1;
            if ticks == 3 {
                Err("cancel publication")
            } else {
                Ok(())
            }
        })
        .unwrap_err(),
        "cancel publication"
    );
    assert_eq!(ticks, 3);
    assert_eq!(h.usage(), (1, 0));
    let mut wire = Vec::new();
    peer.read_to_end(&mut wire).unwrap();
    assert!(wire.is_empty());
    assert_eq!(
        h.call(&request, || Ok::<_, ()>(())).unwrap().unwrap(),
        JsonValue::Null
    );
    h.close_all().unwrap();
}

#[test]
fn try_accept_zero_deadline_and_zero_capacity_refuse_without_dequeuing() {
    for (milliseconds, streams, code) in [(0, 1, "timeout"), (1000, 0, "limit")] {
        let mut h = NetworkHostV4::new(NetworkLimits {
            milliseconds,
            streams,
            ..NetworkLimits::DEFAULT
        })
        .unwrap();
        h.grant_port(0).unwrap();
        let (l, p) = listener(&mut h);
        let _peer = TcpStream::connect((Ipv4Addr::LOCALHOST, p)).unwrap();
        assert_eq!(
            call(
                &mut h,
                &format!(r#"{{"version":4,"op":"net.try_accept","handle":{l}}}"#)
            )
            .unwrap_err()
            .code,
            code
        );
        assert_eq!(h.usage(), (1, 0));
        h.close_all().unwrap();
    }
}

#[test]
fn checked_sovra_try_accept_distinguishes_idle_from_owned_connection() {
    for ready in [false, true] {
        let mut h = host();
        let (l, p) = listener(&mut h);
        let _peer = ready.then(|| TcpStream::connect((Ipv4Addr::LOCALHOST, p)).unwrap());
        let source = format!(
            r#"
fn main()->std::Result<String,std::Error>{{
 let request=std::json_parse("{{\"version\":4,\"op\":\"net.try_accept\",\"handle\":{l}}}")?;
 let accepted=std::host_call(request)?;
 return std::json_stringify(accepted);
}}"#
        );
        let out = execute(&source, h).unwrap();
        assert_eq!(out.value().enum_tag(), Some(0));
        let value = out.value().payload().unwrap().as_str().unwrap();
        if ready {
            assert!(value.parse::<i64>().unwrap() > l);
        } else {
            assert_eq!(value, "null");
        }
        drop(TcpListener::bind((Ipv4Addr::LOCALHOST, p)).unwrap());
    }
}
