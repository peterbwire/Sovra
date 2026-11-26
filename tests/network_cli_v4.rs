//! Explicit external CLI/native socket grants and independent binary client acceptance.
#![cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    path::PathBuf,
    process::{Child, Command, Output, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};
const SOURCE: &str = r#"
fn call(text:String)->std::Result<std::JsonValue,std::Error>{let request=std::json_parse(text)?;return std::host_call(request);}
fn serve()->std::Result<String,std::Error>{
 let listener=call("{\"version\":4,\"op\":\"net.listen\",\"port\":PORT}")?;
 let handle=std::json_stringify(listener)?;
 let accepted=call("{\"version\":4,\"op\":\"net.accept\",\"handle\":"+handle+"}")?;
 let handle=std::json_stringify(accepted)?;
 let received=call("{\"version\":4,\"op\":\"net.read\",\"handle\":"+handle+",\"maximum\":1}")?;
 let encoded=std::json_stringify(received)?;
 let written=call("{\"version\":4,\"op\":\"net.write\",\"handle\":"+handle+",\"data\":"+encoded+"}")?;
 return std::json_stringify(written);
}
fn main(){let result=serve();match result{std::Result::Ok(text)=>{if text!="1"{let fail=1/0;}},std::Result::Err(error)=>{let fail=1/0;}}}
"#;
struct Fixture {
    root: PathBuf,
    port: u16,
}
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "svr network cli {} {}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(root.join("app")).unwrap();
        let probe = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        std::fs::write(
            root.join("app/sovra.toml"),
            "[project]\nname=\"echo\"\nentry=\"main.svr\"\n",
        )
        .unwrap();
        std::fs::write(
            root.join("app/main.svr"),
            SOURCE.replace("PORT", &port.to_string()),
        )
        .unwrap();
        std::fs::write(root.join("policy.json"),format!(r#"{{"version":4,"listen_ports":[{port}],"limits":{{"listeners":1,"streams":1,"bytes":1,"milliseconds":2000}}}}"#)).unwrap();
        Self { root, port }
    }
    fn cli(&self, verb: &str) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_svr"));
        c.arg(verb).arg("--experimental-generics");
        if verb != "lock" {
            c.arg("--experimental-data");
        }
        c
    }
    fn run(&self) -> Command {
        let mut c = self.cli("run");
        c.arg("--locked")
            .arg("--host-policy")
            .arg(self.root.join("policy.json"))
            .arg(self.root.join("app"));
        c
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
fn echo(mut command: Command, port: u16) -> Output {
    let mut process = Process(Some(
        command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    ));
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let end = Instant::now() + Duration::from_secs(5);
    let mut client = loop {
        if let Ok(client) = TcpStream::connect_timeout(&address, Duration::from_millis(40)) {
            break client;
        }
        if let Some(status) = process.0.as_mut().unwrap().try_wait().unwrap() {
            let output = process.0.take().unwrap().wait_with_output().unwrap();
            panic!(
                "server exited {status}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        assert!(Instant::now() < end, "server did not listen");
        std::thread::sleep(Duration::from_millis(5));
    };
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
    drop(client);
    let end = Instant::now() + Duration::from_secs(3);
    while process.0.as_mut().unwrap().try_wait().unwrap().is_none() {
        assert!(Instant::now() < end, "server did not terminate");
        std::thread::sleep(Duration::from_millis(5));
    }
    process.0.take().unwrap().wait_with_output().unwrap()
}
#[test]
fn cli_grants_are_external_exact_and_unavailable_to_check_test_or_build() {
    let f = Fixture::new();
    assert!(f
        .cli("lock")
        .arg(f.root.join("app"))
        .output()
        .unwrap()
        .status
        .success());
    let original_lock = std::fs::read(f.root.join("app/sovra.lock")).unwrap();
    assert!(f
        .cli("check")
        .arg("--locked")
        .arg(f.root.join("app"))
        .output()
        .unwrap()
        .status
        .success());
    assert!(!f
        .cli("run")
        .arg("--locked")
        .arg(f.root.join("app"))
        .output()
        .unwrap()
        .status
        .success());
    for action in ["check", "test", "build"] {
        let output = f
            .cli(action)
            .arg("--host-policy")
            .arg(f.root.join("nonexistent-policy"))
            .arg(f.root.join("app"))
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(2),
            "{action}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let occupied = TcpListener::bind((Ipv4Addr::LOCALHOST, f.port)).unwrap();
    assert!(!f.run().output().unwrap().status.success());
    drop(occupied);
    let output = echo(f.run(), f.port);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // A v4 grant never changes the package lock or creates files in source directories.
    assert_eq!(
        std::fs::read(f.root.join("app/sovra.lock")).unwrap(),
        original_lock
    );
    assert_eq!(std::fs::read_dir(f.root.join("app")).unwrap().count(), 3);
}
#[test]
fn generated_native_server_uses_external_grants_after_project_and_node_are_unavailable() {
    let f = Fixture::new();
    assert!(f
        .cli("lock")
        .arg(f.root.join("app"))
        .output()
        .unwrap()
        .status
        .success());
    let exe = f.root.join("echo-server");
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
    let denied = Command::new(&exe)
        .env_clear()
        .env("PATH", "/nonexistent")
        .current_dir(&f.root)
        .output()
        .unwrap();
    assert!(!denied.status.success());
    let rejected = Command::new(&exe)
        .arg("--host-policy")
        .arg(f.root.join("missing.json"))
        .env_clear()
        .env("PATH", "/nonexistent")
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    let mut native = Command::new(&exe);
    native
        .arg("--host-policy")
        .arg(f.root.join("policy.json"))
        .env_clear()
        .env("PATH", "/nonexistent")
        .current_dir(&f.root);
    let output = echo(native, f.port);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let rebound = TcpListener::bind((Ipv4Addr::LOCALHOST, f.port)).unwrap();
    drop(rebound);
}
