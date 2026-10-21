//! Approved read-only toolchain environment diagnostics (ADR 0031).

mod integrity;
mod probe;

use crate::compiler::{
    check_report::push_string, interpreter, ir, parser::Parser, semantic::SemanticAnalyzer,
    stdlib::escape_terminal,
};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

const HELP: &str = "Usage: svr doctor [--format human|json] [--target interpreter|js|exe|desktop]\n  Read-only diagnostics; optional tools do not prevent interpreter use.\n  Success checks prerequisites, not actual linking or clean-machine qualification.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Interpreter,
    Js,
    Exe,
    Desktop,
}
impl Target {
    fn name(self) -> &'static str {
        match self {
            Self::Interpreter => "interpreter",
            Self::Js => "js",
            Self::Exe => "exe",
            Self::Desktop => "desktop",
        }
    }
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "interpreter" => Ok(Self::Interpreter),
            "js" => Ok(Self::Js),
            "exe" => Ok(Self::Exe),
            "desktop" => Ok(Self::Desktop),
            _ => Err("unknown doctor target".into()),
        }
    }
}

#[derive(Debug)]
struct Check {
    id: &'static str,
    required: bool,
    status: &'static str,
    message: String,
    remediation: Option<String>,
    evidence: Option<String>,
}
impl Check {
    fn result(
        id: &'static str,
        required: bool,
        result: Result<String, String>,
        remediation: &str,
    ) -> Self {
        match result {
            Ok(evidence) => Self {
                id,
                required,
                status: "pass",
                message: "Check passed".into(),
                remediation: None,
                evidence: Some(evidence),
            },
            Err(message) => Self {
                id,
                required,
                status: if required { "fail" } else { "warning" },
                message,
                remediation: Some(remediation.into()),
                evidence: None,
            },
        }
    }
}

#[derive(Debug)]
struct Report {
    target: Target,
    executable: Option<PathBuf>,
    checks: Vec<Check>,
}
impl Report {
    fn success(&self) -> bool {
        self.checks
            .iter()
            .all(|check| !check.required || check.status == "pass")
    }
    fn json(&self) -> String {
        let mut output = format!("{{\"schema_version\":1,\"kind\":\"doctor\",\"target\":\"{}\",\"success\":{},\"compiler\":{{\"version\":", self.target.name(), self.success());
        push_string(&mut output, env!("CARGO_PKG_VERSION"));
        output.push_str(",\"executable\":");
        nullable(
            &mut output,
            self.executable
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .as_deref(),
        );
        output.push_str(",\"host_os\":");
        push_string(&mut output, std::env::consts::OS);
        output.push_str(",\"host_arch\":");
        push_string(&mut output, std::env::consts::ARCH);
        output.push_str("},\"checks\":[");
        for (index, check) in self.checks.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            output.push_str("{\"id\":");
            push_string(&mut output, check.id);
            output.push_str(&format!(",\"required\":{},\"status\":", check.required));
            push_string(&mut output, check.status);
            output.push_str(",\"message\":");
            push_string(&mut output, &check.message);
            output.push_str(",\"remediation\":");
            nullable(&mut output, check.remediation.as_deref());
            output.push_str(",\"evidence\":");
            nullable(&mut output, check.evidence.as_deref());
            output.push('}');
        }
        output.push_str("]}");
        output
    }
    fn human(&self) -> String {
        let mut output = format!(
            "Sovra doctor: {} (svr {})\n",
            self.target.name(),
            env!("CARGO_PKG_VERSION")
        );
        for check in &self.checks {
            output.push_str(&format!(
                "{} {}{}: {}\n",
                check.status.to_uppercase(),
                check.id,
                if check.required { " [required]" } else { "" },
                escape_terminal(&check.message)
            ));
            for value in [check.evidence.as_ref(), check.remediation.as_ref()]
                .into_iter()
                .flatten()
            {
                output.push_str("  ");
                output.push_str(&escape_terminal(value));
                output.push('\n');
            }
        }
        output.push_str(if self.success() {
            "Required checks passed.\n"
        } else {
            "Required checks failed.\n"
        });
        output.push_str("Prerequisite checks do not certify a build or deployment.\n");
        output
    }
}

fn nullable(output: &mut String, value: Option<&str>) {
    match value {
        Some(value) => push_string(output, value),
        None => output.push_str("null"),
    }
}

fn options(args: &[String]) -> Result<(Target, bool), String> {
    let mut target = None;
    let mut json = None;
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        let (name, value) = if let Some(pair) = arg.split_once('=') {
            pair
        } else {
            if !matches!(arg.as_str(), "--target" | "--format") {
                return Err(format!("unexpected doctor argument: {arg}"));
            }
            index += 1;
            (
                arg.as_str(),
                args.get(index)
                    .ok_or("missing doctor option value")?
                    .as_str(),
            )
        };
        match name {
            "--target" if target.is_none() => target = Some(Target::parse(value)?),
            "--format" if json.is_none() => {
                json = Some(match value {
                    "human" => false,
                    "json" => true,
                    _ => return Err("doctor format must be human or json".into()),
                })
            }
            _ => return Err(format!("unknown or duplicate doctor option: {name}")),
        }
        index += 1;
    }
    Ok((target.unwrap_or(Target::Interpreter), json.unwrap_or(false)))
}

pub(crate) fn command(args: &[String]) -> ExitCode {
    if args.len() == 1 && matches!(args[0].as_str(), "--help" | "-h") {
        println!("{HELP}");
        return ExitCode::SUCCESS;
    }
    let (target, json) = match options(args) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("svr: {}\n{HELP}", escape_terminal(&error));
            return ExitCode::from(2);
        }
    };
    let report = collect(target);
    if json {
        println!("{}", report.json());
    } else {
        print!("{}", report.human());
    }
    ExitCode::from(u8::from(!report.success()))
}

fn self_test() -> Result<String, String> {
    let source = "fn twice(n: Int) -> Int { return n * 2; } fn main() { let mut values = [1, 2]; values[0] = twice(3); if values[0] == 6 { print(\"doctor-ok\"); } }";
    let ast = Parser::new()
        .parse_source(source)
        .map_err(|_| "embedded self-test parse failed")?;
    let typed = SemanticAnalyzer::new()
        .analyze(&ast)
        .map_err(|_| "embedded self-test analysis failed")?;
    let output = interpreter::run(&ir::lower(&typed))?;
    if output != ["doctor-ok"] {
        return Err("embedded self-test produced unexpected output".into());
    }
    Ok("In-memory parse/check/lower/interpreter self-test passed".into())
}

fn tool(name: &str, prefix: &str, floor: (u64, u64, u64)) -> Result<String, String> {
    let path = probe::resolve(name)?;
    let output = probe::run(&path, &[OsStr::new("--version")], Duration::from_secs(5))?;
    let version = version(&output, prefix, floor)?;
    // Only publish the validated numeric version, never arbitrary tool chatter.
    Ok(format!("{}: {version}", path.display()))
}

fn version(output: &str, prefix: &str, floor: (u64, u64, u64)) -> Result<String, String> {
    let line = output
        .lines()
        .next()
        .ok_or("version output is empty")?
        .trim();
    let text = line
        .strip_prefix(prefix)
        .ok_or("unrecognized version output")?;
    let number = text
        .split_whitespace()
        .next()
        .ok_or("missing version")?
        .split('-')
        .next()
        .ok_or("missing version")?;
    let mut parts = number.split('.');
    let mut part = || {
        parts
            .next()
            .and_then(|s| {
                if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) {
                    s.parse::<u64>().ok()
                } else {
                    None
                }
            })
            .ok_or("unrecognized version number")
    };
    let found = (part()?, part()?, part()?);
    if parts.next().is_some() {
        return Err("unrecognized version number".into());
    }
    if found < floor {
        return Err(format!(
            "unsupported version; requires >= {}.{}.{}",
            floor.0, floor.1, floor.2
        ));
    }
    Ok(format!("{prefix}{}.{}.{}", found.0, found.1, found.2))
}

fn desktop() -> Result<String, String> {
    if !cfg!(windows) {
        return Err("desktop host is only supported on Windows".into());
    }
    let windows =
        std::env::var_os("SystemRoot").ok_or("Windows system directory is unavailable")?;
    let path = Path::new(&windows).join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let script = "$ErrorActionPreference='Stop'; if ($PSVersionTable.PSVersion -lt [version]'5.1') { throw 'PowerShell 5.1 required' }; Add-Type -AssemblyName PresentationFramework; if ([Threading.Thread]::CurrentThread.ApartmentState -ne 'STA') { throw 'STA required' }; Write-Output 'SVR-WPF-OK'";
    let args = ["-NoProfile", "-NonInteractive", "-STA", "-Command", script].map(OsStr::new);
    let output = probe::run(&path, &args, Duration::from_secs(5))?;
    if output.trim() != "SVR-WPF-OK" {
        return Err("desktop assembly probe returned unexpected output".into());
    }
    Ok("Windows PowerShell 5.1+ STA and WPF assembly load passed; no window opened".into())
}

#[derive(Debug)]
struct Findings {
    executable: Result<PathBuf, String>,
    source: bool,
    installation: Result<String, String>,
    runtime: Result<String, String>,
    node: Result<String, String>,
    rustc: Result<String, String>,
    cargo: Result<String, String>,
    desktop: Result<String, String>,
}

fn collect(target: Target) -> Report {
    let executable = std::env::current_exe().map_err(|e| e.to_string());
    let source = executable
        .as_ref()
        .is_ok_and(|p| crate::native_build::is_source_build(p));
    let installation = executable.as_ref().map_err(Clone::clone).and_then(|p| {
        let root = p
            .parent()
            .and_then(Path::parent)
            .ok_or("cannot identify installation root")?;
        if source {
            return Ok(
                "Recognized source build; no packaged integrity manifest was checked".into(),
            );
        }
        if !p.parent().is_some_and(|p| p.ends_with("bin")) {
            return Err("relocated bare executable has no recognized installation layout".into());
        }
        integrity::validate(root)?;
        Ok(format!(
            "Complete package inventory and hashes validated: {}",
            root.display()
        ))
    });
    let runtime = executable
        .as_ref()
        .map_err(Clone::clone)
        .and_then(|p| crate::native_build::discover_runtime_for(p))
        .and_then(|p| {
            if !source && p == Path::new(env!("CARGO_MANIFEST_DIR")) {
                return Err("unrecognized development executable layout; installed runtime payload is missing".into());
            }
            Ok(format!(
                "{}{}",
                p.display(),
                if source {
                    " (source checkout fallback)"
                } else {
                    " (validated installed payload)"
                }
            ))
        });
    assemble(
        target,
        Findings {
            executable,
            source,
            installation,
            runtime,
            node: tool("node", "v", (22, 0, 0)),
            rustc: tool("rustc", "rustc ", (1, 74, 0)),
            cargo: tool("cargo", "cargo ", (1, 74, 0)),
            desktop: desktop(),
        },
    )
}

fn assemble(target: Target, findings: Findings) -> Report {
    let identity = findings
        .executable
        .as_ref()
        .map(|p| p.display().to_string())
        .map_err(Clone::clone);
    let mut checks = vec![
        Check::result("compiler.identity", true, identity, "Reinstall the complete Sovra toolchain."),
        Check::result("compiler.self_test", true, self_test(), "Reinstall a verified compiler and report the failure."),
        Check::result("install.integrity", !findings.source, findings.installation, "Verify the downloaded ZIP checksum and reinstall the complete package."),
        Check::result("native.runtime", target == Target::Exe, findings.runtime, "Restore the version-matched runtime-source directory beside bin; do not mix toolchain versions."),
        Check::result("tools.node", target == Target::Js, findings.node, "Install Node.js 22 or newer and put its executable on PATH for generated JavaScript execution."),
        Check::result("tools.rustc", target == Target::Exe, findings.rustc, "Install Rust 1.74 or newer for this host; Rust is needed to build native applications, not to run them."),
        Check::result("tools.cargo", target == Target::Exe, findings.cargo, "Install Cargo 1.74 or newer and make it available on PATH."),
        Check { id: "native.linker", required: false, status: if target == Target::Exe { "warning" } else { "not_checked" }, message: "Linker/build environment is unproven; no linking was performed".into(), remediation: Some("Use the documented native-build acceptance procedure with a compatible host linker.".into()), evidence: None },
        Check::result("desktop.host", target == Target::Desktop, findings.desktop, "Use compatible Windows with Windows PowerShell 5.1 and WPF; this does not install a UI package."),
    ];
    if findings.source {
        checks[2].status = "not_checked";
        checks[2].message = "Source build; installed package integrity was not checked".into();
    }
    Report {
        target,
        executable: findings.executable.ok(),
        checks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn absent() -> Findings {
        Findings {
            executable: Ok(PathBuf::from("/source/target/debug/svr")),
            source: true,
            installation: Ok("source".into()),
            runtime: Err("missing runtime".into()),
            node: Err("missing node".into()),
            rustc: Err("missing Rust".into()),
            cargo: Err("missing Cargo".into()),
            desktop: Err("unsupported host".into()),
        }
    }
    #[test]
    fn target_requirements_and_fixed_order_do_not_confuse_optional_tools() {
        let report = assemble(Target::Interpreter, absent());
        assert!(report.success());
        assert_eq!(
            report.checks.iter().map(|c| c.id).collect::<Vec<_>>(),
            [
                "compiler.identity",
                "compiler.self_test",
                "install.integrity",
                "native.runtime",
                "tools.node",
                "tools.rustc",
                "tools.cargo",
                "native.linker",
                "desktop.host"
            ]
        );
        for target in [Target::Js, Target::Exe, Target::Desktop] {
            assert!(!assemble(target, absent()).success());
        }
        let mut installed = absent();
        installed.source = false;
        installed.installation = Err("corrupt".into());
        assert!(!assemble(Target::Interpreter, installed).success());
    }
    #[test]
    fn versions_are_numeric_and_unknown_output_is_not_success() {
        assert_eq!(
            version("v22.0.0\nprivate chatter", "v", (22, 0, 0)).unwrap(),
            "v22.0.0"
        );
        assert!(version("v22.1.0\n", "v", (22, 0, 0)).is_ok());
        assert!(version("rustc 1.100.0 (test)\n", "rustc ", (1, 74, 0)).is_ok());
        for text in [
            "v9.0.0",
            "v21.99.0",
            "v22.0",
            "v22.0.0.1",
            "v22.x.0",
            "unknown",
            "",
        ] {
            assert!(version(text, "v", (22, 0, 0)).is_err(), "{text}");
        }
    }
    #[test]
    fn options_reject_duplicates_and_help_combinations() {
        for args in [
            vec!["--format=json", "--format", "human"],
            vec!["--target", "exe", "--target=js"],
            vec!["--help", "--format=json"],
            vec!["--format"],
            vec!["project"],
            vec!["--target=unknown"],
            vec!["--unknown=x"],
        ] {
            assert!(options(&args.into_iter().map(str::to_owned).collect::<Vec<_>>()).is_err());
        }
        assert_eq!(options(&[]).unwrap(), (Target::Interpreter, false));
        assert_eq!(
            options(&["--target=js".into(), "--format".into(), "json".into()]).unwrap(),
            (Target::Js, true)
        );
    }
    #[test]
    fn human_report_escapes_controls() {
        let mut findings = absent();
        findings.node = Err("bad\u{1b}[31m\nline".into());
        let report = assemble(Target::Interpreter, findings);
        assert!(!report.human().contains('\u{1b}'));
        assert!(report.json().contains("\\u001b"));
    }
}
