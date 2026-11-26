//! External network policy validation and safe bounded policy-file loading.
#![cfg(target_os = "linux")]
use sovra::compiler::v2::*;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "svr network policy {} {}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn open(host: &mut NetworkHostV4, port: u16) -> Result<JsonValue, HostError> {
    let request = parse_json(
        format!(r#"{{"version":4,"op":"net.listen","port":{port}}}"#).as_bytes(),
        JsonLimits::DEFAULT,
    )
    .unwrap();
    host.call(&request, || Ok::<_, ()>(())).unwrap()
}
#[test]
fn policy_version_dispatch_is_exact_and_network_defaults_deny() {
    assert!(matches!(
        DataHostPolicy::from_policy(br#"{"version":3}"#).unwrap(),
        DataHostPolicy::V3(_)
    ));
    let mut host = match DataHostPolicy::from_policy(br#"{"version":4}"#).unwrap() {
        DataHostPolicy::Network(host) => host,
        _ => panic!("wrong policy dispatch"),
    };
    assert_eq!(host.usage(), (0, 0));
    assert_eq!(open(&mut host, 0).unwrap_err().code, "authority");
    let mut host=match DataHostPolicy::from_policy(br#"{"version":4,"listen_ports":[0],"limits":{"listeners":1,"streams":0,"bytes":0,"milliseconds":0}}"#).unwrap(){DataHostPolicy::Network(host)=>host,_=>panic!("wrong policy dispatch")};
    assert_eq!(host.usage(), (0, 0));
    assert!(open(&mut host, 0).is_ok());
    assert_eq!(open(&mut host, 0).unwrap_err().code, "limit");
    host.close_all().unwrap();
    assert_eq!(host.usage(), (0, 0));
}
#[test]
fn malformed_ambiguous_mixed_and_widened_policies_refuse() {
    for bytes in [
        r#"{}"#,
        r#"[]"#,
        r#"{"version":4.0}"#,
        r#"{"version":5}"#,
        r#"{"version":3,"listen_ports":[0]}"#,
        r#"{"version":4,"version":3}"#,
        r#"{"version":4,"clocks":true}"#,
        r#"{"version":4,"read_roots":{}}"#,
        r#"{"version":4,"listen_ports":[0,0]}"#,
        r#"{"version":4,"listen_ports":[-1]}"#,
        r#"{"version":4,"listen_ports":[65536]}"#,
        r#"{"version":4,"listen_ports":[true]}"#,
        r#"{"version":4,"limits":{"bytes":65537}}"#,
        r#"{"version":4,"limits":{"listeners":9}}"#,
        r#"{"version":4,"limits":{"streams":65}}"#,
        r#"{"version":4,"limits":{"milliseconds":30001}}"#,
        r#"{"version":4,"limits":{"bytes":-1}}"#,
        r#"{"version":4,"limits":{"bytes":true}}"#,
        r#"{"version":4,"limits":{"bytes":1,"bytes":0}}"#,
        r#"{"version":4,"limits":{"unknown":0}}"#,
    ] {
        assert!(
            DataHostPolicy::from_policy(bytes.as_bytes()).is_err(),
            "{bytes}"
        );
    }
    let ports = (0..17).map(|n| n.to_string()).collect::<Vec<_>>().join(",");
    assert!(DataHostPolicy::from_policy(
        format!(r#"{{"version":4,"listen_ports":[{ports}]}}"#).as_bytes()
    )
    .is_err());
    assert!(DataHostPolicy::from_policy(&vec![b' '; 65537]).is_err());
}
#[test]
fn policy_files_are_regular_bounded_nonblocking_and_no_follow() {
    use std::os::unix::fs::symlink;
    let root = Root::new();
    let path = root.0.join("policy.json");
    let valid = br#"{"version":4,"listen_ports":[0]}"#;
    std::fs::write(&path, valid).unwrap();
    assert!(DataHostPolicy::load_policy(&path).is_ok());
    let link = root.0.join("link");
    symlink(&path, &link).unwrap();
    assert!(DataHostPolicy::load_policy(&link).is_err());
    assert!(DataHostPolicy::load_policy(&root.0).is_err());
    let fifo = root.0.join("fifo");
    assert!(std::process::Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .unwrap()
        .success());
    assert!(DataHostPolicy::load_policy(&fifo).is_err());
    let mut exact = valid.to_vec();
    exact.resize(65536, b' ');
    std::fs::write(&path, &exact).unwrap();
    assert!(DataHostPolicy::load_policy(&path).is_ok());
    exact.push(b' ');
    std::fs::write(&path, &exact).unwrap();
    assert!(DataHostPolicy::load_policy(&path).is_err());
}
