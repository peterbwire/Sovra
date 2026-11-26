use std::{env, process::ExitCode};
use sovra::compiler::{interpreter::{run_with_host_status, ProcessHost}, native_image};

fn main() -> ExitCode {
    let image = include_bytes!("../program.svir");
    if sovra::compiler::v2::is_generic_package_native_image(image) {
        return run_generic_package(image);
    }
    if sovra::compiler::v2::is_generic_native_image(image) {
        return run_generic(image);
    }
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

fn run_generic(image: &[u8]) -> ExitCode {
    if env::args_os().nth(1).is_some() {
        eprintln!("svr: generic native programs do not accept arguments");
        return ExitCode::from(1);
    }
    match sovra::compiler::v2::execute_generic_native_image(image) {
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("svr: {}", error.chars().take(1024).flat_map(char::escape_default).collect::<String>());
            ExitCode::from(1)
        }
    }
}

fn run_generic_package(image:&[u8])->ExitCode {
    if image.get(8..12)==Some(2047u32.to_le_bytes().as_slice()) {
        let args:Vec<_>=env::args_os().skip(1).collect();
        if args.is_empty(){return match sovra::compiler::v2::execute_generic_data_package_native_image(image){Ok(_)=>ExitCode::SUCCESS,Err(e)=>{eprintln!("svr: {}",e.chars().take(1024).flat_map(char::escape_default).collect::<String>());ExitCode::from(1)}};}
        if args.len()!=2||args[0]!="--host-policy"{eprintln!("svr: data native launch accepts only --host-policy PATH");return ExitCode::from(1);}
        let result=(||{let host=sovra::compiler::v2::DataHostPolicy::load_policy(std::path::Path::new(&args[1])).map_err(|e|e.message)?;let program=sovra::compiler::v2::decode_generic_data_package_native_image(image)?;host.execute(&program,sovra::compiler::v2::generic_command_runtime_policy()).map_err(|e|format!("{e:?}"))?;Ok::<(),String>(())})();
        return match result{Ok(())=>ExitCode::SUCCESS,Err(e)=>{eprintln!("svr: {}",e.chars().take(1024).flat_map(char::escape_default).collect::<String>());ExitCode::from(1)}};
    }

    if env::args_os().nth(1).is_some() {eprintln!("svr: generic native packages do not accept arguments");return ExitCode::from(1);}
    let execute=if image.get(8..12)==Some(2047u32.to_le_bytes().as_slice()) {sovra::compiler::v2::execute_generic_data_package_native_image}else if image.get(8..12)==Some(1023u32.to_le_bytes().as_slice()) {sovra::compiler::v2::execute_generic_collection_package_native_image}else{sovra::compiler::v2::execute_generic_package_native_image};
    match execute(image) {
        Ok(_)=>ExitCode::SUCCESS,
        Err(error)=>{eprintln!("svr: {}",error.chars().take(1024).flat_map(char::escape_default).collect::<String>());ExitCode::from(1)}
    }
}
