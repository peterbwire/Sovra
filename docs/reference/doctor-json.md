# Toolchain doctor reports

Accepted [ADR 0031](../adr/0031-toolchain-doctor.md) defines:

```text
svr doctor [--format human|json] [--target interpreter|js|exe|desktop]
```

Default target: interpreter. Default format: human. Both `--name=value` and
`--name value` are supported. Duplicate/unknown options, extra inputs and help
combined with other arguments exit 2 with stderr usage and no report. Help alone
exits 0. A completed report exits 0 if required checks pass, otherwise 1.

JSON stdout is one schema-1 object, with `kind: "doctor"`, `target`, `success`,
`compiler` and `checks`. Compiler fields are `version`, nullable `executable`,
`host_os` and `host_arch` (the compiler's target OS/architecture). Each check has
`id`, `required`, `status`, `message`, nullable `remediation` and nullable `evidence`.
Status is pass/warning/fail/not_checked. Messages/evidence are explanatory text,
not machine-readable version fields. Do not parse human output.

Checks appear in this order:

| ID | Evidence and requirements |
| --- | --- |
| compiler.identity | Actual current executable, compiled version/host; always required. |
| compiler.self_test | Captured, in-memory parse/check/lower/interpret exercise; always required. |
| install.integrity | Required in a packaged installation: bounded complete inventory and SHA-256 checks, strict manifest paths, no links/reparse points. Standard repository target/debug and target/release builds (including test binaries in deps) report non-required/not_checked. A relocated bare EXE fails this check. |
| native.runtime | Reuses native-build version/hash validation. Required for exe; recognized source checkout fallback is labelled explicitly. |
| tools.node | Resolved executable and `--version`, Node >=22. Required for js execution, not JS code generation or interpreter use. |
| tools.rustc | Resolved executable and `--version`, Rust >=1.74. Required for exe building. |
| tools.cargo | Resolved executable and `--version`, Cargo >=1.74. Required for exe building. |
| native.linker | Warning for exe, otherwise not_checked: linking is unproven because doctor never compiles a probe. |
| desktop.host | Windows system PowerShell 5.1+ in STA and WPF assembly load. Required for desktop; no window is opened. Unsupported hosts fail this target. |

The inventory includes optional tools. Optional missing/blocked/old tools produce
warnings without preventing interpreter success. Required checks must pass.
Only absolute PATH entries are searched; Windows probes execute `.exe` files,
never command wrappers. Desktop probing uses the system PowerShell path. Doctor
reports execution-policy blocks and does not disable them. It neither installs
nor repairs anything and does not execute user code or change PATH/project files.

Each child probe has a five-second deadline and 64 KiB combined output limit.
Both streams are captured into private OS-temporary files, bounded when read,
and cleaned up. Output can overshoot on disk between polling intervals. Timeout
and overflow terminate/reap the direct child; cleanup failure is reported. This
is not a process-tree sandbox. Child output cannot pollute JSON stdout. Human
reports escape terminal controls; JSON uses ordinary JSON escaping.

Probes run from their private capture directory, avoiding the caller's project
configuration. Child-only settings disable Rustup automatic installation
(`RUSTUP_AUTO_INSTALL=0`, documented in the [Rustup book](https://rust-lang.github.io/rustup/environment-variables.html))
and Cargo networking; Node startup hooks/coverage/redirection and Rustup trace
output are removed. The desktop probe uses only system PowerShell modules.
Parent environment settings are unchanged. Tool evidence contains the validated
numeric version rather than arbitrary child chatter. This inventory describes
available tools, not a project's toolchain override configuration.

Integrity checks detect damage relative to the supplied unsigned manifest, not
release authenticity. Verify the downloaded archive checksum independently.
Passing doctor does not certify linking, an application, a UI package, minimum OS
support, screen-reader access or clean-machine deployment. Retain native-build,
installed-toolchain and application acceptance tests for those claims.
