//! Integration tests for the `svr` command-line contract.

use std::io;
use std::process::{Command, Output};

#[test]
fn package_commands_enforce_interface_string_default() {
    let root = std::env::temp_dir().join(format!(
        "svr-interface-strings-default-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(root.join("app")).unwrap();
    std::fs::create_dir_all(root.join("lib")).unwrap();
    std::fs::write(
        root.join("app/sovra.toml"),
        "[project]\nname = \"app\"\nentry = \"main.svr\"\n[dependencies.a]\npath = \"../lib\"\n",
    )
    .unwrap();
    std::fs::write(
        root.join("lib/sovra.toml"),
        "[project]\nname = \"lib\"\nentry = \"main.svr\"\n",
    )
    .unwrap();
    let name = format!("f{}", "x".repeat(1019));
    std::fs::write(
        root.join("lib/main.svr"),
        format!("mod api {{ export fn {name}() {{}} }}"),
    )
    .unwrap();
    std::fs::write(
        root.join("app/main.svr"),
        format!(
            "{} fn main() {{ print(42); }}",
            "use a::api;\n".repeat(65536)
        ),
    )
    .unwrap();
    for args in [
        vec!["check"],
        vec!["check", "--format", "json"],
        vec!["run"],
        vec!["build"],
        vec!["test"],
    ] {
        let Some(output) = output_or_skip(svr().args(&args).arg(root.join("app"))) else {
            std::fs::remove_dir_all(root).unwrap();
            return;
        };
        assert_eq!(output.status.code(), Some(1), "{args:?}: {output:?}");
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(combined.contains("E3024"), "{args:?}: {combined}");
        assert!(!String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| line == "42"));
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn package_commands_enforce_approved_interface_default() {
    let root = std::env::temp_dir().join(format!(
        "svr-package-interface-default-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(root.join("lib")).unwrap();
    std::fs::create_dir_all(root.join("app")).unwrap();
    std::fs::write(
        root.join("app/sovra.toml"),
        "[project]\nname = \"app\"\nentry = \"main.svr\"\n[dependencies.a]\npath = \"../lib\"\n",
    )
    .unwrap();
    std::fs::write(
        root.join("lib/sovra.toml"),
        "[project]\nname = \"lib\"\nentry = \"main.svr\"\n",
    )
    .unwrap();
    let parameters = (0..99)
        .map(|index| format!("p{index}: Int"))
        .collect::<Vec<_>>()
        .join(", ");
    std::fs::write(
        root.join("lib/main.svr"),
        format!("mod api {{ export fn f({parameters}) {{}} }}"),
    )
    .unwrap();
    // Export validation plus 2,500 copies of a 100-node signature exceeds 250,000.
    std::fs::write(
        root.join("app/main.svr"),
        format!(
            "{} fn main() {{ print(42); }}",
            "use a::api;\n".repeat(2500)
        ),
    )
    .unwrap();
    for args in [
        vec!["check"],
        vec!["check", "--format", "json"],
        vec!["run"],
        vec!["build"],
        vec!["test"],
    ] {
        let Some(output) = output_or_skip(svr().args(&args).arg(root.join("app"))) else {
            std::fs::remove_dir_all(root).unwrap();
            return;
        };
        assert_eq!(output.status.code(), Some(1), "{args:?}: {output:?}");
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(combined.contains("E3023"), "{args:?}: {combined}");
        assert!(!String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| line == "42"));
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn standalone_commands_reject_ast_overflow_before_execution_or_emission() {
    let root = std::env::temp_dir().join(format!("svr-ast-budget-cli-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("oversized.svr");
    std::fs::write(&path, format!("fn main() {{ {} }}", "1;".repeat(125_000))).unwrap();
    for args in [
        vec!["check"],
        vec!["check", "--format", "json"],
        vec!["run"],
        vec!["build"],
        vec!["test"],
    ] {
        let Some(output) = output_or_skip(svr().args(&args).arg(&path)) else {
            std::fs::remove_dir_all(root).unwrap();
            return;
        };
        assert_eq!(output.status.code(), Some(1), "{args:?}: {output:?}");
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(combined.contains("E3020"), "{args:?}: {combined}");
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn formatter_command_preserves_source_and_validates_modes() {
    let root = std::env::temp_dir().join(format!("svr-format-cli-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("source file.svr");
    let source = "// Keep comment\r\nfn main(){print(\"Keep // text\");}";
    std::fs::write(&path, source).unwrap();
    let Some(output) = output_or_skip(svr().arg("fmt").arg(&path)) else {
        std::fs::remove_dir_all(root).unwrap();
        return;
    };
    assert!(output.status.success(), "{:?}", output);
    let expected = "// Keep comment\nfn main() {\n    print(\"Keep // text\");\n}\n";
    assert_eq!(output.stdout, expected.as_bytes());
    assert!(output.stderr.is_empty());
    assert_eq!(std::fs::read(&path).unwrap(), source.as_bytes());
    let checked = svr().args(["fmt", "--check"]).arg(&path).output().unwrap();
    assert_eq!(checked.status.code(), Some(1));
    assert!(checked.stdout.is_empty());
    let written = svr().args(["fmt", "--write"]).arg(&path).output().unwrap();
    if cfg!(any(windows, target_os = "linux")) {
        assert!(written.status.success(), "{:?}", written);
        assert_eq!(std::fs::read(&path).unwrap(), expected.as_bytes());
        let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
        assert!(svr()
            .args(["fmt", "--write"])
            .arg(&path)
            .status()
            .unwrap()
            .success());
        assert_eq!(
            std::fs::metadata(&path).unwrap().modified().unwrap(),
            modified
        );
    } else {
        assert_eq!(written.status.code(), Some(1));
        assert_eq!(std::fs::read(&path).unwrap(), source.as_bytes());
        std::fs::write(&path, expected).unwrap();
    }
    assert!(svr()
        .args(["fmt", "--check"])
        .arg(&path)
        .status()
        .unwrap()
        .success());
    for invalid in ["fn main(){let x=;}", "use library::api fn main(){}"] {
        std::fs::write(&path, invalid).unwrap();
        let result = svr().arg("fmt").arg(&path).output().unwrap();
        assert_eq!(result.status.code(), Some(1));
        assert!(result.stdout.is_empty());
        assert_eq!(std::fs::read(&path).unwrap(), invalid.as_bytes());
        let failed_write = svr().args(["fmt", "--write"]).arg(&path).output().unwrap();
        assert_eq!(failed_write.status.code(), Some(1));
        assert!(failed_write.stdout.is_empty());
        assert_eq!(std::fs::read(&path).unwrap(), invalid.as_bytes());
    }
    std::fs::write(&path, [0xff]).unwrap();
    assert_eq!(
        svr().arg("fmt").arg(&path).output().unwrap().status.code(),
        Some(1)
    );
    for args in [
        vec!["fmt"],
        vec!["fmt", "--check", "--write", "x.svr"],
        vec!["fmt", "--help", "x.svr"],
        vec!["fmt", "x.txt"],
    ] {
        assert_eq!(svr().args(args).output().unwrap().status.code(), Some(2));
    }
    assert!(svr().args(["fmt", "--help"]).status().unwrap().success());
    let dashed = root.join("-source.svr");
    std::fs::write(
        &dashed,
        "fn main(){std::write_text(\"must-not-exist.txt\",\"bad\");}",
    )
    .unwrap();
    assert!(svr()
        .current_dir(&root)
        .args(["fmt", "--", "-source.svr"])
        .output()
        .unwrap()
        .status
        .success());
    assert!(!root.join("must-not-exist.txt").exists());
    std::fs::write(&path, "fn main(){print(unknown);}").unwrap();
    assert!(svr()
        .arg("fmt")
        .arg(&path)
        .output()
        .unwrap()
        .status
        .success());
    std::fs::remove_dir_all(root).unwrap();
}

fn svr() -> Command {
    Command::new(env!("CARGO_BIN_EXE_svr"))
}

#[test]
fn line_uniqueness_matches_both_execution_engines() {
    let source = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/lines-unique.svr"
    );
    let expected = "true\ntrue\ntrue\nfalse\ntrue\nfalse\ntrue\nfalse\ntrue\n";
    let Some(run) = output_or_skip(svr().args(["run", source])) else {
        return;
    };
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).replace("\r\n", "\n"),
        expected
    );
    let Some(build) = output_or_skip(svr().args(["build", "--emit", "js", source])) else {
        return;
    };
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let js_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("lines-unique-{}.cjs", std::process::id()));
    std::fs::write(&js_path, build.stdout).unwrap();
    let js = Command::new("node").arg(&js_path).output().unwrap();
    std::fs::remove_file(js_path).unwrap();
    assert!(
        js.status.success(),
        "{}",
        String::from_utf8_lossy(&js.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&js.stdout).replace("\r\n", "\n"),
        expected
    );
}

#[test]
fn explicit_program_exit_status_matches_generated_javascript() {
    let source = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/process-exit-status.svr"
    );
    let Some(build) = output_or_skip(svr().args(["build", "--emit", "js", source])) else {
        return;
    };
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let js_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("process-exit-status-{}.cjs", std::process::id()));
    std::fs::write(&js_path, build.stdout).unwrap();
    for (mode, status, stdout) in [
        ("last", 3, "before\nafter\n"),
        ("clear", 0, ""),
        ("maximum", 125, ""),
        ("negative", 1, ""),
        ("invalid", 1, ""),
        ("runtime", 1, ""),
    ] {
        let Some(interpreter) = output_or_skip(svr().args(["run", source, "--", mode])) else {
            std::fs::remove_file(&js_path).unwrap();
            return;
        };
        let js = Command::new("node")
            .arg(&js_path)
            .arg(mode)
            .output()
            .unwrap();
        assert_eq!(interpreter.status.code(), Some(status), "{mode}");
        assert_eq!(js.status.code(), Some(status), "{mode}");
        assert_eq!(
            String::from_utf8_lossy(&interpreter.stdout).replace("\r\n", "\n"),
            stdout
        );
        assert_eq!(
            String::from_utf8_lossy(&js.stdout).replace("\r\n", "\n"),
            stdout
        );
        if mode == "invalid" || mode == "negative" {
            assert!(String::from_utf8_lossy(&interpreter.stderr)
                .contains("exit code must be between 0 and 125"));
            assert!(
                String::from_utf8_lossy(&js.stderr).contains("exit code must be between 0 and 125")
            );
        }
    }
    std::fs::remove_file(js_path).unwrap();
}

fn output_or_skip(command: &mut Command) -> Option<Output> {
    match command.output() {
        Ok(output) => Some(output),
        Err(error)
            if is_application_control_block(&error)
                && std::env::var_os("SOVRA_REQUIRE_CLI_EXECUTION").is_none() =>
        {
            eprintln!(
                "skipping CLI integration assertion: Windows Application Control blocked svr.exe"
            );
            None
        }
        Err(error) => panic!("svr should run: {error}"),
    }
}

fn is_application_control_block(error: &io::Error) -> bool {
    error.raw_os_error() == Some(4551)
}

#[test]
fn text_decoding_matches_in_interpreter_and_javascript() {
    let source = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/text-decoding.svr"
    );
    let expected = "true\ncafé\nend\nfalse\nabc\n\ntrue\n\nabc\n9223372036854775807\n-9223372036854775808\ntrue\nfalse\nfalse\nfalse\nfalse\n";
    let Some(run) = output_or_skip(svr().args(["run", source])) else {
        return;
    };
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).replace("\r\n", "\n"),
        expected
    );
    let Some(build) = output_or_skip(svr().args(["build", "--emit", "js", source])) else {
        return;
    };
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let js_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("text-decoding-{}.cjs", std::process::id()));
    std::fs::write(&js_path, build.stdout).unwrap();
    let js = Command::new("node").arg(&js_path).output().unwrap();
    std::fs::remove_file(js_path).unwrap();
    assert!(
        js.status.success(),
        "{}",
        String::from_utf8_lossy(&js.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&js.stdout).replace("\r\n", "\n"),
        expected
    );
}

#[test]
fn scoped_type_example_checks_runs_and_builds() {
    let source = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/scoped-types/main.svr"
    );
    for args in [vec!["check", source], vec!["build", source]] {
        let Some(output) = output_or_skip(svr().args(args)) else {
            return;
        };
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let Some(output) = output_or_skip(svr().args(["run", source])) else {
        return;
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n"),
        "1.5\n1.5\n1\nroot\n"
    );
    let Some(output) = output_or_skip(svr().args(["build", "--emit", "js", source])) else {
        return;
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let js = Command::new("node")
        .arg("-e")
        .arg(String::from_utf8(output.stdout).unwrap())
        .output()
        .unwrap();
    assert!(
        js.status.success(),
        "{}",
        String::from_utf8_lossy(&js.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&js.stdout).replace("\r\n", "\n"),
        "1.5\n1.5\n1\nroot\n"
    );
}

#[test]
fn bounded_messages_keep_codes_ranges_and_valid_long_names() {
    let directory =
        std::env::temp_dir().join(format!("svr-message-budget-cli-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let file = directory.join("main.svr");
    let name = "x".repeat(10000);
    let source = format!("fn main() {{ print({name}); }} fn test_one() {{}}");
    std::fs::write(&file, &source).unwrap();
    let start = source.find(&name).unwrap();
    for command in ["check", "test"] {
        let Some(output) = output_or_skip(svr().args([command, "--format", "json"]).arg(&file))
        else {
            std::fs::remove_dir_all(directory).unwrap();
            return;
        };
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.len() < 8192);
        let report = String::from_utf8(output.stdout).unwrap();
        assert!(report.contains("E3001") && report.contains("[message truncated]"));
        assert!(report.contains(&format!("\"start\":{start},\"end\":{}", start + name.len())));
    }
    for args in [vec!["run"], vec!["build"], vec!["build", "--emit", "js"]] {
        let output = svr().args(args).arg(&file).output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty() && output.stderr.len() < 8192);
        assert!(String::from_utf8_lossy(&output.stderr).contains("[message truncated]"));
    }
    std::fs::write(
        &file,
        format!("fn main() {{ let {name} = 42; print({name}); }}"),
    )
    .unwrap();
    let output = svr().arg("run").arg(&file).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "42");
    let manifest = format!("[{}]\n", "λ".repeat(5000));
    std::fs::write(directory.join("sovra.toml"), &manifest).unwrap();
    let output = svr()
        .args(["check", "--format", "json"])
        .arg(&directory)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.len() < 8192);
    let report = String::from_utf8(output.stdout).unwrap();
    assert!(report.contains("E4010") && report.contains("[message truncated]"));
    assert!(report.contains("\"start\":0,\"end\":10002"));
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn manifest_and_semantic_floods_preserve_bounded_json_and_prevent_execution() {
    let directory =
        std::env::temp_dir().join(format!("svr-stage-flood-cli-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let file = directory.join("main.svr");
    let manifest = directory.join("sovra.toml");
    let source = format!(
        "fn main() {{ {} }} fn test_one() {{}}",
        "print(missing);".repeat(1000)
    );
    std::fs::write(&file, &source).unwrap();
    for command in ["check", "test"] {
        let Some(output) = output_or_skip(svr().args([command, "--format", "json"]).arg(&file))
        else {
            std::fs::remove_dir_all(directory).unwrap();
            return;
        };
        assert_eq!(output.status.code(), Some(1));
        let report = String::from_utf8(output.stdout).unwrap();
        assert_eq!(report.matches("\"code\":").count(), 256);
        assert!(report.contains("E3019"));
        let start = source.match_indices("missing").nth(255).unwrap().0;
        assert!(report.contains(&format!("\"start\":{start},\"end\":{}", start + 7)));
    }
    // Keep a valid root but put the same semantic flood in an unused dependency.
    let lib = directory.join("lib");
    std::fs::create_dir_all(&lib).unwrap();
    std::fs::write(
        lib.join("sovra.toml"),
        "[project]\nname = \"lib\"\nentry = \"lib.svr\"\n",
    )
    .unwrap();
    std::fs::write(lib.join("lib.svr"), &source).unwrap();
    std::fs::write(&file, "fn main() { print(42); } fn test_one() {}").unwrap();
    std::fs::write(
        &manifest,
        "[project]\nname = \"app\"\nentry = \"main.svr\"\n[dependencies.lib]\npath = \"lib\"\n",
    )
    .unwrap();
    let output = svr()
        .args(["check", "--format", "json"])
        .arg(&directory)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report = String::from_utf8(output.stdout).unwrap();
    assert_eq!(report.matches("\"code\":").count(), 256);
    assert!(report.contains("E3019") && report.contains("lib.svr"));
    std::fs::write(&manifest, "bad\n".repeat(1000)).unwrap();
    for command in ["check", "test"] {
        let output = svr()
            .args([command, "--format", "json"])
            .arg(&directory)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let report = String::from_utf8(output.stdout).unwrap();
        assert_eq!(report.matches("\"code\":").count(), 256);
        assert!(report.contains("E4018") && report.contains("sovra.toml"));
        assert!(report.contains("\"start\":1020,\"end\":1023"));
    }
    let binary = directory.join("never-created.exe");
    for args in [
        vec!["run"],
        vec!["build"],
        vec!["build", "--emit", "js"],
        vec![
            "build",
            "--emit",
            "exe",
            "--output",
            binary.to_str().unwrap(),
        ],
    ] {
        let output = svr().args(&args).arg(&directory).output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(!binary.exists());
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn approved_manifest_file_default_guards_initial_classification_and_commands() {
    let directory =
        std::env::temp_dir().join(format!("svr-manifest-default-cli-{}", std::process::id()));
    let app = directory.join("app");
    let lib = directory.join("lib");
    std::fs::create_dir_all(&app).unwrap();
    std::fs::create_dir_all(&lib).unwrap();
    std::fs::write(
        app.join("main.svr"),
        "fn main() { print(42); } fn test_one() {}",
    )
    .unwrap();
    std::fs::write(lib.join("main.svr"), "").unwrap();
    std::fs::write(
        lib.join("sovra.toml"),
        "[project]\nname = \"lib\"\nentry = \"main.svr\"\n",
    )
    .unwrap();
    let path = app.join("sovra.toml");
    let mut manifest =
        b"[project]\nname = \"app\"\nentry = \"main.svr\"\n[dependencies.lib]\npath = \"../lib\"\n"
            .to_vec();
    manifest.resize(1024 * 1024, b' ');
    std::fs::write(&path, &manifest).unwrap();
    let Some(output) = output_or_skip(svr().args(["check", "--format", "json"]).arg(&app)) else {
        std::fs::remove_dir_all(directory).unwrap();
        return;
    };
    assert!(output.status.success(), "{output:?}");
    manifest.push(b' ');
    std::fs::write(&path, &manifest).unwrap();
    let binary = directory.join("never-created.exe");
    for args in [
        vec!["check", "--format", "json"],
        vec!["run"],
        vec!["build", "--emit", "js"],
        vec![
            "build",
            "--emit",
            "exe",
            "--output",
            binary.to_str().unwrap(),
        ],
        vec!["test", "--format", "json"],
    ] {
        let output = svr().args(&args).arg(&app).output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        let report = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            report.contains("E4112") && report.contains("1048576") && report.contains("sovra.toml"),
            "{args:?}: {report}"
        );
        if !args.contains(&"json") {
            assert!(output.stdout.is_empty());
        }
        assert!(!binary.exists());
    }
    // The root classifier must also reject oversized manifests without dependencies.
    let mut plain = b"[project]\nname = \"app\"\nentry = \"main.svr\"\n".to_vec();
    plain.resize(1024 * 1024 + 1, b' ');
    std::fs::write(&path, plain).unwrap();
    let output = svr()
        .args(["check", "--format", "json"])
        .arg(&app)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stdout).contains("E4112"));
    let errors = sovra::compiler::project::check_project(&app).unwrap_err();
    assert_eq!(errors.items.len(), 1);
    assert_eq!(errors.items[0].code, "E4112");
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn approved_aggregate_manifest_default_counts_canonical_packages_once() {
    let directory =
        std::env::temp_dir().join(format!("svr-manifest-total-cli-{}", std::process::id()));
    let app = directory.join("app");
    let names = ["app", "a", "b", "c", "d", "e", "f", "g", "h"];
    let tail = b"[project]\nname = \"h\"\nentry = \"main.svr\"\n";
    for name in names {
        let root = directory.join(name);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("main.svr"),
            if name == "app" {
                "fn main() { print(42); } fn test_one() {}"
            } else {
                ""
            },
        )
        .unwrap();
        let mut manifest = format!("[project]\nname = \"{name}\"\nentry = \"main.svr\"\n");
        if name == "app" {
            for dependency in &names[1..] {
                manifest.push_str(&format!(
                    "[dependencies.{dependency}]\npath = \"../{dependency}\"\n"
                ));
            }
            manifest.push_str("[dependencies.alias]\npath = \"../a\"\n");
        }
        let mut bytes = manifest.into_bytes();
        let size = match name {
            "h" => tail.len(),
            "g" => 1024 * 1024 - tail.len(),
            _ => 1024 * 1024,
        };
        bytes.resize(size, b' ');
        std::fs::write(root.join("sovra.toml"), bytes).unwrap();
    }
    let Some(output) = output_or_skip(svr().args(["check", "--format", "json"]).arg(&app)) else {
        std::fs::remove_dir_all(directory).unwrap();
        return;
    };
    assert!(output.status.success(), "{output:?}");
    let path = directory.join("h/sovra.toml");
    let mut bytes = tail.to_vec();
    bytes.push(b' ');
    std::fs::write(&path, bytes).unwrap();
    for args in [
        vec!["check", "--format", "json"],
        vec!["run"],
        vec!["build"],
        vec!["test", "--format", "json"],
    ] {
        let output = svr().args(&args).arg(&app).output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        let report = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            report.contains("E4112")
                && report.contains("aggregate manifest")
                && report.contains("8388608"),
            "{report}"
        );
        if !args.contains(&"json") {
            assert!(output.stdout.is_empty());
        }
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn approved_dependency_edge_default_stops_before_excess_target_resolution() {
    let directory =
        std::env::temp_dir().join(format!("svr-edge-default-cli-{}", std::process::id()));
    let app = directory.join("app");
    let lib = directory.join("lib");
    std::fs::create_dir_all(&app).unwrap();
    std::fs::create_dir_all(&lib).unwrap();
    std::fs::write(
        app.join("main.svr"),
        "fn main() { print(42); } fn test_one() {}",
    )
    .unwrap();
    std::fs::write(lib.join("main.svr"), "").unwrap();
    std::fs::write(
        lib.join("sovra.toml"),
        "[project]\nname = \"lib\"\nentry = \"main.svr\"\n",
    )
    .unwrap();
    let mut manifest = "[project]\nname = \"app\"\nentry = \"main.svr\"\n".to_owned();
    for index in 0..8192 {
        manifest.push_str(&format!("[dependencies.d{index:05}]\npath = \"../lib\"\n"));
    }
    let path = app.join("sovra.toml");
    std::fs::write(&path, &manifest).unwrap();
    let Some(output) = output_or_skip(svr().args(["check", "--format", "json"]).arg(&app)) else {
        std::fs::remove_dir_all(directory).unwrap();
        return;
    };
    assert!(output.status.success(), "{output:?}");
    manifest.push_str("[dependencies.excess]\npath = \"../does-not-exist\"\n");
    std::fs::write(&path, manifest).unwrap();
    for args in [
        vec!["check", "--format", "json"],
        vec!["run"],
        vec!["test", "--format", "json"],
    ] {
        let output = svr().args(&args).arg(&app).output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        let report = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            report.contains("E4112") && report.contains("edge limit of 8192"),
            "{report}"
        );
        assert!(!report.contains("E4110"));
        if !args.contains(&"json") {
            assert!(output.stdout.is_empty());
        }
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn approved_aggregate_package_source_boundary_and_overflow_apply_to_commands() {
    use std::io::Write;
    let directory =
        std::env::temp_dir().join(format!("svr-aggregate-default-cli-{}", std::process::id()));
    let app = directory.join("app");
    let bytes = 16 * 1024 * 1024;
    let padding = vec![b' '; bytes];
    for name in ["app", "a", "b", "c", "z"] {
        let root = directory.join(name);
        std::fs::create_dir_all(&root).unwrap();
        let dependencies = if name == "app" {
            "[dependencies.a]\npath = \"../a\"\n[dependencies.alias]\npath = \"../a\"\n[dependencies.b]\npath = \"../b\"\n[dependencies.c]\npath = \"../c\"\n[dependencies.z]\npath = \"../z\"\n"
        } else {
            ""
        };
        std::fs::write(
            root.join("sovra.toml"),
            format!("[project]\nname = \"{name}\"\nentry = \"main.svr\"\n{dependencies}"),
        )
        .unwrap();
        let mut file = std::fs::File::create(root.join("main.svr")).unwrap();
        if name == "app" {
            let prefix = b"fn main() { print(42); } fn test_one() { print(7); }";
            file.write_all(prefix).unwrap();
            file.write_all(&padding[prefix.len()..]).unwrap();
        } else if name != "z" {
            file.write_all(&padding).unwrap();
        }
    }
    drop(padding);
    // Exactly 64 MiB across four canonical packages; the alias and empty fifth
    // package must not consume extra allowance.
    let Some(output) = output_or_skip(svr().args(["check", "--format", "json"]).arg(&app)) else {
        std::fs::remove_dir_all(directory).unwrap();
        return;
    };
    assert!(output.status.success(), "{:?}", output);
    assert!(String::from_utf8_lossy(&output.stdout).contains("\"success\":true"));
    let excess = directory.join("z/main.svr");
    std::fs::write(&excess, " ").unwrap();
    let binary = directory.join("never-created.exe");
    for args in [
        vec!["check", "--format", "json"],
        vec!["run"],
        vec!["build"],
        vec!["build", "--emit", "js"],
        vec![
            "build",
            "--emit",
            "exe",
            "--output",
            binary.to_str().unwrap(),
        ],
        vec!["test", "--format", "json"],
    ] {
        let output = svr().args(&args).arg(&app).output().unwrap();
        assert_eq!(output.status.code(), Some(1), "{args:?}: {output:?}");
        let message = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            message.contains("E1003")
                && message.contains("aggregate package")
                && message.contains("67108864"),
            "{args:?}: {message}"
        );
        assert!(message.contains("z") && message.contains("main.svr"));
        if !args.contains(&"json") {
            assert!(output.stdout.is_empty());
        }
        assert!(!binary.exists());
    }
    assert_eq!(std::fs::read(&excess).unwrap(), b" ");
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn default_token_and_diagnostic_limits_apply_to_commands() {
    let directory =
        std::env::temp_dir().join(format!("svr-token-budget-cli-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let file = directory.join("flood.svr");
    for (source, expected, count) in [
        (";".repeat(1_000_001), "E1003", 1),
        ("@".repeat(1000), "E1003", 256),
        ("fn broken() { let x = ; } ".repeat(1000), "E2008", 256),
    ] {
        std::fs::write(&file, source).unwrap();
        for command in ["check", "test"] {
            let Some(output) = output_or_skip(svr().args([command, "--format", "json"]).arg(&file))
            else {
                std::fs::remove_dir_all(directory).unwrap();
                return;
            };
            assert_eq!(output.status.code(), Some(1));
            let report = String::from_utf8(output.stdout).unwrap();
            assert!(
                report.contains(&format!("\"code\":\"{expected}\"")),
                "{report}"
            );
            assert_eq!(report.matches("\"code\":").count(), count);
        }
        let output = svr().args(["fmt", "--check"]).arg(&file).output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains(expected));
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn approved_source_byte_limits_reject_before_execution_or_native_publication() {
    use std::io::Write;
    let directory =
        std::env::temp_dir().join(format!("svr-source-budget-cli-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let file = directory.join("large.svr");
    let binary = directory.join("never-created.exe");
    let mut source = std::fs::File::create(&file).unwrap();
    source
        .write_all(b"fn main() { print(42); } fn test_one() {} ")
        .unwrap();
    source.set_len(16 * 1024 * 1024 + 1).unwrap();
    drop(source);
    for args in [
        vec!["check", "--format", "json"],
        vec!["run"],
        vec!["build"],
        vec!["build", "--emit", "js"],
        vec![
            "build",
            "--emit",
            "exe",
            "--output",
            binary.to_str().unwrap(),
        ],
        vec!["test", "--format", "json"],
    ] {
        let Some(output) = output_or_skip(svr().args(&args).arg(&file)) else {
            std::fs::remove_dir_all(directory).unwrap();
            return;
        };
        assert_eq!(output.status.code(), Some(1), "{args:?}: {output:?}");
        let message = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(message.contains("E1003"), "{args:?}: {message}");
        if !args.contains(&"json") {
            assert!(output.stdout.is_empty());
        }
        assert!(!binary.exists());
    }
    let output = svr().args(["fmt", "--check"]).arg(&file).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("1 MiB"));
    assert_eq!(
        std::fs::metadata(&file).unwrap().len(),
        16 * 1024 * 1024 + 1
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn approved_source_byte_boundary_still_checks_and_runs() {
    let directory =
        std::env::temp_dir().join(format!("svr-source-boundary-cli-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let file = directory.join("boundary.svr");
    let mut bytes = b"fn main() { print(42); }".to_vec();
    bytes.resize(16 * 1024 * 1024, b' ');
    std::fs::write(&file, &bytes).unwrap();
    drop(bytes);
    for command in ["check", "run"] {
        let Some(output) = output_or_skip(svr().arg(command).arg(&file)) else {
            std::fs::remove_dir_all(directory).unwrap();
            return;
        };
        assert!(output.status.success(), "{command}: {output:?}");
        if command == "run" {
            assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "42");
        }
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn dependency_entry_source_uses_approved_byte_limits_and_file_identity() {
    let directory = std::env::temp_dir().join(format!(
        "svr-package-source-budget-cli-{}",
        std::process::id()
    ));
    let app = directory.join("app");
    let lib = directory.join("lib");
    std::fs::create_dir_all(&app).unwrap();
    std::fs::create_dir_all(&lib).unwrap();
    std::fs::write(
        app.join("sovra.toml"),
        "[project]\nname = \"app\"\nentry = \"main.svr\"\n[dependencies.util]\npath = \"../lib\"\n",
    )
    .unwrap();
    std::fs::write(app.join("main.svr"), "fn main() { print(42); }").unwrap();
    std::fs::write(
        lib.join("sovra.toml"),
        "[project]\nname = \"lib\"\nentry = \"lib.svr\"\n",
    )
    .unwrap();
    let entry = lib.join("lib.svr");
    std::fs::File::create(&entry)
        .unwrap()
        .set_len(16 * 1024 * 1024 + 1)
        .unwrap();
    for args in [
        vec!["check", "--format", "json"],
        vec!["run"],
        vec!["build"],
        vec!["test", "--format", "json"],
    ] {
        let Some(output) = output_or_skip(svr().args(&args).arg(&app)) else {
            std::fs::remove_dir_all(directory).unwrap();
            return;
        };
        assert_eq!(output.status.code(), Some(1), "{args:?}: {output:?}");
        let message = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            message.contains("E1003") && message.contains("lib.svr"),
            "{message}"
        );
        if !args.contains(&"json") {
            assert!(output.stdout.is_empty());
        }
    }
    std::fs::write(
        &entry,
        "mod math { export fn value() -> Int { return 1; } }",
    )
    .unwrap();
    std::fs::write(
        app.join("main.svr"),
        format!("{}fn main() {{}}", "use util::math;\n".repeat(200_001)),
    )
    .unwrap();
    let output = svr()
        .args(["check", "--format", "json"])
        .arg(&app)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report = String::from_utf8(output.stdout).unwrap();
    assert!(report.contains("E1003") && report.contains("main.svr"));
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn invalid_record_literals_retain_json_value_errors_and_stable_missing_fields() {
    let directory =
        std::env::temp_dir().join(format!("svr-invalid-record-cli-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let file = directory.join("main.svr");
    let source = "struct Point { z: Int, a: Int, m: Int } fn main() { Missing { x: missing_unknown }; Point { z: 1, z: missing_duplicate }; print(later); }";
    std::fs::write(&file, source).unwrap();
    let Some(output) = output_or_skip(svr().args(["check", "--format", "json"]).arg(&file)) else {
        std::fs::remove_dir_all(directory).unwrap();
        return;
    };
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let report = String::from_utf8(output.stdout).unwrap();
    assert!(report.contains("\"code\":\"E3004\""));
    assert!(report.contains("\"code\":\"E3008\""));
    for name in ["missing_unknown", "missing_duplicate", "later"] {
        assert_eq!(
            report
                .matches(&format!("undefined variable `{name}`"))
                .count(),
            1,
            "{report}"
        );
        let start = source.find(name).unwrap();
        assert!(
            report.contains(&format!("\"start\":{start},\"end\":{}", start + name.len())),
            "{report}"
        );
    }
    assert!(report.find("missing field `a`").unwrap() < report.find("missing field `m`").unwrap());
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn invalid_targets_retain_independent_errors_in_json() {
    let directory =
        std::env::temp_dir().join(format!("svr-invalid-target-cli-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let file = directory.join("main.svr");
    let source = "fn main() { missing_target = missing_value; let mut scalar = 1; scalar[0] = missing_rhs; 42(missing_argument); print(later); }";
    std::fs::write(&file, source).unwrap();
    let Some(output) = output_or_skip(svr().args(["check", "--format", "json"]).arg(&file)) else {
        std::fs::remove_dir_all(directory).unwrap();
        return;
    };
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let report = String::from_utf8(output.stdout).unwrap();
    assert!(report.contains("\"success\":false"));
    for name in [
        "missing_target",
        "missing_value",
        "missing_rhs",
        "missing_argument",
        "later",
    ] {
        assert_eq!(
            report
                .matches(&format!("undefined variable `{name}`"))
                .count(),
            1,
            "{report}"
        );
        let start = source.find(name).unwrap();
        assert!(
            report.contains(&format!("\"start\":{start},\"end\":{}", start + name.len())),
            "{report}"
        );
    }
    for args in [vec!["run"], vec!["build"], vec!["build", "--emit", "js"]] {
        let output = svr().args(args).arg(&file).output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(
            output.stdout.is_empty(),
            "invalid source must not execute or emit code"
        );
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn entry_return_aliases_are_validated_before_execution_or_emission() {
    let directory =
        std::env::temp_dir().join(format!("svr-entry-return-cli-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let file = directory.join("main.svr");
    for (source, valid) in [
        (
            "type Base = Unit; type Result = Base; fn main() -> Result { print(42); }",
            true,
        ),
        (
            "type Base = Int; type Result = Base; fn main() -> Result { print(42); return 1; }",
            false,
        ),
        (
            "struct Result { value: Int } fn main() -> Result { return Result { value: 1 }; }",
            false,
        ),
    ] {
        std::fs::write(&file, source).unwrap();
        let path = file.to_str().unwrap();
        for args in [
            vec!["check", path],
            vec!["check", "--format", "json", path],
            vec!["run", path],
            vec!["build", path],
            vec!["build", "--emit", "js", path],
        ] {
            let Some(output) = output_or_skip(svr().args(&args)) else {
                std::fs::remove_dir_all(&directory).unwrap();
                return;
            };
            assert_eq!(
                output.status.code(),
                Some(if valid { 0 } else { 1 }),
                "{args:?}: {output:?}"
            );
            if !valid {
                let diagnostic = format!(
                    "{}{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
                assert!(diagnostic.contains("E3010"), "{args:?}: {diagnostic}");
                if !args.contains(&"json") {
                    assert!(
                        output.stdout.is_empty(),
                        "invalid main must not execute or emit code"
                    );
                }
            } else if args[0] == "run" {
                assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "42");
            }
        }
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn long_alias_chain_checks_runs_and_builds_without_stack_overflow() {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("alias-cli-{}-{suffix}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let file = directory.join("main.svr");
    let mut source = String::new();
    for index in 0..20000 {
        source.push_str(&format!(
            "type T{index} = {};\n",
            if index == 19999 {
                "Float".into()
            } else {
                format!("T{}", index + 1)
            }
        ));
    }
    source.push_str("fn main() { let x: T0 = 3; print(x / 2); }");
    std::fs::write(&file, source).unwrap();
    let path = file.to_str().unwrap();
    for args in [
        vec!["check", path],
        vec!["build", path],
        vec!["build", "--emit", "js", path],
    ] {
        let Some(output) = output_or_skip(svr().args(args)) else {
            return;
        };
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let Some(output) = output_or_skip(svr().args(["run", path])) else {
        return;
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "1.5");
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn package_cli_rejects_private_calls_with_original_json_locations() {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("package-cli-{}-{suffix}", std::process::id()));
    let app = directory.join("app");
    let library = directory.join("lib");
    std::fs::create_dir_all(&app).unwrap();
    std::fs::create_dir_all(&library).unwrap();
    std::fs::write(
        app.join("sovra.toml"),
        "[project]\nname = \"app\"\nentry = \"main.svr\"\n[dependencies.util]\npath = \"../lib\"",
    )
    .unwrap();
    std::fs::write(
        library.join("sovra.toml"),
        "[project]\nname = \"lib\"\nentry = \"lib.svr\"",
    )
    .unwrap();
    std::fs::write(
        library.join("lib.svr"),
        "mod math { fn secret() -> Int { return 42 } }",
    )
    .unwrap();
    std::fs::write(
        app.join("main.svr"),
        "// λ\r\nuse util::math;\r\nfn main() { util::math::secret(); }",
    )
    .unwrap();
    let path = app.to_str().unwrap();
    let Some(report) = output_or_skip(svr().args(["check", "--format=json", path])) else {
        return;
    };
    assert_eq!(report.status.code(), Some(1));
    assert_json_report(
        &report,
        r#"
        assert.equal(report.success, false);
        assert.equal(report.diagnostics.length, 1);
        const error = report.diagnostics[0];
        assert.equal(error.code, 'E3004');
        assert.equal(error.location.line, 2);
        const bytes = require('node:fs').readFileSync(error.location.file);
        assert.equal(bytes.subarray(error.location.start, error.location.end).toString(), 'util::math::secret');
    "#,
    );
    for args in [
        vec!["run", path],
        vec!["build", path],
        vec!["build", "--emit", "js", path],
    ] {
        let Some(output) = output_or_skip(svr().args(args)) else {
            return;
        };
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("E3004"));
        assert!(output.stdout.is_empty());
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn local_library_consumer_checks_runs_and_builds_on_both_engines() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/local-packages/app");
    let Some(checked) = output_or_skip(svr().args(["check", "--format=json", root])) else {
        return;
    };
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stdout)
    );
    assert_json_report(&checked, "assert.equal(report.success, true); assert.equal(report.kind, 'project'); assert.deepEqual(report.diagnostics, []);");
    let Some(run) = output_or_skip(svr().args(["run", root])) else {
        return;
    };
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout)
            .replace("\r\n", "\n")
            .trim(),
        "49\n25\n1.5"
    );
    let Some(ir) = output_or_skip(svr().args(["build", root])) else {
        return;
    };
    assert!(ir.status.success());
    assert!(String::from_utf8_lossy(&ir.stdout).contains("@package1::arithmetic::square"));
    let Some(js) = output_or_skip(svr().args(["build", "--emit", "js", root])) else {
        return;
    };
    assert!(js.status.success());
    let output = Command::new("node")
        .arg("-e")
        .arg(String::from_utf8(js.stdout).unwrap())
        .output()
        .expect("Node required");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout)
            .replace("\r\n", "\n")
            .trim(),
        "49\n25\n1.5"
    );
}

#[test]
fn library_foundations_example_checks_runs_and_builds() {
    let source = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/library-foundations/main.svr"
    );
    for args in [vec!["check", source], vec!["build", source]] {
        let Some(output) = output_or_skip(svr().args(args)) else {
            return;
        };
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let Some(interpreted) = output_or_skip(svr().args(["run", source])) else {
        return;
    };
    assert!(interpreted.status.success());
    assert_eq!(
        String::from_utf8_lossy(&interpreted.stdout).replace("\r\n", "\n"),
        "square: 49\nletters: 5\n"
    );
    let Some(built) = output_or_skip(svr().args(["build", "--emit", "js", source])) else {
        return;
    };
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let js = Command::new("node")
        .arg("-e")
        .arg(String::from_utf8(built.stdout).unwrap())
        .output()
        .expect("Node is required for backend validation");
    assert!(
        js.status.success(),
        "{}",
        String::from_utf8_lossy(&js.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&js.stdout).replace("\r\n", "\n"),
        "square: 49\nletters: 5\n"
    );
}

#[test]
fn structural_depth_errors_replace_subprocess_crashes() {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("depth-cli-{}-{suffix}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("main.svr");
    let source_path = path.to_str().unwrap();
    for expression in [
        format!("{}1{}", "(".repeat(2048), ")".repeat(2048)),
        format!("{}1{}", "f(".repeat(2048), ")".repeat(2048)),
        vec!["1"; 2049].join("+"),
        format!("{}1{}", "[".repeat(2048), "]".repeat(2048)),
        format!("root{}", ".field".repeat(2048)),
        format!("root{}", "[0]".repeat(2048)),
        format!("{}1{}", "Box { value: ".repeat(2048), "}".repeat(2048)),
        format!("{}print(1){}", "if (true) {".repeat(2048), "}".repeat(2048)),
        format!(
            "{}print(1){}",
            "while (false) {".repeat(2048),
            "}".repeat(2048)
        ),
    ] {
        let source = format!("// Unicode: λ\r\nfn main() {{ {expression}; }}");
        std::fs::write(&path, &source).unwrap();
        for args in [
            vec!["check", source_path],
            vec!["run", source_path],
            vec!["build", source_path],
            vec!["build", "--emit", "js", source_path],
            vec!["fmt", source_path],
            vec!["fmt", "--check", source_path],
        ] {
            let Some(output) = output_or_skip(svr().args(args)) else {
                return;
            };
            assert_eq!(
                output.status.code(),
                Some(1),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(String::from_utf8_lossy(&output.stderr).contains("E2007"));
            assert!(output.stdout.is_empty());
            assert_eq!(std::fs::read_to_string(&path).unwrap(), source);
        }
        let Some(output) = output_or_skip(svr().args(["check", "--format", "json", source_path]))
        else {
            return;
        };
        assert_eq!(output.status.code(), Some(1));
        assert_json_report(
            &output,
            r#"
            assert.equal(report.success, false);
            assert.equal(report.diagnostics.length, 1);
            const error = report.diagnostics[0];
            assert.equal(error.code, 'E2007');
            assert.equal(error.location.line, 1);
            const source = require('node:fs').readFileSync(error.location.file);
            assert.ok(['(', '+', '[', '{', '.'].includes(source.subarray(error.location.start, error.location.end).toString()));
            assert.ok(error.message.includes('128'));
        "#,
        );
    }
    std::fs::write(
        directory.join("sovra.toml"),
        "[project]\nname = \"depth\"\nentry = \"main.svr\"\n",
    )
    .unwrap();
    let Some(output) = output_or_skip(svr().args([
        "check",
        "--service-calls",
        "--format",
        "json",
        directory.to_str().unwrap(),
    ])) else {
        return;
    };
    assert_eq!(output.status.code(), Some(1));
    assert_json_report(
        &output,
        r#"
        assert.equal(report.service_coverage.complete, false);
        assert.deepEqual(report.diagnostics.map(d => d.code), ['E4096']);
        assert.ok(report.service_coverage.files[0].reason.includes('128'));
        assert.deepEqual(report.member_calls, []);
    "#,
    );
    std::fs::remove_dir_all(&directory).unwrap();
}

fn assert_json_report(output: &Output, assertions: &str) {
    use std::io::Write as _;
    use std::process::Stdio;
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let script = format!(
        "const assert = require('node:assert/strict'); const report = JSON.parse(require('node:fs').readFileSync(0, 'utf8')); assert.equal(report.schema_version, 1); {assertions}"
    );
    let mut child = Command::new("node")
        .args(["-e", &script])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Node.js required for JSON contract tests");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(&output.stdout)
        .expect("write JSON");
    let result = child.wait_with_output().expect("Node.js result");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn doctor_reports_source_health_without_optional_tools_and_fails_required_targets() {
    let Some(output) = output_or_skip(svr().args(["doctor", "--format=json"]).env("PATH", ""))
    else {
        return;
    };
    assert_eq!(output.status.code(), Some(0));
    assert_json_report(
        &output,
        r#"
        assert.equal(report.kind, 'doctor');
        assert.equal(report.target, 'interpreter');
        assert.equal(report.success, true);
        assert.equal(report.compiler.version, '1.0.0');
        assert.ok(report.compiler.executable.endsWith(process.platform === 'win32' ? 'svr.exe' : 'svr'));
        assert.deepEqual(report.checks.map(c => c.id), ['compiler.identity','compiler.self_test','install.integrity','native.runtime','tools.node','tools.rustc','tools.cargo','native.linker','desktop.host']);
        assert.equal(report.checks.find(c => c.id === 'install.integrity').status, 'not_checked');
        for (const id of ['tools.node','tools.rustc','tools.cargo']) {
            const check = report.checks.find(c => c.id === id);
            assert.equal(check.required, false);
            assert.equal(check.status, 'warning');
            assert.equal(typeof check.remediation, 'string');
        }
    "#,
    );
    for target in ["js", "exe"] {
        let Some(output) = output_or_skip(
            svr()
                .args(["doctor", "--format", "json", "--target", target])
                .env("PATH", ""),
        ) else {
            return;
        };
        assert_eq!(output.status.code(), Some(1));
        assert_json_report(&output, "assert.equal(report.success, false); assert.ok(report.checks.some(c => c.required && c.status === 'fail')); ");
    }
}

#[test]
fn doctor_help_and_invalid_options_do_not_probe_or_emit_partial_json() {
    let Some(output) = output_or_skip(svr().args(["doctor", "--help"])) else {
        return;
    };
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Usage: svr doctor"));
    for args in [
        vec!["--format=json", "--format=human"],
        vec!["--target=js", "--target=exe"],
        vec!["--format"],
        vec!["--target=invalid"],
        vec!["source.svr"],
        vec!["--help", "--format=json"],
    ] {
        let Some(output) = output_or_skip(svr().arg("doctor").args(args)) else {
            return;
        };
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn check_json_recovers_declarations_without_per_token_error_floods() {
    let source = format!(
        "{}/tests/fixtures/declaration-recovery.svr",
        env!("CARGO_MANIFEST_DIR")
    );
    let Some(output) = output_or_skip(svr().args(["check", "--format=json", &source])) else {
        return;
    };
    assert_eq!(output.status.code(), Some(1));
    assert_json_report(
        &output,
        r#"
        assert.equal(report.success, false);
        assert.equal(report.diagnostics.length, 3);
        assert.deepEqual(report.diagnostics.map(d => d.code), ['E2000', 'E2000', 'E2001']);
        const bytes = require('node:fs').readFileSync(report.target);
        for (const diagnostic of report.diagnostics) {
            assert.equal(diagnostic.location.file, report.target);
            assert.ok(diagnostic.location.end <= bytes.length);
        }
        assert.deepEqual(report.diagnostics.map(d => d.location.line), [0, 1, 2]);
        "#,
    );
}

#[test]
fn nested_statement_recovery_reports_original_ranges_and_prevents_execution() {
    let source = format!(
        "{}/tests/fixtures/statement-recovery.svr",
        env!("CARGO_MANIFEST_DIR")
    );
    let Some(output) = output_or_skip(svr().args(["check", "--format=json", &source])) else {
        return;
    };
    assert_eq!(output.status.code(), Some(1));
    assert_json_report(
        &output,
        r#"
        assert.equal(report.success, false);
        assert.equal(report.diagnostics.length, 4);
        assert.deepEqual(report.diagnostics.map(d => d.code), Array(4).fill('E2001'));
        assert.deepEqual(report.diagnostics.map(d => d.location.line), [2, 3, 5, 7]);
        const bytes = require('node:fs').readFileSync(report.target);
        for (const {location} of report.diagnostics) {
            assert.equal(location.file, report.target);
            assert.equal(bytes.subarray(location.start, location.end).toString(), ';');
        }
        "#,
    );
    for arguments in [vec!["run", &source], vec!["build", "--emit", "js", &source]] {
        let Some(output) = output_or_skip(svr().args(arguments)) else {
            return;
        };
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty(), "invalid source produced output");
        assert!(String::from_utf8_lossy(&output.stderr).contains("E2001"));
    }
}

#[test]
fn malformed_index_reports_one_error_per_fault_and_retains_later_locations() {
    let source = format!(
        "{}/tests/fixtures/index-recovery.svr",
        env!("CARGO_MANIFEST_DIR")
    );
    let Some(output) = output_or_skip(svr().args(["check", "--format=json", &source])) else {
        return;
    };
    assert_eq!(output.status.code(), Some(1));
    assert_json_report(
        &output,
        r#"
        assert.equal(report.success, false);
        assert.deepEqual(report.diagnostics.map(d => d.code), ['E2001', 'E2005', 'E2001']);
        assert.deepEqual(report.diagnostics.map(d => d.location.line), [1, 2, 3]);
        const bytes = require('node:fs').readFileSync(report.target);
        for (const {location} of report.diagnostics) {
            assert.equal(location.file, report.target);
            assert.equal(bytes.subarray(location.start, location.end).toString(), ';');
        }
        "#,
    );
}

#[test]
fn check_json_reports_source_success() {
    let source = format!("{}/examples/functions/main.svr", env!("CARGO_MANIFEST_DIR"));
    let Some(output) = output_or_skip(svr().args(["check", "--format=json", &source])) else {
        return;
    };
    assert_eq!(output.status.code(), Some(0));
    assert_json_report(&output, "assert.equal(report.kind, 'source'); assert.equal(report.success, true); assert.deepEqual(report.diagnostics, []); assert.ok(report.target.endsWith('main.svr'));");
}

#[test]
fn check_project_diagnostics_identify_scanned_files() {
    for (fixture, expected) in [
        ("project-locations", "[['E4034', 'main.svr', '  route broken', 1], ['E4080', 'auth.svr', '  allow broken', 0], ['E4033', 'main.svr', '  route GET \"/missing\" -> absent.handler', 2], ['E4081', 'auth.svr', '  allow user to read on Missing', 1], ['E4003', 'sovra.toml', 'target = \"unknown\"', 5]]"),
        ("manifest-locations", "[['E4010', 'sovra.toml', '[unknown]', 0]]"),
    ] {
        let project = format!("{}/tests/fixtures/{fixture}", env!("CARGO_MANIFEST_DIR"));
        let Some(output) = output_or_skip(svr().args(["check", "--format=json", &project])) else {
            return;
        };
        assert_eq!(output.status.code(), Some(1));
        assert_json_report(&output, &format!(r#"
            assert.equal(report.kind, 'project');
            assert.equal(report.success, false);
            if (report.target.endsWith('project-locations')) {{
                for (const code of ['E4024', 'E4062']) {{
                    const d = report.diagnostics.find(d => d.code === code);
                    assert.ok(d.location.file.endsWith('main.svr'));
                    assert.equal(d.location.line, code === 'E4024' ? 3 : 4);
                }}
            }}
            for (const [code, file, spelling, line] of {expected}) {{
                const d = report.diagnostics.find(d => d.code === code);
                const path = require('node:path').join(report.target, file);
                const bytes = require('node:fs').readFileSync(path);
                const start = bytes.indexOf(spelling);
                assert.equal(require('node:path').resolve(d.location.file),
                    require('node:path').resolve(path));
                assert.deepEqual(d.location, {{ file: d.location.file, start,
                    end: start + Buffer.byteLength(spelling), line, column: 0 }});
            }}
        "#));
        let Some(output) = output_or_skip(svr().args(["check", &project])) else {
            return;
        };
        assert_eq!(output.status.code(), Some(1));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(if fixture == "project-locations" { "main.svr:2:1" } else { "sovra.toml:1:1" }));
    }
}

#[test]
fn check_json_reports_expression_locations() {
    let source = format!(
        "{}/tests/fixtures/expression-locations.svr",
        env!("CARGO_MANIFEST_DIR")
    );
    let Some(output) = output_or_skip(svr().args(["check", "--format=json", &source])) else {
        return;
    };
    assert_eq!(output.status.code(), Some(1));
    assert_json_report(
        &output,
        r#"
        assert.equal(report.success, false);
        const bytes = require('node:fs').readFileSync(report.target);
        for (const [code, spelling, line, column] of [
            ['E3007', '42', 1, 25], ['E3001', 'missing', 2, 10]
        ]) {
            const diagnostic = report.diagnostics.find(d => d.code === code);
            const start = bytes.indexOf(spelling);
            assert.deepEqual(diagnostic.location, {
                file: report.target, start, end: start + spelling.length, line, column
            });
        }
    "#,
    );
}

#[test]
fn check_json_reports_parameter_diagnostic_location() {
    let source = format!(
        "{}/tests/fixtures/untyped-parameter.svr",
        env!("CARGO_MANIFEST_DIR")
    );
    let Some(output) = output_or_skip(svr().args(["check", &source, "--format", "json"])) else {
        return;
    };
    assert_eq!(output.status.code(), Some(1));
    assert_json_report(&output, "assert.equal(report.kind, 'source'); assert.equal(report.success, false); const d = report.diagnostics.find(d => d.code === 'E3014'); assert.equal(d.severity, 'error'); assert.ok(d.message.includes('value: Type')); assert.equal(d.location.file, report.target); assert.equal(d.location.start, 10); assert.equal(d.location.end, 15); assert.equal(d.location.line, 0); assert.equal(d.location.column, 10);");
}

#[test]
fn unicode_crlf_diagnostics_keep_source_bytes_scalar_columns_and_human_locations() {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "svr-unicode-diagnostic-{}-{suffix}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("source-λ-😀.svr");
    // Keep source CRLF and escapes independent of checkout newline conversion.
    let source = "// 😀 preface\r\nfn main() {\r\n\tprint(\"e\u{301}😀\\n\"); std::len(42);\r\n\tprint(missing);\r\n}\r\n";
    std::fs::write(&path, source).unwrap();
    (|| {
        let Some(json) = output_or_skip(svr().args(["check", "--format", "json"]).arg(&path))
        else {
            return;
        };
        assert_eq!(json.status.code(), Some(1));
        assert_json_report(
            &json,
            r#"
            assert.equal(report.kind, 'source');
            assert.equal(report.success, false);
            assert.equal(report.diagnostics.length, 2);
            const fs = require('node:fs');
            const bytes = fs.readFileSync(report.target);
            const source = bytes.toString('utf8');
            assert.ok(report.target.endsWith('source-λ-😀.svr'));
            assert.ok(source.includes('\r\n'));
            const expected = [['E3007', '42', 2], ['E3001', 'missing', 3]];
            for (const [code, spelling, line] of expected) {
                const d = report.diagnostics.find(value => value.code === code);
                const start = bytes.indexOf(Buffer.from(spelling));
                const sourceLine = source.split('\r\n')[line];
                const prefix = sourceLine.slice(0, sourceLine.indexOf(spelling));
                const column = Array.from(prefix).length;
                assert.deepEqual(d.location, { file: report.target, start,
                    end: start + Buffer.byteLength(spelling), line, column });
                assert.equal(bytes.subarray(d.location.start, d.location.end).toString(), spelling);
                if (code === 'E3007') {
                    assert.notEqual(column, prefix.length); // UTF-16 differs.
                    assert.notEqual(column, Buffer.byteLength(prefix)); // UTF-8 differs.
                    assert.ok(prefix.includes('e\u0301😀\\n'));
                } else {
                    assert.equal(column, 7); // A tab is one scalar, not a tab stop.
                }
            }
        "#,
        );
        let Some(human) = output_or_skip(svr().arg("check").arg(&path)) else {
            return;
        };
        assert_eq!(human.status.code(), Some(1));
        assert!(human.stdout.is_empty());
        let stderr = String::from_utf8(human.stderr).unwrap();
        for (code, spelling, line) in [("E3007", "42", 2), ("E3001", "missing", 3)] {
            let source_line = source.split("\r\n").nth(line).unwrap();
            let column = source_line[..source_line.find(spelling).unwrap()]
                .chars()
                .count();
            assert!(
                stderr.contains(&format!("error[{code}] at {}:{}:", line + 1, column + 1)),
                "{stderr}"
            );
        }
        assert_eq!(std::fs::read_to_string(&path).unwrap(), source);
        let valid = source
            .replace("std::len(42)", "std::len(\"valid\")")
            .replace("print(missing)", "print(42)");
        std::fs::write(&path, &valid).unwrap();
        let Some(checked) = output_or_skip(svr().args(["check", "--format", "json"]).arg(&path))
        else {
            return;
        };
        assert_eq!(checked.status.code(), Some(0));
        assert_json_report(
            &checked,
            "assert.equal(report.success, true); assert.deepEqual(report.diagnostics, []);",
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), valid);
    })();
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn check_json_distinguishes_project_validation() {
    let project = format!("{}/examples/fielddesk", env!("CARGO_MANIFEST_DIR"));
    let Some(output) = output_or_skip(svr().args(["check", "--format", "json", &project])) else {
        return;
    };
    assert_eq!(output.status.code(), Some(0));
    assert_json_report(&output, "assert.equal(report.kind, 'project'); assert.equal(report.success, true); assert.deepEqual(report.diagnostics, []);");

    let no_manifest = format!("{}/examples/hello-world", env!("CARGO_MANIFEST_DIR"));
    let Some(output) = output_or_skip(svr().args(["check", "--format", "json", &no_manifest]))
    else {
        return;
    };
    assert_eq!(output.status.code(), Some(1));
    assert_json_report(&output, "assert.equal(report.kind, 'project'); assert.equal(report.success, false); assert.equal(report.diagnostics[0].code, 'E4000'); assert.equal(report.diagnostics[0].location, null);");
}

#[test]
fn check_json_reports_io_errors() {
    let missing = format!(
        "{}/tests/fixtures/no-such-source.svr",
        env!("CARGO_MANIFEST_DIR")
    );
    let Some(output) = output_or_skip(svr().args(["check", "--format", "json", &missing])) else {
        return;
    };
    assert_eq!(output.status.code(), Some(1));
    assert_json_report(&output, "assert.equal(report.kind, null); assert.equal(report.success, false); assert.equal(report.diagnostics[0].code, 'E0001'); assert.equal(report.diagnostics[0].location, null);");
}

#[test]
fn oversized_io_path_messages_keep_json_path_identity_and_previous_clipping() {
    let path = format!("{}.svr", "x".repeat(20000));
    let suffix = " [message truncated]";
    let mut expected = format!("svr: cannot inspect `{path}`");
    expected.truncate(4096 - suffix.len());
    expected.push_str(suffix);
    let Some(human) = output_or_skip(svr().args(["check", &path])) else {
        return;
    };
    assert_eq!(human.status.code(), Some(1));
    assert!(human.stdout.is_empty());
    assert_eq!(
        String::from_utf8(human.stderr).unwrap().trim_end(),
        expected
    );
    let json = svr()
        .args(["check", "--format", "json", &path])
        .output()
        .unwrap();
    assert_eq!(json.status.code(), Some(1));
    assert_json_report(&json, &format!("assert.equal(report.target, {path:?}); assert.equal(report.kind, null); assert.equal(report.success, false); assert.equal(report.diagnostics.length, 1); assert.equal(report.diagnostics[0].code, 'E0001'); assert.equal(report.diagnostics[0].location, null); assert.equal(report.diagnostics[0].message, {expected:?});"));
}

#[test]
fn human_io_and_argument_diagnostics_escape_terminal_controls_but_json_keeps_target() {
    let path = "missing-λ-\u{001b}[31mred\r\nFORGED-\u{009b}x\u{007f}.svr";
    for args in [
        vec!["check", path],
        vec!["check", "--format", path],
        vec![path],
    ] {
        let Some(output) = output_or_skip(svr().args(&args)) else {
            return;
        };
        assert!(!output.status.success());
        let message = String::from_utf8(output.stderr).unwrap();
        assert!(!message.contains('\u{001b}'), "raw ESC in human diagnostic");
        assert!(
            !message.contains('\r')
                && !message.contains('\u{009b}')
                && !message.contains('\u{007f}')
        );
        assert_eq!(message.lines().count(), if args.len() == 1 { 2 } else { 1 });
        assert!(
            message.contains("\\u{001B}")
                && message.contains("\\u{000D}")
                && message.contains("\\u{000A}")
        );
        assert!(message.contains('λ'));
    }
    let json = svr()
        .args(["check", "--format", "json", path])
        .output()
        .unwrap();
    assert_eq!(json.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&json.stdout)
        .chars()
        .any(|character| ('\u{007f}'..='\u{009f}').contains(&character)));
    assert_json_report(&json, &format!("assert.equal(report.target, {path:?}); assert.equal(report.diagnostics[0].code, 'E0001'); assert.equal(report.diagnostics[0].location, null); assert.ok(report.diagnostics[0].message.includes({path:?}));"));
}

#[test]
fn delegated_formatter_and_doctor_errors_preserve_help_and_escape_controls() {
    let argument = "--bad-λ-\u{001b}[31m\r\nFORGED-\u{009b}\u{007f}";
    for command in ["fmt", "doctor"] {
        let Some(output) = output_or_skip(svr().args([command, argument])) else {
            return;
        };
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(!error
            .chars()
            .any(|character| (character <= '\u{001f}' && character != '\n')
                || ('\u{007f}'..='\u{009f}').contains(&character)));
        assert!(error.contains(&format!("Usage: svr {command}")));
        let first = error.lines().next().unwrap();
        assert!(
            first.contains("\\u{001B}")
                && first.contains("\\u{000D}")
                && first.contains("\\u{000A}")
        );
        assert!(first.contains('λ') && first.contains("FORGED"));
        assert_eq!(
            error.lines().filter(|line| line.contains("FORGED")).count(),
            1
        );
    }
}

#[test]
fn primary_project_cli_enforces_approved_source_byte_limit() {
    let root = std::env::temp_dir().join(format!("svr-primary-default-cli-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("sovra.toml"),
        "[project]\nname = \"scan\"\nentry = \"main.svr\"\n",
    )
    .unwrap();
    std::fs::write(root.join("main.svr"), vec![b' '; 16 * 1024 * 1024 + 1]).unwrap();
    for args in [
        vec!["check"],
        vec!["check", "--format", "json"],
        vec!["check", "--service-calls"],
    ] {
        let Some(output) = output_or_skip(svr().args(&args).arg(&root)) else {
            std::fs::remove_dir_all(root).unwrap();
            return;
        };
        assert_eq!(output.status.code(), Some(1));
        if args.contains(&"json") {
            assert_json_report(&output, "assert.equal(report.kind, 'project'); assert.equal(report.success, false); assert.equal(report.diagnostics.length, 1); assert.equal(report.diagnostics[0].code, 'E4150'); assert.equal(report.diagnostics[0].location, null);");
        } else {
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains("E4150"));
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn compiler_human_diagnostics_escape_source_controls_on_check_run_and_build() {
    let root = std::env::temp_dir().join(format!("svr-terminal-diagnostic-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("main.svr");
    std::fs::write(&source, "fn main() { \u{001b} }").unwrap();
    for command in ["check", "run", "build"] {
        let Some(output) = output_or_skip(svr().arg(command).arg(&source)) else {
            std::fs::remove_dir_all(root).unwrap();
            return;
        };
        assert_eq!(output.status.code(), Some(1));
        let message = String::from_utf8(output.stderr).unwrap();
        assert!(message.contains("E1000"));
        assert!(
            !message.contains('\u{001b}'),
            "raw source control in {command} diagnostic"
        );
        assert!(message.contains("\\u{001B}"));
    }
    // Program output remains under program control; this policy applies only
    // to diagnostic lines generated by the CLI.
    std::fs::write(&source, "fn main() { print(\"\u{001b}[31mλ\"); }").unwrap();
    let output = svr().arg("run").arg(&source).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "\u{001b}[31mλ\n");
    assert!(output.stderr.is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn check_json_respects_option_terminator() {
    for path in ["--help", "-h"] {
        let Some(output) = output_or_skip(
            svr()
                .current_dir(format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR")))
                .args(["check", "--format=json", "--", path]),
        ) else {
            return;
        };
        assert_eq!(output.status.code(), Some(1));
        assert_json_report(&output, "assert.equal(report.success, false); assert.equal(report.kind, null); assert.equal(report.diagnostics[0].code, 'E0001');");
    }
}

#[test]
fn check_rejects_invalid_format_arguments() {
    for arguments in [
        vec!["check", "--format", "xml", "README.md"],
        vec!["check", "--format"],
        vec!["check", "--format", "json"],
        vec!["check", "--format=json", "--format=human", "README.md"],
        vec!["check", "--format=json", "README.md"],
    ] {
        let Some(output) = output_or_skip(svr().args(&arguments)) else {
            return;
        };
        assert_eq!(output.status.code(), Some(2), "{arguments:?}");
        assert!(output.stdout.is_empty(), "{arguments:?}");
        assert!(!output.stderr.is_empty(), "{arguments:?}");
    }
}

#[test]
fn source_commands_reject_unknown_annotations() {
    let source = format!(
        "{}/tests/fixtures/unknown-types.svr",
        env!("CARGO_MANIFEST_DIR")
    );
    for args in [
        vec!["check", source.as_str()],
        vec!["run", source.as_str()],
        vec!["build", source.as_str()],
        vec!["build", "--emit", "js", source.as_str()],
    ] {
        let Some(output) = output_or_skip(svr().args(&args)) else {
            return;
        };
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("E3017"));
    }
    let Some(output) = output_or_skip(svr().args(["check", "--format=json", &source])) else {
        return;
    };
    assert_eq!(output.status.code(), Some(1));
    assert_json_report(
        &output,
        r#"
        assert.equal(report.success, false);
        assert.equal(report.diagnostics.length, 3);
        const bytes = require('node:fs').readFileSync(report.target);
        const snippets = report.diagnostics.map(d => {
            assert.equal(d.code, 'E3017');
            assert.equal(d.location.file, report.target);
            assert.ok(d.message.includes('Strng'));
            return bytes.subarray(d.location.start, d.location.end).toString('utf8');
        });
        assert.deepEqual(snippets, ['Strng', 'Strng', 'Strng']);
        const expected = [...bytes.toString('utf8').matchAll(/Strng/g)].map(m => m.index);
        assert.deepEqual(report.diagnostics.map(d => d.location.start), expected);
    "#,
    );
}

#[test]
fn check_json_reports_errors_inside_excess_arguments() {
    let source = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/excess-argument.svr"
    );
    let Some(output) = output_or_skip(svr().args(["check", "--format", "json", source])) else {
        return;
    };
    assert_eq!(output.status.code(), Some(1));
    assert_json_report(
        &output,
        r#"
        assert.equal(report.success, false);
        assert.deepEqual(report.diagnostics.map(d => d.code), ['E3006', 'E3001']);
        const diagnostic = report.diagnostics[1];
        assert.equal(diagnostic.location.file, report.target);
        const bytes = require('node:fs').readFileSync(report.target);
        assert.equal(bytes.subarray(diagnostic.location.start, diagnostic.location.end).toString(), 'missing');
    "#,
    );
}

#[test]
fn check_json_reports_service_contract_declarations() {
    let project = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/service-contract"
    );
    let Some(output) = output_or_skip(svr().args(["check", "--format", "json", project])) else {
        return;
    };
    assert_eq!(output.status.code(), Some(1));
    assert_json_report(
        &output,
        r#"
        assert.equal(report.success, false);
        assert.equal(report.kind, 'project');
        assert.equal(report.diagnostics.length, 2);
        const duplicate = report.diagnostics.find(d => d.code === 'E4025');
        const unclosed = report.diagnostics.find(d => d.code === 'E4026');
        assert.ok(duplicate);
        assert.ok(unclosed);
        for (const diagnostic of [duplicate, unclosed]) {
            assert.ok(diagnostic.location.file.replaceAll('\\', '/').endsWith('/main.svr'));
        }
        const bytes = require('node:fs').readFileSync(duplicate.location.file);
        assert.equal(bytes.subarray(duplicate.location.start, duplicate.location.end).toString().trim(), 'fn send(other: Text) -> Text');
        assert.equal(bytes.subarray(unclosed.location.start, unclosed.location.end).toString(), 'service mail');
    "#,
    );
}

#[test]
fn project_service_parameters_require_annotations_in_json() {
    let project = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/service-untyped"
    );
    let Some(output) = output_or_skip(svr().args(["check", "--format=json", project])) else {
        return;
    };
    assert_eq!(output.status.code(), Some(1));
    assert_json_report(
        &output,
        r#"
        assert.equal(report.success, false);
        assert.equal(report.diagnostics.length, 1);
        const error = report.diagnostics[0];
        assert.equal(error.code, 'E4097');
        assert.ok(error.message.includes('message'));
        const bytes = require('node:fs').readFileSync(error.location.file);
        assert.equal(bytes.subarray(error.location.start, error.location.end).toString().trim(), 'fn send(message);');
    "#,
    );
}

#[test]
fn check_json_exposes_unresolved_service_metadata() {
    let project = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/service-metadata"
    );
    let Some(output) = output_or_skip(svr().args(["check", "--format", "json", project])) else {
        return;
    };
    assert!(output.status.success());
    assert_json_report(
        &output,
        r#"
        assert.equal(report.kind, 'project');
        assert.equal(report.success, true);
        assert.deepEqual(report.diagnostics, []);
        assert.equal(report.service_operations.length, 2);
        const [send, ping] = report.service_operations;
        assert.equal(send.service, 'mail');
        assert.equal(send.name, 'send');
        assert.deepEqual(send.parameters, [
            {name: 'value', annotation: 'Result<Text, Error>'},
            {name: 'context', annotation: 'Int'}
        ]);
        assert.equal(send.return_annotation, 'Receipt');
        assert.equal(send.has_body, false);
        assert.equal(ping.return_annotation, null);
        assert.equal(ping.has_body, true);
        assert.deepEqual(ping.parameters, []);
        const bytes = require('node:fs').readFileSync(send.location.file);
        assert.equal(bytes.subarray(send.location.start, send.location.end).toString().trim(),
            'fn send(value: Result<Text, Error>, context: Int) -> Receipt;');
    "#,
    );
}

#[test]
fn service_call_cli_reports_errors_and_incomplete_coverage() {
    let root = env!("CARGO_MANIFEST_DIR");
    for (relative, exit, codes, complete) in [
        ("examples/service-contracts", 0, "[]", true),
        ("tests/fixtures/application-callable-collisions", 1, "['E4127', 'E4133']", true),
        ("tests/fixtures/application-unresolved-members", 1, "['E4133', 'E4133']", true),
        ("tests/fixtures/application-scalar-member-calls", 1, "['E4133', 'E4133', 'E4133']", true),
        ("examples/application-record-imports", 0, "[]", true),
        ("tests/fixtures/application-constructor-errors", 1, "['E4139', 'E4136']", true),
        ("tests/fixtures/application-field-errors", 1, "['E4133', 'E4138', 'E4137']", true),
        ("examples/application-record-contracts", 0, "[]", true),
        ("tests/fixtures/application-record-mismatch", 1, "['E4119']", true),
        ("tests/fixtures/application-integer-bounds", 1, "['E3012', 'E3012', 'E3012']", true),
        ("examples/application-aliases", 0, "[]", true),
        ("tests/fixtures/application-alias-errors", 1, "['E4129', 'E4120', 'E4119']", true),
        ("tests/fixtures/application-discarded-expressions", 1, "['E4137', 'E4137', 'E4137']", true),
        ("tests/fixtures/application-local-types", 1, "['E4135', 'E4136', 'E4136']", true),
        ("tests/fixtures/application-logical-guards", 1, "['E4125', 'E4125', 'E4121', 'E4121']", true),
        ("examples/application-imports", 0, "[]", true),
        ("tests/fixtures/application-import-errors", 1, "['E4133', 'E4129']", true),
        ("tests/fixtures/ordinary-unknown-annotations", 1, "['E4134', 'E4134']", true),
        ("tests/fixtures/task-unknown-annotations", 1, "['E4134', 'E4131']", true),
        ("tests/fixtures/task-return-contracts", 1, "['E4131', 'E4131', 'E4131', 'E4131']", true),
        ("tests/fixtures/unresolved-ordinary-calls", 1, "['E4133', 'E4133', 'E4133']", true),
        ("tests/fixtures/application-stdlib", 1, "['E4129', 'E4128', 'E4130']", true),
        ("examples/application-functions", 0, "[]", true),
        ("tests/fixtures/ordinary-function-types", 1, "['E4128', 'E4129', 'E4130', 'E4131', 'E4119']", true),
        ("tests/fixtures/service-unresolved-types", 1, "['E4126', 'E4125', 'E4125']", true),
        ("tests/fixtures/service-control-flow", 1, "['E4123', 'E4123', 'E4124', 'E4124']", true),
        (
            "tests/fixtures/service-argument-types",
            1,
            "['E4123', 'E4122', 'E4119', 'E4119', 'E4119', 'E4119', 'E4119', 'E4119', 'E4119', 'E4120', 'E4119', 'E4121', 'E4119', 'E4119', 'E4119', 'E4119', 'E4119']",
            true,
        ),
        (
            "tests/fixtures/service-metadata",
            1,
            "['E4133', 'E4131', 'E4130', 'E4135', 'E4117', 'E4117']",
            true,
        ),
        ("tests/fixtures/service-body-coverage", 1, "['E4093']", true),
        (
            "tests/fixtures/service-call-errors",
            1,
            "['E4130', 'E4130', 'E4094', 'E4093']",
            true,
        ),
        ("examples/fielddesk", 1, "null", false),
    ] {
        let project = format!("{root}/{relative}");
        let Some(output) =
            output_or_skip(svr().args(["check", "--service-calls", "--format=json", &project]))
        else {
            return;
        };
        assert_eq!(output.status.code(), Some(exit));
        assert_json_report(
            &output,
            &format!(
                r#"
            assert.equal(report.success, {success});
            assert.equal(report.service_coverage.complete, {complete});
            assert.ok(report.service_coverage.files.length > 0);
            assert.ok(Array.isArray(report.member_calls));
            assert.ok(Array.isArray(report.ordinary_calls));
            assert.ok(Array.isArray(report.local_bindings));
            if ('{relative}' === 'examples/application-record-imports') {{
                const created = report.local_bindings.find(b => b.name === 'created');
                assert.equal(created.resolved_type, null);
                assert.equal(created.resolved_type_info.kind, 'record');
                assert.deepEqual(created.declared_type_info, created.resolved_type_info);
                const save = report.member_calls.find(c => c.operation === 'save');
                assert.deepEqual(save.argument_type_info[0], created.resolved_type_info);
                const x = report.local_bindings.find(b => b.name === 'x');
                assert.deepEqual(x.resolved_type_info, {{kind:'scalar', name:'Float'}});
                const wrapped = report.local_bindings.find(b => b.name === 'wrapped_x');
                assert.deepEqual(wrapped.resolved_type_info, {{kind:'scalar', name:'Float'}});
                const made = report.local_bindings.find(b => b.name === 'made');
                assert.deepEqual(made.resolved_type_info, created.resolved_type_info);
                const make = report.ordinary_calls.find(c => c.callee === 'app::factory::make');
                assert.deepEqual(make.declared_return_type_info, made.resolved_type_info);
                assert.ok(make.declaration_file.endsWith('factory.svr'));
            }}
            if ('{relative}' === 'tests/fixtures/application-integer-bounds') {{
                const source = require('fs').readFileSync(report.diagnostics[0].location.file);
                assert.deepEqual(report.diagnostics.map(d => source.subarray(d.location.start, d.location.end).toString()), ['9223372036854775808', '18446744073709551616', '999999999999999999999999']);
            }}
            if ('{relative}' === 'examples/application-aliases') {{
                const call = report.ordinary_calls.find(c => c.callee === 'app::pricing::price');
                assert.equal(call.declared_return_type, 'Float');
                assert.equal(call.arguments[0].type, 'Int');
                assert.ok(call.declaration_file.endsWith('pricing.svr'));
                const value = report.local_bindings.find(b => b.name === 'value');
                assert.equal(value.declared_type, 'Float');
                assert.equal(value.initializer_type, 'Float');
                assert.equal(value.resolved_type, 'Float');
            }}
            if ('{relative}' === 'tests/fixtures/application-discarded-expressions') {{
                const source = require('fs').readFileSync(report.diagnostics[0].location.file);
                assert.deepEqual(report.diagnostics.map(d => source.subarray(d.location.start, d.location.end).toString()), ['missing', 'absent + 1', 'unknown.field']);
            }}
            if ('{relative}' === 'tests/fixtures/application-local-types') {{
                assert.deepEqual(report.local_bindings.map(b => b.resolved_type), [null, null, null, 'Float']);
                for (const error of report.diagnostics) {{
                    assert.ok(error.location.file.endsWith('main.svr'));
                    assert.ok(error.location.end > error.location.start);
                }}
            }}
            if ('{relative}' === 'tests/fixtures/task-return-contracts') {{
                const source = require('fs').readFileSync(report.diagnostics[0].location.file);
                const ranges = report.diagnostics.map(d => source.subarray(d.location.start, d.location.end).toString());
                assert.ok(ranges[0].startsWith('task missing() -> Int'));
                assert.deepEqual(ranges.slice(1), ['"wrong"', '1', 'missing_value']);
                assert.ok(report.diagnostics.every(d => d.location.file.endsWith('main.svr')));
            }}
            for (const call of report.ordinary_calls) {{
                assert.ok(report.service_coverage.files.some(f => f.file === call.location.file && f.inspected));
                assert.ok(['builtin', 'ordinary', 'unresolved'].includes(call.kind));
                assert.ok(Array.isArray(call.arguments));
            }}
            if ('{relative}' === 'tests/fixtures/unresolved-ordinary-calls') {{
                assert.deepEqual(report.ordinary_calls.map(c => c.kind), ['unresolved', 'unresolved', 'unresolved']);
                assert.deepEqual(report.ordinary_calls.map(c => c.callee), ['missing', 'std::missing', 'helper']);
            }}
            if ('{relative}' === 'examples/application-imports') {{
                const imported = report.ordinary_calls.find(c => c.callee === 'app::helpers::format');
                assert.equal(imported.kind, 'ordinary');
                assert.equal(imported.declared_return_type, 'String');
                assert.ok(imported.declaration_file.endsWith('helpers.svr'));
                assert.ok(imported.location.file.endsWith('main.svr'));
            }}
            for (const call of report.member_calls) {{
                assert.ok(report.service_coverage.files.some(f => f.file === call.location.file && f.inspected));
                assert.equal(typeof call.operation, 'string');
            }}
            if ({complete} && '{relative}' !== 'tests/fixtures/task-return-contracts')
                assert.ok(report.member_calls.some(c => c.receiver.kind === 'service'));
              if ('{relative}' === 'tests/fixtures/service-body-coverage') {{
                const call = report.member_calls.find(c => c.function === 'mail.send');
                assert.ok(call);
                assert.equal(call.is_task, false);
                assert.equal(call.operation, 'missing');
                assert.equal(call.receiver.kind, 'service');
                const bytes = require('node:fs').readFileSync(call.location.file);
                assert.equal(bytes.subarray(call.location.start, call.location.end).toString(), 'mail.missing');
                  assert.deepEqual(report.diagnostics[0].location, call.location);
              }}
              if ('{relative}' === 'tests/fixtures/service-metadata') {{
                  assert.equal(report.diagnostics[0].code, 'E4133');
                  assert.equal(report.diagnostics[1].code, 'E4131');
                  assert.equal(report.diagnostics[2].code, 'E4130');
                  assert.equal(report.diagnostics[3].code, 'E4135');
                  for (const error of report.diagnostics.slice(4)) {{
                      assert.equal(error.code, 'E4117');
                      assert.equal(error.location.line, 1);
                      const bytes = require('node:fs').readFileSync(error.location.file);
                      assert.equal(bytes.subarray(error.location.start, error.location.end).toString().trim(),
                          'fn send(value: Result<Text, Error>, context: Int) -> Receipt;');
                  }}
              }}
            if ({codes} !== null) assert.deepEqual(report.diagnostics.map(d => d.code), {codes});
            else {{
                assert.ok(report.diagnostics.some(d => d.code === 'E4096'));
                assert.ok(report.service_coverage.files.some(f => !f.inspected && typeof f.reason === 'string'));
            }}
        "#,
                success = exit == 0
            ),
        );
        let Some(human) = output_or_skip(svr().args(["check", "--service-calls", &project])) else {
            return;
        };
        assert_eq!(human.status.code(), Some(exit));
        assert!(String::from_utf8_lossy(&human.stdout).contains("file(s) covered"));
    }
}

#[test]
fn service_call_option_rejects_source_files_and_duplicate_flags() {
    let source = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/hello-world/main.svr");
    for args in [
        vec!["check", "--service-calls", source],
        vec!["check", "--service-calls", "--service-calls", source],
    ] {
        let Some(output) = output_or_skip(svr().args(args)) else {
            return;
        };
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn private_module_access_is_rejected_by_source_commands() {
    let source = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/private-access.svr"
    );
    for args in [
        vec!["check", source],
        vec!["run", source],
        vec!["build", source],
        vec!["build", "--emit", "js", source],
    ] {
        let Some(output) = output_or_skip(svr().args(args)) else {
            return;
        };
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("E3004"));
    }
    let Some(output) = output_or_skip(svr().args(["check", "--format", "json", source])) else {
        return;
    };
    assert_eq!(output.status.code(), Some(1));
    assert_json_report(
        &output,
        r#"
        assert.equal(report.success, false);
        assert.equal(report.diagnostics.length, 1);
        const diagnostic = report.diagnostics[0];
        assert.equal(diagnostic.code, 'E3004');
        assert.equal(diagnostic.location.file, report.target);
        const bytes = require('node:fs').readFileSync(report.target);
        assert.equal(bytes.subarray(diagnostic.location.start, diagnostic.location.end).toString(), 'math::helper');
    "#,
    );
}

#[test]
fn module_example_executes_private_helper() {
    let source = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/modules/main.svr");
    let Some(output) = output_or_skip(svr().args(["run", source])) else {
        return;
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "42");
    for args in [
        vec!["check", source],
        vec!["build", source],
        vec!["build", "--emit", "js", source],
    ] {
        let Some(output) = output_or_skip(svr().args(args)) else {
            return;
        };
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn records_example_checks_and_executes() {
    let source = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/records/main.svr");
    let Some(output) = output_or_skip(svr().args(["run", source])) else {
        return;
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .collect::<Vec<_>>(),
        [
            "3",
            "4",
            "Location { name: Studio, point: Point { x: 3, y: 4 } }"
        ]
    );
    for args in [
        vec!["check", source],
        vec!["build", source],
        vec!["build", "--emit", "js", source],
    ] {
        let Some(output) = output_or_skip(svr().args(args)) else {
            return;
        };
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn source_commands_reject_builtin_collisions() {
    let source = format!(
        "{}/tests/fixtures/builtin-collision.svr",
        env!("CARGO_MANIFEST_DIR")
    );
    for args in [
        vec!["check", source.as_str()],
        vec!["run", source.as_str()],
        vec!["build", source.as_str()],
        vec!["build", "--emit", "js", source.as_str()],
    ] {
        let Some(output) = output_or_skip(svr().args(&args)) else {
            return;
        };
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("E3016"));
    }
    let Some(output) = output_or_skip(svr().args(["check", "--format=json", &source])) else {
        return;
    };
    assert_eq!(output.status.code(), Some(1));
    assert_json_report(
        &output,
        r#"
        const conflicts = report.diagnostics.filter(d => d.code === 'E3016');
        assert.equal(conflicts.length, 2);
        assert.deepEqual(conflicts.map(d => d.location.line), [0, 2]);
        for (const d of conflicts) assert.equal(d.location.file, report.target);
    "#,
    );
}

#[test]
fn source_commands_reject_duplicate_private_functions() {
    let source = format!(
        "{}/tests/fixtures/duplicate-private.svr",
        env!("CARGO_MANIFEST_DIR")
    );
    for args in [
        vec!["check", source.as_str()],
        vec!["run", source.as_str()],
        vec!["build", source.as_str()],
        vec!["build", "--emit", "js", source.as_str()],
    ] {
        let Some(output) = output_or_skip(svr().args(&args)) else {
            return;
        };
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("E3008"));
    }
}

#[test]
fn source_commands_reject_function_values() {
    let source = format!(
        "{}/tests/fixtures/function-value.svr",
        env!("CARGO_MANIFEST_DIR")
    );
    for args in [
        vec!["check", source.as_str()],
        vec!["run", source.as_str()],
        vec!["build", source.as_str()],
        vec!["build", "--emit", "js", source.as_str()],
    ] {
        let Some(output) = output_or_skip(svr().args(&args)) else {
            return;
        };
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("E3015"));
    }
}

#[test]
fn source_commands_reject_mistyped_concatenation() {
    let source = format!(
        "{}/tests/fixtures/invalid-concatenation.svr",
        env!("CARGO_MANIFEST_DIR")
    );
    for args in [
        vec!["check", source.as_str()],
        vec!["run", source.as_str()],
        vec!["build", source.as_str()],
        vec!["build", "--emit", "js", source.as_str()],
    ] {
        let Some(output) = output_or_skip(svr().args(&args)) else {
            return;
        };
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("E3002"));
    }
}

#[test]
fn version_command_prints_canonical_version() {
    let Some(output) = output_or_skip(svr().arg("--version")) else {
        return;
    };
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "svr 1.0.0");
}

#[test]
fn command_exit_codes_and_help_terminator_work_on_minimum_rust() {
    for (args, expected) in [
        (&["build"][..], 2),
        (&["wat"][..], 2),
        (&["check", "--format=json", "--", "--help"][..], 1),
        (&["check", "--format=json", "--", "-h"][..], 1),
    ] {
        let Some(output) = output_or_skip(svr().args(args)) else {
            return;
        };
        assert_eq!(output.status.code(), Some(expected), "{args:?}");
    }
}

#[test]
fn help_command_describes_m11() {
    let Some(output) = output_or_skip(svr().arg("--help")) else {
        return;
    };
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("Usage:"));
    assert!(help.contains("build"));
    assert!(help.contains("M12"));
}

#[test]
fn run_command_executes_source_file() {
    let source = format!(
        "{}/examples/hello-world/main.svr",
        env!("CARGO_MANIFEST_DIR")
    );
    let Some(output) = output_or_skip(svr().args(["run", source.as_str()])) else {
        return;
    };
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "Hello, Sovra!"
    );
}

#[test]
fn build_command_emits_inspectable_ir() {
    let source = format!(
        "{}/examples/hello-world/main.svr",
        env!("CARGO_MANIFEST_DIR")
    );
    let Some(output) = output_or_skip(svr().args(["build", source.as_str()])) else {
        return;
    };
    assert!(output.status.success());
    let ir = String::from_utf8_lossy(&output.stdout);
    assert!(ir.contains("function main:"));
    assert!(ir.contains("call print 1"));
}

#[test]
fn build_command_emits_javascript_backend() {
    let source = format!(
        "{}/examples/hello-world/main.svr",
        env!("CARGO_MANIFEST_DIR")
    );
    let Some(output) = output_or_skip(svr().args(["build", "--emit", "js", source.as_str()]))
    else {
        return;
    };
    assert!(output.status.success());
    let javascript = String::from_utf8_lossy(&output.stdout);
    assert!(javascript.contains("\"use strict\";"));
    assert!(javascript.contains("svrFunctions[\"main\"] = svr_fn_0;"));
    assert!(javascript.contains("svrWriteLine(args[0])"));
}

#[test]
fn build_help_describes_current_usage() {
    let Some(output) = output_or_skip(svr().args(["build", "--help"])) else {
        return;
    };
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "Usage: svr build [--locked] [--emit ir|js] <source.svr|package-directory>\n       svr build --emit exe --output PATH <source.svr|package-directory>\n  exe: host-native output; Rust/Cargo and a host linker are build-time requirements."
    );
}

#[test]
fn check_help_describes_current_usage() {
    let Some(output) = output_or_skip(svr().args(["check", "--help"])) else {
        return;
    };
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "Usage: svr check [--format human|json] [--service-calls] [--symbols] <source.svr|project-directory>\n       svr check --format json --overlays-stdin <package-directory>\n  --service-calls  Experimental project-only contract checks; incomplete coverage fails.\n  --symbols        Opt-in checked symbol/type evidence for successful standalone JSON checks.\n  --overlays-stdin Framed unsaved package buffers; requires JSON, never executes or writes source.\n  --package-context Resolve canonical entry paths only; JSON, no source validation.\n  --locked         Verify sovra.lock; requires --executable-package.\n  --executable-package Compile all package sources and linked IR; never execute them."
    );
}

#[test]
fn check_command_validates_source_file() {
    let source = format!(
        "{}/examples/hello-world/main.svr",
        env!("CARGO_MANIFEST_DIR")
    );
    let Some(output) = output_or_skip(svr().args(["check", source.as_str()])) else {
        return;
    };
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("checked source"));
}

#[test]
fn source_commands_reject_invalid_module_bodies() {
    let source = format!(
        "{}/tests/fixtures/invalid-module.svr",
        env!("CARGO_MANIFEST_DIR")
    );
    for arguments in [
        vec!["check", source.as_str()],
        vec!["run", source.as_str()],
        vec!["build", source.as_str()],
        vec!["build", "--emit", "js", source.as_str()],
    ] {
        let Some(output) = output_or_skip(svr().args(&arguments)) else {
            return;
        };
        assert_eq!(output.status.code(), Some(1), "{arguments:?}");
        assert!(output.stdout.is_empty(), "{arguments:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("E3001"), "{arguments:?}: {stderr}");
        assert!(stderr.contains("undefined variable `missing`"));
    }
}

#[test]
fn source_commands_require_parameter_annotations() {
    let source = format!(
        "{}/tests/fixtures/untyped-parameter.svr",
        env!("CARGO_MANIFEST_DIR")
    );
    for arguments in [
        vec!["check", source.as_str()],
        vec!["run", source.as_str()],
        vec!["build", source.as_str()],
        vec!["build", "--emit", "js", source.as_str()],
    ] {
        let Some(output) = output_or_skip(svr().args(&arguments)) else {
            return;
        };
        assert_eq!(output.status.code(), Some(1), "{arguments:?}");
        assert!(output.stdout.is_empty(), "{arguments:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("E3014"), "{arguments:?}: {stderr}");
        assert!(stderr.contains("write `value: Type`"));
    }
}

#[test]
fn check_command_validates_project_directory() {
    let project = format!("{}/examples/fielddesk", env!("CARGO_MANIFEST_DIR"));
    let Some(output) = output_or_skip(svr().args(["check", project.as_str()])) else {
        return;
    };
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("checked project `fielddesk`"));
    assert!(stdout.contains("2 service(s), 4 model(s), 3 route(s), 2 page(s)"));
    assert!(stdout.contains("1 scheduled task(s), 3 auth policy(ies), auth auth.session"));
    assert!(stdout.contains("entry"));
}

#[test]
fn run_command_rejects_non_sovra_paths() {
    let Some(output) = output_or_skip(svr().args(["run", "README.md"])) else {
        return;
    };
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains(".svr extension"));
}

#[test]
fn all_reserved_commands_are_recognized() {
    for command in [
        "new", "init", "run", "build", "test", "check", "fmt", "repl", "install", "update", "doc",
    ] {
        let Some(output) = output_or_skip(svr().arg(command)) else {
            return;
        };
        if matches!(command, "new" | "run" | "build" | "check" | "test" | "fmt") {
            assert_eq!(output.status.code(), Some(2));
        } else {
            assert_ne!(output.status.code(), Some(2));
        }
    }
}

#[test]
fn new_creates_a_checked_and_runnable_project_without_overwriting() {
    let Some(missing) = output_or_skip(svr().arg("new")) else {
        return;
    };
    assert_eq!(missing.status.code(), Some(2));
    let help = svr().args(["new", "--help"]).output().unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("svr new <path>"));

    let root = std::env::temp_dir().join(format!(
        "sovra-new-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let project = root.join("hello");
    let invalid = root.join("2bad");
    let output = svr().arg("new").arg(&invalid).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(!invalid.exists());
    let output = svr().arg("new").arg(&project).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let main = std::fs::read_to_string(project.join("main.svr")).unwrap();
    let manifest = std::fs::read_to_string(project.join("sovra.toml")).unwrap();
    assert_eq!(
        main,
        include_str!("../toolchain/templates/console/main.svr")
    );
    assert!(manifest.contains("name = \"hello\""));
    let existing = svr().arg("new").arg(&project).output().unwrap();
    assert_eq!(existing.status.code(), Some(1));
    assert_eq!(
        std::fs::read_to_string(project.join("main.svr")).unwrap(),
        main
    );
    let extra = svr()
        .arg("new")
        .arg(&project)
        .arg("extra")
        .output()
        .unwrap();
    assert_eq!(extra.status.code(), Some(2));
    let check = svr()
        .arg("check")
        .arg(project.join("main.svr"))
        .output()
        .unwrap();
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let run = svr().arg("run").arg(&project).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "Hello, Sovra!");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn unknown_command_has_helpful_error() {
    let Some(output) = output_or_skip(svr().arg("unknown")) else {
        return;
    };
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("svr --help"));
}

#[test]
fn test_command_reports_template_and_rejects_malformed_invocation() {
    let source = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/toolchain/templates/console/main.svr"
    );
    let Some(output) = output_or_skip(svr().args(["test", "--format", "json", source])) else {
        return;
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json = String::from_utf8_lossy(&output.stdout);
    assert!(json.starts_with("{\"schema_version\":1"));
    assert!(json.contains("\"success\":true"));
    assert!(json.contains("\"name\":\"test_greeting\""));
    assert!(output.stderr.is_empty());
    for args in [
        vec!["test"],
        vec!["test", "--format", "yaml", source],
        vec!["test", source, source],
    ] {
        let Some(output) = output_or_skip(svr().args(args)) else {
            return;
        };
        assert_eq!(output.status.code(), Some(2));
    }
}

#[test]
fn native_build_rejects_invalid_output_options() {
    for args in [
        vec!["build", "--emit", "exe", "examples/hello-world/main.svr"],
        vec![
            "build",
            "--output",
            "app.exe",
            "examples/hello-world/main.svr",
        ],
        vec![
            "build",
            "--emit",
            "js",
            "--output",
            "app.exe",
            "examples/hello-world/main.svr",
        ],
        vec![
            "build",
            "--emit",
            "exe",
            "--output=",
            "examples/hello-world/main.svr",
        ],
        vec![
            "build",
            "--emit",
            "exe",
            "--output",
            "a.exe",
            "--output",
            "b.exe",
            "examples/hello-world/main.svr",
        ],
    ] {
        let Some(output) = output_or_skip(svr().args(args)) else {
            return;
        };
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}
