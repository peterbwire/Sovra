//! The `svr` command-line entry point.

use std::env;
use std::process::ExitCode;

use sovra::cli;

fn main() -> ExitCode {
    cli::run(env::args().skip(1))
}
