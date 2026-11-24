//! Independent Python HTTP client against actual Sovra CLI and native handlers.
#![cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
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
                .replace("47123", &port.to_string()),
        )
        .unwrap();
        std::fs::write(
            root.join("policy.json"),
            include_str!("../examples/http-loopback/policy.json")
                .replace("47123", &port.to_string()),
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
fn client_and_wait(mut server: Command, port: u16) {
    let mut process = Process(Some(
        server
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    ));
    let script = r#"
import http.client,json,sys,time
port=int(sys.argv[1]);deadline=time.monotonic()+8
cases=[('GET','/health',None,200,{'ok':True}),('HEAD','/health',None,200,None),('POST','/echo','{"n":7,"message":"λ😀"}'.encode('utf-8'),200,{'n':7,'message':'λ😀'}),('GET','/missing',None,404,{'error':'not_found'}),('GET','/failure',None,500,{'error':'handler'})]
for index,(method,target,body,status,expected) in enumerate(cases):
 while True:
  connection=http.client.HTTPConnection('127.0.0.1',port,timeout=3)
  try:
   connection.request(method,target,body=body)
   break
  except ConnectionRefusedError:
   connection.close()
   if index or time.monotonic()>deadline:raise
   time.sleep(.02)
 response=connection.getresponse();assert response.status==status,(target,response.status)
 assert response.getheader('Content-Type')=='application/json'
 assert response.getheader('Connection')=='close'
 data=response.read()
 if method=='HEAD':assert data==b'' and int(response.getheader('Content-Length'))==len(b'{"ok":true}')
 else:assert int(response.getheader('Content-Length'))==len(data) and json.loads(data)==expected,(target,data)
 connection.close()
print('Independent HTTP client: GET/HEAD, JSON echo, 404, handler 500 and close framing passed')
"#;
    let client = Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(port.to_string())
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
    let rebound = TcpListener::bind((Ipv4Addr::LOCALHOST, port)).unwrap();
    drop(rebound);
}
#[test]
fn sovra_cli_dispatches_http_json_routes_and_handler_failures() {
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
    client_and_wait(server, f.port);
}
#[test]
fn native_http_handlers_run_without_project_or_node() {
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
    std::fs::remove_dir_all(f.root.join("app")).unwrap();
    let mut server = Command::new(exe);
    server
        .arg("--host-policy")
        .arg(f.root.join("policy.json"))
        .env_clear()
        .env("PATH", "/nonexistent")
        .current_dir(&f.root);
    client_and_wait(server, f.port);
}
