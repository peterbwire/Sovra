//! Explicit host authority and owned child/resource regression checks.
#![cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
use sovra::compiler::v2::*;
use std::{collections::BTreeMap, path::PathBuf};
fn request(text: &str) -> JsonValue {
    parse_json(text.as_bytes(), JsonLimits::DEFAULT).unwrap()
}
fn run(host: &HostV3, text: &str) -> Result<JsonValue, HostError> {
    host.call(&request(text), || Ok::<_, ()>(())).unwrap()
}
struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "svr host v3 {} {}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn deny_default_roots_symlink_parent_escape_and_finite_file_directory_operations() {
    let root = Root::new();
    let mut host = HostV3::default();
    let read = r#"{"version":3,"op":"fs.read","root":"data","path":"file"}"#;
    assert_eq!(run(&host, read).unwrap_err().code, "authority");
    host.grant_read("data", &root.0).unwrap();
    host.grant_write("data", &root.0).unwrap();
    assert_eq!(
        run(
            &host,
            r#"{"version":3,"op":"fs.write","root":"data","path":"file","data":"YWJj"}"#
        )
        .unwrap(),
        JsonValue::Int(3)
    );
    assert_eq!(run(&host, read).unwrap(), JsonValue::String("YWJj".into()));
    assert!(run(
        &host,
        r#"{"version":3,"op":"fs.list","root":"data","path":""}"#
    )
    .is_ok());
    assert!(run(
        &host,
        r#"{"version":3,"op":"fs.read","root":"data","path":"../file"}"#
    )
    .is_err());
    std::os::unix::fs::symlink("/etc", root.0.join("link")).unwrap();
    assert!(run(
        &host,
        r#"{"version":3,"op":"fs.read","root":"data","path":"link/passwd"}"#
    )
    .is_err());
    host.limits.bytes = 2;
    assert_eq!(run(&host, read).unwrap_err().code, "limit");
    host.limits.bytes = 3;
    assert!(run(
        &host,
        r#"{"version":3,"op":"fs.rename","root":"data","path":"file","to":"renamed"}"#
    )
    .is_ok());
    assert!(!root.0.join("file").exists());
    assert!(run(
        &host,
        r#"{"version":3,"op":"fs.remove","root":"data","path":"renamed"}"#
    )
    .is_ok());
    assert!(run(
        &host,
        r#"{"version":3,"op":"fs.mkdir","root":"data","path":"folder"}"#
    )
    .is_ok());
    host.limits.entries = 0;
    assert_eq!(
        run(
            &host,
            r#"{"version":3,"op":"fs.list","root":"data","path":""}"#
        )
        .unwrap_err()
        .code,
        "limit"
    );
}
#[test]
fn final_symlinks_cannot_read_or_truncate_files_outside_granted_roots() {
    let root = Root::new();
    let outside = Root::new();
    let target = outside.0.join("secret");
    std::fs::write(&target, b"unchanged").unwrap();
    std::os::unix::fs::symlink(&target, root.0.join("link")).unwrap();
    let mut host = HostV3::default();
    host.grant_read("data", &root.0).unwrap();
    host.grant_write("data", &root.0).unwrap();
    assert!(run(
        &host,
        r#"{"version":3,"op":"fs.read","root":"data","path":"link"}"#
    )
    .is_err());
    assert!(run(
        &host,
        r#"{"version":3,"op":"fs.write","root":"data","path":"link","data":"eA=="}"#
    )
    .is_err());
    assert_eq!(std::fs::read(target).unwrap(), b"unchanged");
}

#[test]
fn owned_process_captures_binary_streams_clears_environment_and_cleans_on_control() {
    let mut host = HostV3::default();
    let cwd = std::env::temp_dir();
    host.grant_program(
        "cat",
        std::path::Path::new("/bin/cat"),
        &cwd,
        BTreeMap::new(),
    )
    .unwrap();
    let input = r#"{"version":3,"op":"process.run","program":"cat","arguments":[],"input":"AP8="}"#;
    let result = run(&host, input).unwrap();
    let JsonValue::Object(fields) = result else {
        panic!()
    };
    assert!(fields.contains(&("exit_code".into(), JsonValue::Int(0))));
    assert!(fields.contains(&("stdout".into(), JsonValue::String("AP8=".into()))));
    host.grant_program(
        "env",
        std::path::Path::new("/usr/bin/env"),
        &cwd,
        BTreeMap::new(),
    )
    .unwrap();
    let result = run(
        &host,
        r#"{"version":3,"op":"process.run","program":"env","arguments":[],"input":""}"#,
    )
    .unwrap();
    let JsonValue::Object(fields) = result else {
        panic!()
    };
    assert!(fields.contains(&("stdout".into(), JsonValue::String("".into()))));
    host.grant_program(
        "sleep",
        std::path::Path::new("/bin/sleep"),
        &cwd,
        BTreeMap::new(),
    )
    .unwrap();
    let request = request(
        r#"{"version":3,"op":"process.run","program":"sleep","arguments":["10"],"input":""}"#,
    );
    let mut checks = 0;
    let start = std::time::Instant::now();
    let result = host.call(&request, || {
        checks += 1;
        if checks >= 3 {
            Err("cancelled")
        } else {
            Ok(())
        }
    });
    assert!(result.is_err());
    assert!(start.elapsed() < std::time::Duration::from_secs(2));
    host.limits.milliseconds = 10;
    let timed = host
        .call(&request, || Ok::<_, ()>(()))
        .unwrap()
        .unwrap_err();
    assert_eq!(timed.code, "timeout");
    host.limits.milliseconds = 30000;
    host.limits.bytes = 8;
    host.grant_program(
        "yes",
        std::path::Path::new("/usr/bin/yes"),
        &cwd,
        BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(
        run(
            &host,
            r#"{"version":3,"op":"process.run","program":"yes","arguments":[],"input":""}"#
        )
        .unwrap_err()
        .code,
        "limit"
    );

    assert_eq!(run(&host,r#"{"version":3,"op":"process.run","program":"cat","arguments":["/nonexistent-sovra-file"],"input":""}"#).unwrap_err().code,"limit");
    host.limits.arguments = 0;
    assert_eq!(
        run(
            &host,
            r#"{"version":3,"op":"process.run","program":"cat","arguments":["x"],"input":""}"#
        )
        .unwrap_err()
        .code,
        "limit"
    );
}
#[test]
fn clock_timer_authority_duration_and_cancellation_are_explicit() {
    let mut host = HostV3::default();
    assert_eq!(
        run(&host, r#"{"version":3,"op":"clock.wall"}"#)
            .unwrap_err()
            .code,
        "authority"
    );
    host.clocks = true;
    assert!(matches!(
        run(&host, r#"{"version":3,"op":"clock.monotonic"}"#),
        Ok(JsonValue::Int(_))
    ));
    host.timers = true;
    host.limits.milliseconds = 1;
    assert!(run(
        &host,
        r#"{"version":3,"op":"timer.sleep","milliseconds":2}"#
    )
    .is_err());
    assert!(run(
        &host,
        r#"{"version":3,"op":"timer.sleep","milliseconds":0}"#
    )
    .is_ok());
}

#[test]
fn gregorian_utc_fixed_offsets_and_arithmetic_cover_full_range() {
    assert_eq!(parse_date("1970-01-01T00:00:00Z").unwrap(), 0);
    assert_eq!(parse_date("1969-12-31T23:59:59.999Z").unwrap(), -1);
    for year in (1..=9999).step_by(17) {
        for month in 1..=12 {
            let input = format!("{year:04}-{month:02}-15T12:34:56.789Z");
            let ms = parse_date(&input).unwrap();
            assert_eq!(format_date(ms, 0).unwrap(), input);
        }
    }
    for (input, zone) in [
        ("2000-02-29T23:59:59.123-03:30", -210),
        ("2024-02-29T00:00:00.000+02:00", 120),
        ("0001-01-01T00:00:00.000Z", 0),
        ("9999-12-31T23:59:59.999Z", 0),
    ] {
        let ms = parse_date(input).unwrap();
        assert_eq!(format_date(ms, zone).unwrap(), input);
    }
    for invalid in [
        "1900-02-29T00:00:00Z",
        "2023-02-29T00:00:00Z",
        "0000-01-01T00:00:00Z",
        "2024-01-01T00:00:60Z",
        "2024-01-01T00:00:00",
        "2024-01-01T00:00:00+24:00",
        "2024-01-01T00:00:00.1234Z",
    ] {
        assert!(parse_date(invalid).is_err(), "{invalid}");
    }
    assert!(add_date(parse_date("9999-12-31T23:59:59.999Z").unwrap(), 1).is_err());
    assert!(add_date(0, i64::MAX).is_err());
    assert_eq!(
        format_date(add_date(0, 86400000).unwrap(), 0).unwrap(),
        "1970-01-02T00:00:00.000Z"
    );
}
#[test]
fn policy_unknown_fields_ceilings_and_source_override_attempts_refuse() {
    assert!(HostV3::from_policy(br#"{"version":3,"clocks":"yes"}"#).is_err());
    assert!(HostV3::from_policy(br#"{"version":3,"limits":{"bytes":16777217}}"#).is_err());
    let host = HostV3::default();
    assert_eq!(
        run(&host, r#"{"version":3,"op":"clock.wall","grant":true}"#)
            .unwrap_err()
            .code,
        "input"
    );
}

#[test]
fn rooted_parent_swap_races_never_read_or_write_the_outside_target() {
    let root = Root::new();
    let allowed = root.0.join("allowed");
    let outside = root.0.join("outside");
    std::fs::create_dir_all(allowed.join("child")).unwrap();
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(allowed.join("child/file"), b"inside").unwrap();
    std::fs::write(outside.join("file"), b"outside").unwrap();
    let mut host = HostV3::default();
    host.grant_read("root", &allowed).unwrap();
    host.grant_write("root", &allowed).unwrap();
    let racing = allowed.clone();
    let thread = std::thread::spawn(move || {
        for _ in 0..500 {
            let child = racing.join("child");
            let parked = racing.join("parked");
            if std::fs::rename(&child, &parked).is_ok() {
                let _ = std::os::unix::fs::symlink(&outside, &child);
                std::thread::yield_now();
                let _ = std::fs::remove_file(&child);
                let _ = std::fs::rename(&parked, &child);
            }
        }
    });
    for _ in 0..500 {
        if let Ok(value) = run(
            &host,
            r#"{"version":3,"op":"fs.read","root":"root","path":"child/file"}"#,
        ) {
            assert_eq!(value, JsonValue::String("aW5zaWRl".into()));
        }
        if let Ok(value) = run(
            &host,
            r#"{"version":3,"op":"fs.write","root":"root","path":"child/file","data":"aW5zaWRl"}"#,
        ) {
            assert_eq!(value, JsonValue::Int(6));
        }
    }
    thread.join().unwrap();
    assert_eq!(
        std::fs::read(root.0.join("outside/file")).unwrap(),
        b"outside"
    );
    std::fs::write(allowed.join("file"), b"prior").unwrap();
    let req = request(r#"{"version":3,"op":"fs.write","root":"root","path":"file","data":"YWJj"}"#);
    let mut ticks = 0;
    assert!(host
        .call(&req, || {
            ticks += 1;
            if ticks >= 2 {
                Err("cancel")
            } else {
                Ok(())
            }
        })
        .is_err());
    assert_eq!(std::fs::read(allowed.join("file")).unwrap(), b"");
}
#[test]
fn portable_relative_paths_are_pure_and_cannot_introduce_parent_or_absolute_roots() {
    let host = HostV3::default();
    assert_eq!(
        run(
            &host,
            r#"{"version":3,"op":"path.normalize","path":"a/./λ//b"}"#
        )
        .unwrap(),
        JsonValue::String("a/λ/b".into())
    );
    assert_eq!(
        run(
            &host,
            r#"{"version":3,"op":"path.join","left":"a/λ","right":"b"}"#
        )
        .unwrap(),
        JsonValue::String("a/λ/b".into())
    );
    for path in ["../secret", "a/../secret", "/secret"] {
        let v = JsonValue::Object(vec![
            ("version".into(), JsonValue::Int(3)),
            ("op".into(), JsonValue::String("path.normalize".into())),
            ("path".into(), JsonValue::String(path.into())),
        ]);
        assert!(host.call(&v, || Ok::<_, ()>(())).unwrap().is_err());
    }
}

#[test]
fn owned_child_fixture() {
    if let Some(path) = std::env::var_os("SOVRA_OWNED_CHILD_PID") {
        std::fs::write(path, std::process::id().to_string()).unwrap();
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}
#[test]
fn control_refusal_kills_and_reaps_the_observed_owned_child() {
    let root = Root::new();
    let pidfile = root.0.join("child.pid");
    let mut env = BTreeMap::new();
    env.insert(
        "SOVRA_OWNED_CHILD_PID".into(),
        pidfile.to_str().unwrap().into(),
    );
    let mut host = HostV3::default();
    host.grant_program("fixture", &std::env::current_exe().unwrap(), &root.0, env)
        .unwrap();
    let req = request(
        r#"{"version":3,"op":"process.run","program":"fixture","arguments":["--exact","owned_child_fixture","--nocapture"],"input":""}"#,
    );
    let start = std::time::Instant::now();
    let out = host.call(&req, || {
        if std::fs::read_to_string(&pidfile)
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .is_some()
        {
            Err("cancel")
        } else {
            Ok(())
        }
    });
    assert!(out.is_err());
    assert!(start.elapsed() < std::time::Duration::from_secs(5));
    let pid = std::fs::read_to_string(pidfile).unwrap();
    assert!(
        !std::path::Path::new("/proc").join(pid).exists(),
        "owned child must be dead and reaped"
    );
}
