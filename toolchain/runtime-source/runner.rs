use std::{env, process::ExitCode};
use sovra::compiler::{interpreter::{run_with_host_status, ProcessHost}, native_image};

fn main() -> ExitCode {
    let image = include_bytes!("../program.svir");
    if sovra::compiler::v2::is_native_image(image) {
        return run_v2(image);
    }
    let program = match native_image::decode(image) {
        Ok(program) => program,
        Err(error) => { eprintln!("svr: invalid embedded program: {error}"); return ExitCode::from(1); }
    };
    let mut host = ProcessHost::new(env::args().skip(1).collect());
    match run_with_host_status(&program, &mut host) {
        Ok(outcome) => ExitCode::from(outcome.exit_code),
        Err(error) => { eprintln!("svr: runtime error: {error}"); ExitCode::from(1) }
    }
}

fn run_v2(image: &[u8]) -> ExitCode {
    use std::io::Write;
    let mut arguments = Vec::new();
    let mut argument_bytes = 0usize;
    for argument in env::args_os().skip(1) {
        let argument = match argument.into_string() {
            Ok(argument) => argument,
            Err(_) => { eprintln!("svr: v2 arguments must be UTF-8"); return ExitCode::from(1); }
        };
        argument_bytes = match argument_bytes.checked_add(argument.len()).filter(|bytes| *bytes <= 65_536) {
            Some(bytes) if arguments.len() < 1024 => bytes,
            _ => { eprintln!("svr: v2 native argument limit exceeded"); return ExitCode::from(1); }
        };
        arguments.push(argument);
    }
    let outcome = match sovra::compiler::v2::execute_native_image(image, &arguments) {
        Ok(outcome) => outcome,
        Err(error) => {
            eprintln!("svr: {}", error.chars().take(1024).flat_map(char::escape_default).collect::<String>());
            return ExitCode::from(1);
        }
    };
    let mut stdout = std::io::stdout().lock();
    for line in outcome.execution.output {
        if writeln!(stdout, "{line}").is_err() { return ExitCode::from(1); }
    }
    if stdout.flush().is_err() { return ExitCode::from(1); }
    ExitCode::from(outcome.exit_code)
}
