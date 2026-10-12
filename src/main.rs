//! The `svr` command-line entry point.

use std::env;
use std::process::ExitCode;

use sovra::cli;

#[cfg(not(windows))]
fn main() -> ExitCode {
    cli::run(env::args().skip(1))
}

#[cfg(windows)]
fn main() -> ExitCode {
    // The Windows main thread's default stack is too small for the supported
    // structural nesting limit on MSVC. Reserve a larger stack for CLI work.
    let args: Vec<String> = env::args().skip(1).collect();
    match std::thread::Builder::new()
        .name("svr-compiler".to_owned())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || cli::run(args))
    {
        Ok(worker) => worker
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic)),
        Err(error) => {
            eprintln!("svr: unable to start compiler: {error}");
            ExitCode::from(1)
        }
    }
}
