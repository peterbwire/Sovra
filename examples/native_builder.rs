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
    let image = native_image::encode(&program)?;
    fs::create_dir_all(output.join("src")).map_err(|e| e.to_string())?;
    fs::write(output.join("program.svir"), image).map_err(|e| e.to_string())?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = format!(
        "[package]\nname = \"sovra-native-proof\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[dependencies]\nsovra = {{ path = {} }}\n",
        toml_string(root)
    );
    fs::write(output.join("Cargo.toml"), manifest).map_err(|e| e.to_string())?;
    fs::write(output.join("src/main.rs"), RUNNER).map_err(|e| e.to_string())?;
    let status = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--manifest-path")
        .arg(output.join("Cargo.toml"))
        .status()
        .map_err(|e| format!("cannot launch Cargo: {e}"))?;
    if !status.success() {
        return Err(format!("Cargo build failed: {status}"));
    }
    let executable = output.join("target/release").join(if cfg!(windows) {
        "sovra-native-proof.exe"
    } else {
        "sovra-native-proof"
    });
    println!("{}", executable.display());
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
