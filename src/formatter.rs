//! Comment-preserving standalone formatter behind accepted ADR 0032.

mod equivalence;
mod files;
mod layout;

use crate::compiler::{
    diagnostics::Diagnostics,
    lexer::{Lexer, TokenKind},
    parser::Parser,
    stdlib::escape_terminal,
};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const HELP: &str = "Usage: svr fmt [--check | --write] [--] SOURCE.svr\n  Default: formatted source on stdout; input unchanged.\n  --check: exit 1 if formatting is needed; no writes.\n  --write: guarded atomic replacement (currently Windows only).\n  Standalone executable syntax only; package imports/application syntax unsupported.";

#[derive(Debug)]
enum Failure {
    Diagnostics(Diagnostics),
    Message(String),
}
impl From<String> for Failure {
    fn from(message: String) -> Self {
        Self::Message(message)
    }
}
impl From<Diagnostics> for Failure {
    fn from(diagnostics: Diagnostics) -> Self {
        Self::Diagnostics(diagnostics)
    }
}

fn format_source(source: &str) -> Result<String, Failure> {
    if source.len() > files::INPUT_LIMIT {
        return Err(Failure::Message("formatter source exceeds 1 MiB".into()));
    }
    let tokens = Lexer::new().tokenize(source)?;
    if tokens.iter().any(|t| t.kind == TokenKind::Keyword("use")) {
        return Err(Failure::Message(
            "formatting package imports is not supported in this first slice".into(),
        ));
    }
    let mut program = Parser::new().parse_tokens(&tokens)?;
    let output = layout::render(source, &tokens, &program)?;
    let formatted_tokens = Lexer::new()
        .tokenize(&output)
        .map_err(|_| Failure::Message("formatter safety check failed: output lexing".into()))?;
    if tokens.len() != formatted_tokens.len()
        || tokens.iter().zip(&formatted_tokens).any(|(a, b)| {
            a.kind != b.kind || source[a.span.start..a.span.end] != output[b.span.start..b.span.end]
        })
    {
        return Err(Failure::Message(
            "formatter safety check failed: token spelling/order changed".into(),
        ));
    }
    if layout::comments(source, &tokens)? != layout::comments(&output, &formatted_tokens)? {
        return Err(Failure::Message(
            "formatter safety check failed: comment content/attachment changed".into(),
        ));
    }
    let mut formatted_program = Parser::new()
        .parse_tokens(&formatted_tokens)
        .map_err(|_| Failure::Message("formatter safety check failed: output parsing".into()))?;
    if layout::render(&output, &formatted_tokens, &formatted_program)? != output {
        return Err(Failure::Message(
            "formatter safety check failed: output is not idempotent".into(),
        ));
    }
    equivalence::normalize(&mut program);
    equivalence::normalize(&mut formatted_program);
    if program != formatted_program {
        return Err(Failure::Message(
            "formatter safety check failed: AST structure changed".into(),
        ));
    }
    Ok(output)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Mode {
    Stdout,
    Check,
    Write,
}
fn options(args: &[String]) -> Result<(Mode, PathBuf), String> {
    let mut mode = Mode::Stdout;
    let mut path = None;
    let mut terminated = false;
    for arg in args {
        if !terminated && arg == "--" {
            terminated = true;
            continue;
        }
        if !terminated && matches!(arg.as_str(), "--check" | "--write") {
            if mode != Mode::Stdout {
                return Err("conflicting or duplicate formatter mode".into());
            }
            mode = if arg == "--check" {
                Mode::Check
            } else {
                Mode::Write
            };
            continue;
        }
        if !terminated && arg.starts_with('-') {
            return Err(format!("unknown formatter option: {arg}"));
        }
        if path.replace(PathBuf::from(arg)).is_some() {
            return Err("formatter expects exactly one source file".into());
        }
    }
    let path = path.ok_or("formatter expects one .svr source file")?;
    if path.extension() != Some(std::ffi::OsStr::new("svr")) || path.is_dir() {
        return Err("formatter expects a .svr file, not a directory".into());
    }
    Ok((mode, path))
}

pub(crate) fn command(args: &[String]) -> ExitCode {
    if args.len() == 1 && matches!(args[0].as_str(), "-h" | "--help") {
        println!("{HELP}");
        return ExitCode::SUCCESS;
    }
    let (mode, path) = match options(args) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("svr: {}\n{HELP}", escape_terminal(&message));
            return ExitCode::from(2);
        }
    };
    match execute(mode, &path) {
        Ok(changed) => ExitCode::from(u8::from(mode == Mode::Check && changed)),
        Err(Failure::Message(message)) => {
            eprintln!(
                "svr fmt: {}: {}",
                escape_terminal(&path.to_string_lossy()),
                escape_terminal(&message)
            );
            ExitCode::from(1)
        }
        Err(Failure::Diagnostics(diagnostics)) => {
            for diagnostic in diagnostics.items {
                eprintln!(
                    "error[{}] {}:{}:{}: {}",
                    diagnostic.code,
                    escape_terminal(&path.to_string_lossy()),
                    diagnostic.span.line + 1,
                    diagnostic.span.column + 1,
                    escape_terminal(&diagnostic.message)
                );
            }
            ExitCode::from(1)
        }
    }
}

fn execute(mode: Mode, path: &Path) -> Result<bool, Failure> {
    if mode == Mode::Write && !cfg!(windows) {
        return Err(Failure::Message("formatter --write is currently qualified only on Windows; use stdout mode on this host".into()));
    }
    let snapshot = files::Snapshot::read(path)?;
    if mode == Mode::Write {
        snapshot.writable()?;
    }
    let output = format_source(&snapshot.text)?;
    let changed = output != snapshot.text;
    match mode {
        Mode::Stdout => print!("{output}"),
        Mode::Check if changed => eprintln!(
            "Formatting needed: {}",
            escape_terminal(&path.to_string_lossy())
        ),
        Mode::Check => {}
        Mode::Write => {
            snapshot.replace(&output)?;
            eprintln!(
                "{}: {}",
                if changed { "Formatted" } else { "Unchanged" },
                escape_terminal(&path.to_string_lossy())
            );
        }
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_example_and_fixed_point() {
        let source = "fn main(){let title=\"Keep // text\";print(title);}";
        let expected = "fn main() {\n    let title = \"Keep // text\";\n    print(title);\n}\n";
        assert_eq!(format_source(source).unwrap(), expected);
        assert_eq!(format_source(expected).unwrap(), expected);
    }
    #[test]
    fn preserves_comments_literals_optional_semicolons_and_nested_records() {
        for source in [
            "// banner\r\nexport struct P{x:Int,y:Int}\r\nfn main(){let x=P{x:1,y:2} // keep  \r\nlet a=[2,3]; print(x.x)}",
            "mod m{export fn f(a:Int)->Int{return 0-1}} fn main(){if (true){m::f(1)}else if false{print(2)}else{print(3)}}",
            "fn main(){let text=\"a\\r\\nb \\\" // c\"; print(text); // trailing\n// own line\n}",
            "// Unicode comment: λ 日本語\nfn main(){print(\"naïve 🦀 // text\");}",
            "fn main(){print(1) print(2) let a=[1,2]; a[0]=3; while false{ return; }}",
            "// only comment  ", "", "fn f(){let x = missing;}"
        ] {
            let formatted = format_source(source).unwrap_or_else(|e| panic!("{source}: {e:?}"));
            assert_eq!(format_source(&formatted).unwrap(), formatted);
        }
    }
    #[test]
    fn invalid_unsupported_and_large_inputs_fail_without_repairs() {
        assert!(matches!(
            format_source("fn main(){let x=;}"),
            Err(Failure::Diagnostics(_))
        ));
        assert!(matches!(
            format_source("use lib::tools; fn main(){}"),
            Err(Failure::Message(_))
        ));
        assert!(format_source(&" ".repeat(files::INPUT_LIMIT + 1)).is_err());
        assert!(format_source("app Demo { }").is_err());
        assert!(format_source("fn main(){print(\"raw\nnewline\");}").is_err());
    }
    #[test]
    fn options_enforce_one_mode_and_allow_dash_filenames() {
        assert_eq!(
            options(&["--check".into(), "--".into(), "-source.svr".into()]).unwrap(),
            (Mode::Check, PathBuf::from("-source.svr"))
        );
        for args in [
            vec!["--check", "--write", "x.svr"],
            vec!["--write", "--write", "x.svr"],
            vec!["x.svr", "y.svr"],
            vec!["--help", "x.svr"],
            vec!["x.txt"],
            vec!["--check"],
        ] {
            assert!(options(&args.into_iter().map(str::to_owned).collect::<Vec<_>>()).is_err());
        }
    }
}
