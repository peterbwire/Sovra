//! CLI registered-module dispatch, execution, provenance and explicit tooling refusal.
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
mod linux {
    use std::{
        path::{Path, PathBuf},
        process::{Command, Output},
        sync::atomic::{AtomicUsize, Ordering},
    };
    fn svr() -> Command {
        Command::new(env!("CARGO_BIN_EXE_svr"))
    }
    fn run(args: &[&str], root: &Path) -> Output {
        svr().args(args).arg(root).output().unwrap()
    }
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let root = std::env::temp_dir().join(format!(
                "sovra-registered-cli-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&root).unwrap();
            Self(root)
        }
        fn package(&self) -> PathBuf {
            let root = self.0.join("app");
            std::fs::create_dir(&root).unwrap();
            std::fs::write(root.join("sovra.toml"),"[project]\nname=\"registered\"\nentry=\"main.svr\"\n[modules]\nmath=\"math.svr\"\n").unwrap();
            std::fs::write(root.join("math.svr"),"mod math{export struct Point{x:Float} export fn read(p:Point)->Float{return p.x;}}").unwrap();
            std::fs::write(
                root.join("main.svr"),
                "fn main(){print(math::read(math::Point{x:3})/2);}",
            )
            .unwrap();
            root
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn package_formatter_selects_every_registered_source_and_preserves_lock() {
        let fixture = Fixture::new();
        let root = fixture.package();
        assert!(run(&["lock"], &root).status.success());
        let lock = std::fs::read(root.join("sovra.lock")).unwrap();
        let before = std::fs::read(root.join("math.svr")).unwrap();
        let check = run(&["fmt", "--package", "--check"], &root);
        assert_eq!(check.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&check.stderr).contains("math.svr"));
        assert_eq!(std::fs::read(root.join("math.svr")).unwrap(), before);
        let write = run(&["fmt", "--package", "--write"], &root);
        assert!(
            write.status.success(),
            "{}",
            String::from_utf8_lossy(&write.stderr)
        );
        assert!(run(&["fmt", "--package", "--check"], &root)
            .status
            .success());
        assert_ne!(std::fs::read(root.join("math.svr")).unwrap(), before);
        assert_eq!(std::fs::read(root.join("sovra.lock")).unwrap(), lock);
        assert!(!run(&["run", "--locked"], &root).status.success());
        assert!(run(&["run"], &root).status.success());
    }
    #[test]
    fn locks_gate_execution_emission_and_tests_without_rewriting_or_fallback() {
        let fixture = Fixture::new();
        let root = fixture.package();
        let entry = root.join("main.svr");
        let source = std::fs::read_to_string(&entry).unwrap();
        std::fs::write(
            entry,
            format!("{source} fn test_registered() {{print(math::read(math::Point{{x:3}}));}}"),
        )
        .unwrap();
        assert!(!run(&["run", "--locked"], &root).status.success());
        assert!(run(&["lock"], &root).status.success());
        let original = std::fs::read(root.join("sovra.lock")).unwrap();
        for args in [
            &["lock", "--check"][..],
            &["run", "--locked"][..],
            &["build", "--locked", "--emit", "js"][..],
            &["test", "--locked", "--format", "json"][..],
            &[
                "check",
                "--locked",
                "--executable-package",
                "--format",
                "json",
            ][..],
        ] {
            let output = run(args, &root);
            assert!(
                output.status.success(),
                "{args:?}: {} {}",
                String::from_utf8_lossy(&output.stderr),
                String::from_utf8_lossy(&output.stdout)
            );
        }
        assert_eq!(std::fs::read(root.join("sovra.lock")).unwrap(), original);
        let module = root.join("math.svr");
        let bytes = std::fs::read_to_string(&module).unwrap();
        std::fs::write(&module, format!("// comment changed\n{bytes}")).unwrap();
        for args in [
            &["lock", "--check"][..],
            &["run", "--locked"][..],
            &["build", "--locked", "--emit", "js"][..],
            &["test", "--locked"][..],
        ] {
            let output = run(args, &root);
            assert!(!output.status.success());
            assert!(String::from_utf8_lossy(&output.stderr).contains("E4165"));
            assert!(output.stdout.is_empty());
        }
        let artifact = root.join("must-not-exist");
        let output = svr()
            .args(["build", "--locked", "--emit", "exe", "--output"])
            .arg(&artifact)
            .arg(&root)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(!artifact.exists());
        assert_eq!(std::fs::read(root.join("sovra.lock")).unwrap(), original);
        assert!(run(&["lock"], &root).status.success());
        assert!(run(&["run", "--locked"], &root).status.success());
        std::fs::write(
            module,
            "mod math {export fn broken()->Int {return missing;}}",
        )
        .unwrap();
        assert!(run(&["lock"], &root).status.success());
        assert!(run(&["lock", "--check"], &root).status.success());
        assert!(!run(&["run", "--locked"], &root).status.success());
    }
    #[test]
    fn locked_flags_and_unsafe_publication_targets_are_rejected() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let fixture = Fixture::new();
        let root = fixture.package();
        for args in [
            &["lock", "--check", "--check"][..],
            &["run", "--locked", "--locked"][..],
            &["test", "--locked", "--locked"][..],
            &["check", "--locked"][..],
            &[
                "check",
                "--locked",
                "--executable-package",
                "--package-context",
            ][..],
        ] {
            assert_eq!(run(args, &root).status.code(), Some(2));
        }
        assert_eq!(
            run(&["run", "--locked"], &root.join("main.svr"))
                .status
                .code(),
            Some(2)
        );
        let path = root.join("sovra.lock");
        let outside = fixture.0.join("outside");
        std::fs::write(&outside, "preserve").unwrap();
        symlink(&outside, &path).unwrap();
        assert!(!run(&["lock"], &root).status.success());
        assert_eq!(std::fs::read_to_string(&outside).unwrap(), "preserve");
        std::fs::remove_file(&path).unwrap();
        std::fs::hard_link(&outside, &path).unwrap();
        assert!(!run(&["lock"], &root).status.success());
        std::fs::remove_file(&path).unwrap();
        assert!(run(&["lock"], &root).status.success());
        let lock = std::fs::read(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o400)).unwrap();
        assert!(!run(&["lock"], &root).status.success());
        assert_eq!(std::fs::read(&path).unwrap(), lock);
    }
    #[test]
    fn registered_examples_check_run_build_ir_and_execute_javascript() {
        for (example, expected) in [
            ("examples/multi-file-packages/app", "49"),
            ("examples/multi-file-records/app", "25\n1.5"),
        ] {
            let root = Path::new(example);
            for args in [
                &["check"][..],
                &["check", "--executable-package"][..],
                &["check", "--format", "json"][..],
            ] {
                let output = run(args, root);
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                if args.contains(&"json") {
                    assert!(String::from_utf8_lossy(&output.stdout).contains("\"success\":true"));
                }
            }
            let output = run(&["run"], root);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), expected);
            let output = run(&["build"], root);
            assert!(output.status.success());
            assert!(!output.stdout.is_empty());
            let output = run(&["build", "--emit", "js"], root);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let js = std::process::Command::new("node")
                .arg("--eval")
                .arg(String::from_utf8(output.stdout).unwrap())
                .output()
                .unwrap();
            assert!(
                js.status.success(),
                "{}",
                String::from_utf8_lossy(&js.stderr)
            );
            assert_eq!(String::from_utf8(js.stdout).unwrap().trim(), expected);
        }
    }
    #[test]
    fn local_registration_without_dependencies_selects_full_compilation_and_root_tests() {
        let fixture = Fixture::new();
        let root = fixture.package();
        let output = run(&["run"], &root);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "1.5");
        std::fs::write(root.join("math.svr"),"mod math{export struct Point{x:Float} export fn read(p:Point)->Float{return p.x;} fn test_other(){print(777);}}").unwrap();

        std::fs::write(
            root.join("main.svr"),
            "fn main(){print(999);} fn test_record(){print(math::read(math::Point{x:3})/2);}",
        )
        .unwrap();
        let output = run(&["test", "--format", "json"], &root);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        let report = String::from_utf8(output.stdout).unwrap();
        assert!(report.contains("test_record"));
        assert!(report.contains("1.5"));
        assert!(!report.contains("999"));
        assert!(!report.contains("777"));
        assert!(!report.contains("test_other"));
    }
    #[test]
    fn source_errors_keep_module_locations_and_never_emit_partial_programs() {
        let fixture = Fixture::new();
        let root = fixture.package();
        std::fs::write(
            root.join("math.svr"),
            "// λ\r\nmod math{fn invalid()->Int{return true;}}",
        )
        .unwrap();
        std::fs::write(root.join("main.svr"), "fn main(){}").unwrap();
        let output = run(&["check", "--format", "json"], &root);
        assert_eq!(output.status.code(), Some(1));
        let report = String::from_utf8(output.stdout).unwrap();
        assert!(report.contains("math.svr"));
        assert!(report.contains("\"line\":1"));
        assert!(!report.contains("\"success\":true"));
        for args in [&["run"][..], &["build"][..], &["build", "--emit", "js"][..]] {
            let output = run(args, &root);
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
        }
        let output = fixture.0.join("bad-program");
        let result = svr()
            .args(["build", "--emit", "exe", "--output"])
            .arg(&output)
            .arg(&root)
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(1));
        assert!(!output.exists());
    }
    #[test]
    fn unsupported_graph_tools_refuse_modules_instead_of_claiming_entry_only_coverage() {
        let fixture = Fixture::new();
        let root = fixture.package();
        {
            let args = &["check", "--format", "json", "--package-context"][..];
            let output = run(args, &root);
            assert!(!output.status.success());
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(combined.contains("modules"));
        }
        let output = run(&["check", "--service-calls"], &root);
        assert_eq!(output.status.code(), Some(2));
        // Overlay graph admission remains strict, even before overlay selection.
        let mut request = sovra::compiler::project::packages::SOURCE_OVERLAY_MAGIC.to_vec();
        request.extend_from_slice(&1u32.to_be_bytes());
        request.extend_from_slice(&0u32.to_be_bytes());
        let mut command = svr();
        command
            .args(["check", "--format", "json", "--overlays-stdin"])
            .arg(root);
        use std::io::Write;
        let mut child = command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(&request).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("modules"));
    }
    #[test]
    fn application_scan_cannot_mask_registered_source_errors() {
        let fixture = Fixture::new();
        let root = fixture.package();
        std::fs::write(root.join("main.svr"), "app Showcase{}\nfn main(){}").unwrap();
        let output = run(&["check", "--format", "json"], &root);
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stdout).contains("E2000"));
    }
    #[test]
    fn empty_registration_sections_and_unused_dependency_modules_do_not_fall_back() {
        let fixture = Fixture::new();
        let root = fixture.package();
        std::fs::write(
            root.join("sovra.toml"),
            "[project]\nname=\"app\"\nentry=\"main.svr\"\n[modules]\n",
        )
        .unwrap();
        std::fs::write(root.join("main.svr"), "fn main(){let value:Int=true;}").unwrap();
        let output = run(&["check", "--format", "json"], &root);
        assert_eq!(output.status.code(), Some(1));
        let report = String::from_utf8_lossy(&output.stdout);
        assert!(report.contains("main.svr") && report.contains("Bool"));
        let library = fixture.0.join("library");
        std::fs::create_dir(&library).unwrap();
        std::fs::write(
            library.join("sovra.toml"),
            "[project]\nname=\"library\"\nentry=\"main.svr\"\n[modules]\nbad=\"bad.svr\"\n",
        )
        .unwrap();
        std::fs::write(library.join("main.svr"), "").unwrap();
        std::fs::write(
            library.join("bad.svr"),
            "mod bad{fn unused()->Int{return true;}}",
        )
        .unwrap();
        std::fs::write(root.join("sovra.toml"),"[project]\nname=\"app\"\nentry=\"main.svr\"\n[dependencies.library]\npath=\"../library\"\n").unwrap();
        std::fs::write(root.join("main.svr"), "fn main(){print(123);}").unwrap();
        let output = run(&["check", "--format", "json"], &root);
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stdout).contains("bad.svr"));
        let output = run(&["run"], &root);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
    }
}
