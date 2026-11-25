//! External verifier protocol and owned-process controls; backend code is a non-production fixture.
#![cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
use sovra::compiler::v2::*;
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, TcpStream},
    path::PathBuf,
    process::Command,
    time::Duration,
};
struct Fixture(PathBuf);
impl Fixture {
    fn new(code: &str) -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "svr-command-auth-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(p.join("backend.py"), code).unwrap();
        Self(p)
    }
    fn verifier(&self, ms: u64) -> HttpCommandVerifier {
        let output = Command::new("python3")
            .args(["-c", "import sys;print(sys.executable)"])
            .output()
            .unwrap();
        let path = PathBuf::from(String::from_utf8(output.stdout).unwrap().trim());
        HttpCommandVerifier::new(
            &path,
            &self.0,
            &[self.0.join("backend.py").to_str().unwrap().into()],
            ms,
        )
        .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn j(text: &str) -> JsonValue {
    parse_json(text.as_bytes(), JsonLimits::DEFAULT).unwrap()
}
fn call(h: &mut NetworkHostV4, text: &str) -> Result<JsonValue, HostError> {
    h.call(&j(text), || Ok::<_, ()>(())).unwrap()
}
fn int(j: JsonValue) -> i64 {
    let JsonValue::Int(n) = j else { panic!("Int") };
    n
}
fn setup(verifier: HttpCommandVerifier, ms: u64) -> (NetworkHostV4, i64, TcpStream) {
    let mut h = NetworkHostV4::new(NetworkLimits::DEFAULT).unwrap();
    h.set_http_limits(HttpExchangeLimits {
        milliseconds: ms,
        ..HttpExchangeLimits::DEFAULT
    })
    .unwrap();
    h.set_http_access(HttpAccessPolicy::new(&["localhost"], &[], true).unwrap())
        .unwrap();
    h.set_http_bearer_verifier(Box::new(verifier)).unwrap();
    h.grant_port(0).unwrap();
    let l = int(call(&mut h, r#"{"version":4,"op":"net.listen","port":0}"#).unwrap());
    let p = int(call(
        &mut h,
        &format!(r#"{{"version":4,"op":"net.port","handle":{l}}}"#),
    )
    .unwrap());
    let s = TcpStream::connect((Ipv4Addr::LOCALHOST, p as u16)).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    let n = int(call(
        &mut h,
        &format!(r#"{{"version":4,"op":"net.accept","handle":{l}}}"#),
    )
    .unwrap());
    (h, n, s)
}
fn request() -> &'static [u8] {
    b"POST /safe HTTP/1.1\r\nHost: localhost\r\nContent-Length: 1\r\nAuthorization: Bearer fixture-credential\r\nExpect: 100-continue\r\n\r\n"
}
fn receive(n: i64) -> String {
    format!(r#"{{"version":4,"op":"http.receive","handle":{n}}}"#)
}
#[test]
fn strict_protocol_grants_principal_and_redacts_credentials() {
    let f = Fixture::new(
        r#"import json,sys,os
r=json.load(sys.stdin)
assert set(r)=={'version','credential','method','target','authority','origin','transport'}
assert r=={'version':1,'credential':'fixture-credential','method':'POST','target':'/safe','authority':'localhost','origin':None,'transport':'loopback-plaintext'}
assert 'PATH' not in os.environ
print(json.dumps({'version':1,'decision':'grant','principal':'process-client'}))
"#,
    );
    let v = f.verifier(1000);
    assert!(!format!("{v:?}").contains("backend.py"));
    let (mut h, n, mut peer) = setup(v, 5000);
    peer.write_all(request()).unwrap();
    peer.write_all(b"x").unwrap();
    let value = call(&mut h, &receive(n)).unwrap();
    let text = serialize_json(&value, JsonLimits::DEFAULT).unwrap();
    assert!(text.contains("process-client"));
    assert!(!text.contains("fixture-credential"));
    assert!(!text.to_ascii_lowercase().contains("authorization"));
    assert!(call(
        &mut h,
        r#"{"version":3,"op":"process.run","program":"http-verifier","arguments":[],"input":""}"#
    )
    .is_err());
    h.close_all().unwrap();
}
#[test]
fn malformed_failed_or_oversized_responses_never_grant() {
    for code in ["print('{}')", "print('{\"version\":2,\"decision\":\"grant\",\"principal\":\"client\"}')", "print('{\"version\":1,\"decision\":\"grant\",\"principal\":\"client\",\"credential\":\"echo\"}')", "print('{\"version\":1,\"decision\":\"grant\",\"principal\":\"bad name\"}')", "print('{\"version\":1,\"version\":1,\"decision\":\"grant\",\"principal\":\"client\"}')", "print('x'*4097)", "import sys;print('{\"version\":1,\"decision\":\"grant\",\"principal\":\"client\"}');sys.exit(1)"] {
 let f=Fixture::new(code);let (mut h,n,mut peer)=setup(f.verifier(1000),5000);peer.write_all(request()).unwrap();assert_eq!(call(&mut h,&receive(n)).unwrap_err().code,"authentication_unavailable");let mut wire=Vec::new();peer.read_to_end(&mut wire).unwrap();assert!(wire.starts_with(b"HTTP/1.1 503 Service Unavailable\r\n"));assert_eq!(h.usage(),(1,0));h.close_all().unwrap();}
}
#[test]
fn explicit_denial_decisions_map_to_fixed_final_responses() {
    for (decision, status, code) in [
        ("unauthenticated", "401", "authentication"),
        ("forbidden", "403", "authority"),
        ("unavailable", "503", "authentication_unavailable"),
    ] {
        let f = Fixture::new(&format!(
            "print('{{\"version\":1,\"decision\":\"{decision}\"}}')"
        ));
        let (mut h, n, mut peer) = setup(f.verifier(1000), 5000);
        peer.write_all(request()).unwrap();
        assert_eq!(call(&mut h, &receive(n)).unwrap_err().code, code);
        let mut wire = Vec::new();
        peer.read_to_end(&mut wire).unwrap();
        assert_eq!(&wire[9..12], status.as_bytes());
        h.close_all().unwrap();
    }
}
#[test]
fn child_timeout_cancellation_and_http_deadline_cleanup_are_distinct() {
    for kind in [0, 1, 2] {
        let f =
            Fixture::new("import os,time\nopen('pid','w').write(str(os.getpid()))\ntime.sleep(10)");
        let (mut h, n, mut peer) = setup(
            f.verifier(if kind == 0 { 100 } else { 1000 }),
            if kind == 2 { 100 } else { 5000 },
        );
        peer.write_all(request()).unwrap();
        if kind == 1 {
            let result = h.call(&j(&receive(n)), || {
                if observed_pid(&f).is_some() {
                    Err("cancel command verifier")
                } else {
                    Ok(())
                }
            });
            assert_eq!(result.unwrap_err(), "cancel command verifier");
        } else {
            assert_eq!(
                call(&mut h, &receive(n)).unwrap_err().code,
                if kind == 0 {
                    "authentication_unavailable"
                } else {
                    "timeout"
                }
            );
        }
        if let Ok(pid) = std::fs::read_to_string(f.0.join("pid")) {
            assert!(!std::path::Path::new("/proc").join(pid.trim()).exists());
        }
        assert_eq!(h.usage(), (1, 0));
        h.close_all().unwrap();
    }
}
#[test]
fn configuration_is_bounded_absolute_and_requires_http_access() {
    let cwd = std::env::temp_dir();
    for args in [
        vec!["x".to_string(); 17],
        vec!["x".repeat(8192)],
        vec!["bad\0arg".into()],
    ] {
        assert!(
            HttpCommandVerifier::new(std::path::Path::new("/bin/cat"), &cwd, &args, 1000).is_err()
        );
    }
    assert!(HttpCommandVerifier::new(std::path::Path::new("relative"), &cwd, &[], 1000).is_err());
    assert!(HttpCommandVerifier::new(std::path::Path::new("/bin/cat"), &cwd, &[], 1001).is_err());
    for text in [
        r#"{"version":4,"http_authentication":{}}"#,
        r#"{"version":4,"http_authentication":{"executable":"/bin/cat","cwd":"/tmp","arguments":[],"milliseconds":1000}}"#,
        r#"{"version":4,"http_access":{"hosts":[],"origins":[],"allow_missing_origin":true},"http_authentication":{"executable":"relative","cwd":"/tmp","arguments":[],"milliseconds":1000}}"#,
    ] {
        assert!(DataHostPolicy::from_policy(text.as_bytes()).is_err());
    }
}

fn prepared(h: &mut NetworkHostV4, n: i64) {
    let poll = format!(r#"{{"version":4,"op":"http.poll_head","handle":{n}}}"#);
    let mut ready = false;
    for _ in 0..100 {
        ready = call(h, &poll).unwrap() == JsonValue::Bool(true);
        if ready {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(ready);
}
fn extra(h: &mut NetworkHostV4) -> (i64, TcpStream) {
    let l = int(call(h, r#"{"version":4,"op":"net.listen","port":0}"#).unwrap());
    let p = int(call(
        h,
        &format!(r#"{{"version":4,"op":"net.port","handle":{l}}}"#),
    )
    .unwrap());
    let s = TcpStream::connect((Ipv4Addr::LOCALHOST, p as u16)).unwrap();
    let n = int(call(
        h,
        &format!(r#"{{"version":4,"op":"net.accept","handle":{l}}}"#),
    )
    .unwrap());
    (n, s)
}
fn body_poll(n: i64) -> String {
    format!(r#"{{"version":4,"op":"http.poll_body","handle":{n}}}"#)
}
fn complete(h: &mut NetworkHostV4, n: i64) {
    let mut ready = false;
    for _ in 0..500 {
        ready = call(h, &body_poll(n)).unwrap() == JsonValue::Bool(true);
        if ready {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(ready);
}
#[test]
fn command_jobs_yield_slow_verification_while_another_request_completes() {
    let f = Fixture::new(
        r#"import json,sys,pathlib,time
r=json.load(sys.stdin)
with pathlib.Path('calls').open('a') as out:out.write(r['target']+'\n')
if r['target']=='/slow':
 while not pathlib.Path('release').exists():time.sleep(.002)
print(json.dumps({'version':1,'decision':'grant','principal':'process-client'}))
"#,
    );
    let (mut h, slow, mut slow_peer) = setup(f.verifier(1000), 5000);
    let wire = String::from_utf8(request().to_vec())
        .unwrap()
        .replace("/safe", "/slow")
        .replace("Expect: 100-continue\r\n", "");
    slow_peer.write_all(wire.as_bytes()).unwrap();
    slow_peer.write_all(b"x").unwrap();
    prepared(&mut h, slow);
    assert_eq!(
        call(&mut h, &body_poll(slow)).unwrap(),
        JsonValue::Bool(false)
    );
    assert!(call(&mut h, &receive(slow)).is_err());
    assert!(!format!("{h:?}").contains("fixture-credential"));
    let (fast, mut fast_peer) = extra(&mut h);
    let wire = String::from_utf8(request().to_vec())
        .unwrap()
        .replace("Expect: 100-continue\r\n", "");
    fast_peer.write_all(wire.as_bytes()).unwrap();
    fast_peer.write_all(b"x").unwrap();
    prepared(&mut h, fast);
    complete(&mut h, fast);
    let value = call(&mut h, &receive(fast)).unwrap();
    assert!(format!("{value:?}").contains("process-client"));
    assert!(!f.0.join("release").exists());
    std::fs::write(f.0.join("release"), b"go").unwrap();
    complete(&mut h, slow);
    call(&mut h, &receive(slow)).unwrap();
    let calls = std::fs::read_to_string(f.0.join("calls")).unwrap();
    assert_eq!(calls.lines().count(), 2);
    h.close_all().unwrap();
}
#[test]
fn pending_command_close_kills_reaps_and_releases_capacity() {
    let f = Fixture::new(
        r#"import json,sys,pathlib,os,time
r=json.load(sys.stdin)
pathlib.Path('pid').write_text(str(os.getpid()))
while True:time.sleep(.01)
"#,
    );
    let (mut h, n, mut peer) = setup(f.verifier(1000), 5000);
    peer.write_all(request()).unwrap();
    prepared(&mut h, n);
    assert_eq!(call(&mut h, &body_poll(n)).unwrap(), JsonValue::Bool(false));
    for _ in 0..100 {
        if observed_pid(&f).is_some() {
            break;
        }
        assert_eq!(call(&mut h, &body_poll(n)).unwrap(), JsonValue::Bool(false));
        std::thread::sleep(Duration::from_millis(1));
    }
    let pid = observed_pid(&f).expect("verifier published a positive PID");
    call(
        &mut h,
        &format!(r#"{{"version":4,"op":"net.close","handle":{n}}}"#),
    )
    .unwrap();
    assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
    assert_eq!(h.usage(), (1, 0));
    h.close_all().unwrap();
}
#[test]
fn pending_command_jobs_have_a_two_child_ceiling() {
    let f = Fixture::new("import json,sys,time\nr=json.load(sys.stdin)\ntime.sleep(10)\n");
    let (mut h, first, mut a) = setup(f.verifier(1000), 5000);
    a.write_all(request()).unwrap();
    prepared(&mut h, first);
    assert_eq!(
        call(&mut h, &body_poll(first)).unwrap(),
        JsonValue::Bool(false)
    );
    let (second, mut b) = extra(&mut h);
    b.write_all(request()).unwrap();
    prepared(&mut h, second);
    assert_eq!(
        call(&mut h, &body_poll(second)).unwrap(),
        JsonValue::Bool(false)
    );
    let (third, mut c) = extra(&mut h);
    c.write_all(request()).unwrap();
    prepared(&mut h, third);
    assert_eq!(
        call(&mut h, &body_poll(third)).unwrap_err().code,
        "authentication_unavailable"
    );
    call(
        &mut h,
        &format!(r#"{{"version":4,"op":"net.close","handle":{first}}}"#),
    )
    .unwrap();
    let (fourth, mut d) = extra(&mut h);
    d.write_all(request()).unwrap();
    prepared(&mut h, fourth);
    assert_eq!(
        call(&mut h, &body_poll(fourth)).unwrap(),
        JsonValue::Bool(false)
    );
    h.close_all().unwrap();
}

#[test]
fn pending_command_timeout_and_claimed_cancellation_kill_and_reap() {
    for cancel in [false, true] {
        let f=Fixture::new("import json,sys,pathlib,os,time\nr=json.load(sys.stdin)\npathlib.Path('pid').write_text(str(os.getpid()))\nwhile True:time.sleep(.01)\n");
        let (mut h, n, mut peer) = setup(f.verifier(1000), 5000);
        peer.write_all(request()).unwrap();
        prepared(&mut h, n);
        assert_eq!(call(&mut h, &body_poll(n)).unwrap(), JsonValue::Bool(false));
        for _ in 0..200 {
            if observed_pid(&f).is_some() {
                break;
            }
            assert_eq!(call(&mut h, &body_poll(n)).unwrap(), JsonValue::Bool(false));
            std::thread::sleep(Duration::from_millis(1));
        }
        let pid = observed_pid(&f).expect("verifier published a positive PID");
        if cancel {
            let mut ticks = 0;
            assert_eq!(
                h.call(&j(&body_poll(n)), || {
                    ticks += 1;
                    if ticks == 2 {
                        Err("cancel verification")
                    } else {
                        Ok(())
                    }
                })
                .unwrap_err(),
                "cancel verification"
            );
        } else {
            std::thread::sleep(Duration::from_millis(1100));
            assert_eq!(
                call(&mut h, &body_poll(n)).unwrap_err().code,
                "authentication_unavailable"
            );
        }
        assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
        assert_eq!(h.usage(), (1, 0));
        h.close_all().unwrap();
    }
}

// Creating the PID file precedes writing its contents; existence alone races.
fn observed_pid(fixture: &Fixture) -> Option<u32> {
    std::fs::read_to_string(fixture.0.join("pid"))
        .ok()?
        .parse::<u32>()
        .ok()
        .filter(|pid| *pid > 0)
}
