//! Independent CLI/native authenticated consumers using an explicitly delegated test-only backend.
#![cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
use sovra::compiler::v2::{parse_json, serialize_json, JsonLimits, JsonValue as J};
const FIXTURE_SECRET: &str = "fixture-command-authentication-secret-2026-opaque-test-only";
use std::{
    net::{Ipv4Addr, TcpListener},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};
struct Fixture {
    root: PathBuf,
    port: u16,
}
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "svr http server {} {}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(root.join("app")).unwrap();
        let probe = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        std::fs::write(
            root.join("app/sovra.toml"),
            include_str!("../examples/http-loopback/app/sovra.toml"),
        )
        .unwrap();
        std::fs::write(
            root.join("app/main.svr"),
            include_str!("../examples/http-loopback/app/main.svr")
                .replace("47123", &port.to_string())
                .replace("serve(13)", "serve(6)")
                .replace("    if target == \"/health\"", "    if target == \"/identity\" { let output = std::json_stringify(request)?; return std::Result::<Reply<String>, std::Error>::Ok(Reply::<String>{status: 200, body: output}); }\n    if target == \"/health\""),
        )
        .unwrap();
        std::fs::write(root.join("key"), FIXTURE_SECRET).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(root.join("key"), std::fs::Permissions::from_mode(0o600)).unwrap();
        std::fs::write(
            root.join("backend.py"),
            r#"import json,sys,pathlib,hmac,os,time
r=json.load(sys.stdin)
assert set(r)=={'version','method','target','authority','origin','transport','credential'}
assert r['version']==1 and r['transport']=='loopback-plaintext' and r['origin'] in (None,'https://client.example')
assert 'PATH' not in os.environ
with pathlib.Path('invocations').open('a') as out:out.write('x')
if r['origin'] is not None:
 while not pathlib.Path('release').exists():time.sleep(.002)
key=pathlib.Path('key').read_text()
if not hmac.compare_digest(r['credential'],key):out={'version':1,'decision':'unauthenticated'}
elif r['method']!='GET' or r['target']!='/identity':out={'version':1,'decision':'forbidden'}
else:out={'version':1,'decision':'grant','principal':'fixture-client'}
print(json.dumps(out))
"#,
        )
        .unwrap();
        let python = Command::new("python3")
            .args(["-c", "import sys;print(sys.executable)"])
            .output()
            .unwrap();
        let executable = String::from_utf8(python.stdout).unwrap().trim().to_string();
        let raw = include_str!("../examples/http-loopback/policy.json")
            .replace("47123", &port.to_string());
        let mut policy = parse_json(raw.as_bytes(), JsonLimits::DEFAULT).unwrap();
        let J::Object(fields) = &mut policy else {
            panic!("policy")
        };
        let J::Object(access) = &mut fields
            .iter_mut()
            .find(|(k, _)| k == "http_access")
            .unwrap()
            .1
        else {
            panic!("access")
        };
        access.iter_mut().find(|(k, _)| k == "targets").unwrap().1 = J::Array(vec![
            J::String("/identity".into()),
            J::String("/echo".into()),
        ]);
        fields.push((
            "http_admission".into(),
            J::Object(vec![
                ("requests".into(), J::Int(5)),
                ("milliseconds".into(), J::Int(60000)),
            ]),
        ));
        fields.push((
            "http_authentication".into(),
            J::Object(vec![
                ("executable".into(), J::String(executable)),
                ("cwd".into(), J::String(root.to_str().unwrap().into())),
                (
                    "arguments".into(),
                    J::Array(vec![J::String(
                        root.join("backend.py").to_str().unwrap().into(),
                    )]),
                ),
                ("milliseconds".into(), J::Int(1000)),
            ]),
        ));
        std::fs::write(
            root.join("policy.json"),
            serialize_json(&policy, JsonLimits::DEFAULT).unwrap(),
        )
        .unwrap();
        Self { root, port }
    }
    fn cli(&self, action: &str) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_svr"));
        command.arg(action).arg("--experimental-generics");
        if action != "lock" {
            command.arg("--experimental-data");
        }
        command
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
struct Process(Option<Child>);
impl Drop for Process {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
fn client_and_wait(mut server: Command, port: u16, key: &std::path::Path) {
    let mut process = Process(Some(
        server
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    ));
    let script = r#"
import http.client,json,sys,time,pathlib
port=int(sys.argv[1]);secret=pathlib.Path(sys.argv[2]).read_text();deadline=time.monotonic()+8
cases=[('GET','/identity',secret,200),('POST','/echo','wrong-fixture-token',401),('POST','/echo',secret,403),('GET','/identity',secret,200),('GET','/identity',secret,429)]
slow=http.client.HTTPConnection('127.0.0.1',port,timeout=3)
while True:
 try:
  slow.request('GET','/identity',headers={'Authorization':'Bearer '+secret,'Origin':'https://client.example'})
  break
 except ConnectionRefusedError:
  slow.close();slow=http.client.HTTPConnection('127.0.0.1',port,timeout=3)
  if time.monotonic()>deadline:raise
  time.sleep(.02)
for index,(method,target,token,status) in enumerate(cases):
 while True:
  c=http.client.HTTPConnection('127.0.0.1',port,timeout=3)
  try:
   if status==200:c.request(method,target,headers={'Authorization':'Bearer '+token})
   else:
    c.putrequest(method,target);c.putheader('Authorization','Bearer '+token)
    c.putheader('Content-Length','100');c.putheader('Expect','100-continue');c.endheaders()
   break
  except ConnectionRefusedError:
   c.close()
   if index or time.monotonic()>deadline:raise
   time.sleep(.02)
 r=c.getresponse();assert r.status==status,(target,r.status)
 assert r.getheader('Connection')=='close'
 body=r.read();assert int(r.getheader('Content-Length'))==len(body)
 if status==200:
  value=json.loads(body);assert value['principal']=='fixture-client'
  assert not any(h['name'].lower()=='authorization' for h in value['headers'])
  assert secret.encode('ascii') not in body
 else:
  assert body==b''
  if status==401:assert r.getheader('WWW-Authenticate')=='Bearer realm="Sovra"'
  if status==429:assert r.getheader('Cache-Control')=='no-store'
 c.close()
 if index==0:
  pathlib.Path(sys.argv[2]).parent.joinpath('release').write_text('go')
  pending=slow.getresponse();assert pending.status==200
  delayed=json.loads(pending.read());assert delayed['principal']=='fixture-client'
  assert secret not in json.dumps(delayed)
  slow.close()
print('Independent authenticated HTTP client: valid identity/redaction, wrong credential 401, request permission 403, continued service and pre-verifier 429')
"#;
    let client = Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(port.to_string())
        .arg(key)
        .output()
        .unwrap();
    if !client.status.success() {
        let _ = process.0.as_mut().unwrap().kill();
        let server = process.0.take().unwrap().wait_with_output().unwrap();
        panic!(
            "client: {}\nserver: {}",
            String::from_utf8_lossy(&client.stderr),
            String::from_utf8_lossy(&server.stderr)
        );
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    while process.0.as_mut().unwrap().try_wait().unwrap().is_none() {
        assert!(
            Instant::now() < deadline,
            "Sovra server did not exit after bounded request count"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = process.0.take().unwrap().wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(key.parent().unwrap().join("invocations")).unwrap(),
        "xxxxx",
        "quota-denied requests must not spawn the verifier"
    );
    let rebound = TcpListener::bind((Ipv4Addr::LOCALHOST, port)).unwrap();
    drop(rebound);
}
#[test]
fn cli_delegates_authentication_and_recovers_after_credential_and_permission_refusal() {
    let f = Fixture::new();
    assert!(f
        .cli("lock")
        .arg(f.root.join("app"))
        .output()
        .unwrap()
        .status
        .success());
    let checked = f
        .cli("check")
        .arg("--locked")
        .arg(f.root.join("app"))
        .output()
        .unwrap();
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    let mut server = f.cli("run");
    server
        .arg("--locked")
        .arg("--host-policy")
        .arg(f.root.join("policy.json"))
        .arg(f.root.join("app"));
    client_and_wait(server, f.port, &f.root.join("key"));
}
#[test]
fn native_delegates_authentication_without_project_node_or_embedded_credentials() {
    let f = Fixture::new();
    assert!(f
        .cli("lock")
        .arg(f.root.join("app"))
        .output()
        .unwrap()
        .status
        .success());
    let exe = f.root.join("http-server");
    let built = f
        .cli("build")
        .arg("--locked")
        .arg("--emit")
        .arg("exe")
        .arg("--output")
        .arg(&exe)
        .arg(f.root.join("app"))
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    assert!(!std::fs::read(&exe)
        .unwrap()
        .windows(FIXTURE_SECRET.len())
        .any(|part| part == FIXTURE_SECRET.as_bytes()));
    std::fs::remove_dir_all(f.root.join("app")).unwrap();
    let mut server = Command::new(exe);
    server
        .arg("--host-policy")
        .arg(f.root.join("policy.json"))
        .env_clear()
        .env("PATH", "/nonexistent")
        .current_dir(&f.root);
    client_and_wait(server, f.port, &f.root.join("key"));
}
