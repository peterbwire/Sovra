//! Internal N1 proof: `cargo run --example native_builder -- SOURCE OUTPUT_DIR`.
//! Requires a Rust toolchain on the build machine; the produced binary does not.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use sovra::compiler::{
    ir, native_image, parser::Parser, project::packages, semantic::SemanticAnalyzer,
};

fn main() {
    if let Err(error) = build() {
        eprintln!("native builder: {error}");
        std::process::exit(1);
    }
}

fn build() -> Result<(), String> {
    let mut args = env::args_os().skip(1);
    let source = PathBuf::from(
        args.next()
            .ok_or("expected source path and output directory")?,
    );
    let output = PathBuf::from(args.next().ok_or("expected output directory")?);
    if args.next().is_some() {
        return Err("expected exactly two arguments".into());
    }
    let program = if source.is_dir() {
        packages::compile(&source).map_err(diagnostics)?
    } else {
        let text = fs::read_to_string(&source).map_err(|e| format!("{}: {e}", source.display()))?;
        let parsed = Parser::new().parse_source(&text).map_err(diagnostics)?;
        let typed = SemanticAnalyzer::new()
            .analyze(&parsed)
            .map_err(diagnostics)?;
        ir::lower(&typed)
    };
    let entry = program
        .functions
        .iter()
        .find(|function| function.name == "main")
        .ok_or("entry function `main` was not found")?;
    if !entry.parameters.is_empty() {
        return Err("entry function `main` cannot declare parameters".into());
    }
    let image = native_image::encode(&program)?;
    native_image::decode(&image)?;
    let stage = reserve_output(&output)?;
    let result = build_in_stage(&stage, &output, &image);
    if let Err(error) = result {
        fs::remove_dir_all(&stage).map_err(|cleanup| {
            format!("{error}; cannot remove build staging directory: {cleanup}")
        })?;
        return Err(error);
    }
    let executable = output.join("target/release").join(if cfg!(windows) {
        "sovra-native-proof.exe"
    } else {
        "sovra-native-proof"
    });
    println!("{}", executable.display());
    Ok(())
}

fn reserve_output(output: &Path) -> Result<PathBuf, String> {
    match fs::symlink_metadata(output) {
        Ok(_) => return Err(format!("output already exists: {}", output.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("{}: {error}", output.display())),
    }
    let parent = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    for attempt in 0..32 {
        let stage = parent.join(format!(
            ".sovra-native-stage-{}-{attempt}",
            std::process::id()
        ));
        match fs::create_dir(&stage) {
            Ok(()) => return Ok(stage),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("{}: {error}", stage.display())),
        }
    }
    Err("cannot reserve a native build staging directory".into())
}

fn build_in_stage(stage: &Path, output: &Path, image: &[u8]) -> Result<(), String> {
    fs::create_dir(stage.join("src")).map_err(|e| e.to_string())?;
    fs::write(stage.join("program.svir"), image).map_err(|e| e.to_string())?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = format!(
        "[package]\nname = \"sovra-native-proof\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[dependencies]\nsovra = {{ path = {} }}\n",
        toml_string(root)
    );
    fs::write(stage.join("Cargo.toml"), manifest).map_err(|e| e.to_string())?;
    fs::write(stage.join("src/main.rs"), RUNNER).map_err(|e| e.to_string())?;
    let status = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--manifest-path")
        .arg(stage.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(stage.join("target"))
        .status()
        .map_err(|e| format!("cannot launch Cargo: {e}"))?;
    if !status.success() {
        return Err(format!("Cargo build failed: {status}"));
    }
    let executable = stage.join("target/release").join(if cfg!(windows) {
        "sovra-native-proof.exe"
    } else {
        "sovra-native-proof"
    });
    if !executable.is_file() {
        return Err(format!(
            "Cargo produced no executable: {}",
            executable.display()
        ));
    }
    if fs::symlink_metadata(output).is_ok() {
        return Err(format!("output already exists: {}", output.display()));
    }
    fs::rename(stage, output).map_err(|error| format!("{}: {error}", output.display()))?;
    Ok(())
}

fn diagnostics(errors: sovra::compiler::diagnostics::Diagnostics) -> String {
    errors
        .items
        .into_iter()
        .map(|d| format!("error[{}]: {}", d.code, d.message))
        .collect::<Vec<_>>()
        .join("\n")
}

fn toml_string(path: &Path) -> String {
    format!("{:?}", path.to_string_lossy().replace('\\', "/"))
}

const RUNNER: &str = r#"use std::{env, process::ExitCode};
use sovra::compiler::{interpreter::{run_with_host_status, ProcessHost}, native_image};

fn main() -> ExitCode {
    let program = match native_image::decode(include_bytes!("../program.svir")) {
        Ok(program) => program,
        Err(error) => { eprintln!("svr: invalid embedded program: {error}"); return ExitCode::from(1); }
    };
    let mut host = ProcessHost::new(env::args().skip(1).collect());
    match run_with_host_status(&program, &mut host) {
        Ok(outcome) => ExitCode::from(outcome.exit_code),
        Err(error) => { eprintln!("svr: runtime error: {error}"); ExitCode::from(1) }
    }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn output_reservation_refuses_existing_projects_without_changes() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "sovra-native-reserve-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        let existing = root.join("existing");
        fs::create_dir(&existing).unwrap();
        fs::write(existing.join("sentinel"), "keep").unwrap();
        assert!(reserve_output(&existing)
            .unwrap_err()
            .contains("already exists"));
        assert_eq!(
            fs::read_to_string(existing.join("sentinel")).unwrap(),
            "keep"
        );

        let output = root.join("new");
        let stage = reserve_output(&output).unwrap();
        assert!(stage.is_dir());
        assert!(!output.exists());
        fs::remove_dir_all(&root).unwrap();
    }
}
