use std::{env, process::ExitCode};
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
