# ADR 0031: Toolchain environment diagnostics

Status: **Accepted (2026-10-20).** The user approved the proposed command,
targets, exit statuses, schema and bounded read-only probe contract.
Implementation and validation are tracked in the development log.

## Context and inspected implementation

At proposal time the CLI supported new/check/run/test/build, with reserved future
commands listed in `src/compiler.rs`. Doctor was not registered. Runtime payload discovery
and version/hash validation are private functions in `src/native_build.rs`.
`toolchain/install.ps1` checks the complete package integrity manifest before
installation. `toolchain/verify-install.ps1` creates/checks/runs/tests a temporary
project, while `scripts/test-windows-toolchain.ps1` tests installation and runs
acceptance with a restricted process PATH. These are existing implementation,
not a portable Rust environment-diagnostics command.

The private Windows desktop host uses Windows PowerShell and WPF. It does not
require Node or Rust to run an already-built application. Native compilation
still requires Rust/Cargo and a compatible linker on the developer's machine.
Do not conflate installed-compiler health, optional backends, desktop hosting,
successful linking, clean-machine qualification or release approval.

## Accepted public contract

```text
svr doctor [--format human|json] [--target interpreter|js|exe|desktop]
```

- Default format is human; default target is interpreter. Both `--format=value`
  and `--format value` forms are accepted, likewise for target. Options may be
  ordered freely. Reject duplicate options, unknown values/options and positional
  inputs with exit 2. `svr doctor --help` documents the contract and exits 0;
  combining help with other arguments is malformed invocation, not a probe run.
- Exit 0 when required checks for the selected target pass, 1 when any required
  check fails, and 2 for malformed invocation. Missing optional tools produce
  warnings, not interpreter failures. A report describes only the checks performed;
  success does not certify a native build or desktop application.
- No installation, downloads, PATH changes, project writes, user-source execution,
  registry modification, window creation or automatic repairs. Runtime/toolchain
  files are read only. Remediation is text, never executable commands automatically
  applied by doctor. Explicit developer-facing command suggestions are permitted.
- Check the running executable using `current_exe`, not whichever `svr` happens
  to resolve on PATH. Report compiled version, host OS/architecture and actual
  executable path. Report source-checkout fallback explicitly; an installed copy
  must not silently validate against its builder's checkout.
- Run an in-memory parse/check/lower/interpreter self-test using captured output,
  no process/file/input APIs and no application-specific program recognition.
  Self-test failure is required for every target.
- Inspect optional tools for the inventory, but mark target requirements explicitly.
  Never call a tool simply named by a report or a source file. Resolve fixed tool
  names on PATH and launch explicit argument arrays without a shell. Windows
  probes resolve executable files, not `.cmd`/`.bat` wrappers.

## Target requirements and evidence

| Check | interpreter | js | exe | desktop |
| --- | --- | --- | --- | --- |
| Compiler identity and in-memory self-test | Required | Required | Required | Required |
| Installed package integrity, if a packaged layout is detected | Required | Required | Required | Required |
| Version-matched native runtime source | Optional | Optional | Required | Optional |
| Runnable Node, supported host version | Optional | Required | Optional | Optional |
| Runnable rustc and Cargo, supported version | Optional | Optional | Required | Optional |
| Linker/build environment evidence | Informational | Informational | Warning if unproven | Informational |
| Compatible Windows PowerShell and WPF load probe | Optional | Optional | Optional | Required |

`js` means execution of generated Node JavaScript, not code generation alone.
Use Node >=22 as the initial supported execution-host floor. Rust must satisfy
the declared Cargo minimum (currently 1.74); parse versions numerically and report
unrecognized version output as unverified, never guess. A desktop target on other
OSes fails with a clear unsupported-host result; it does not claim a portable UI.
Optional failures never turn into required failures merely because a tool is absent.

Use existing runtime discovery/hash/version checks after extracting a shared,
read-only helper; preserve native-build behavior and test its callers. Implement
package integrity checking as a reusable Rust validator matching the installer:
schema, safe relative paths, unique complete file inventory and hashes. Bound
manifest size and traversal; refuse links/reparse points and paths outside the
package root. A missing manifest in a detected packaged layout is a required
failure. A recognized source build reports package-integrity `not_checked`, not
`pass`. An arbitrary relocated bare EXE must report installation files missing
rather than identify itself as a source checkout.

Probe `node --version`, `rustc --version` and `cargo --version` using resolved
executables. Desktop probing uses the Windows system PowerShell path in a hidden
STA process, no profile, with a fixed WPF assembly-load script; never invoke a
browser or show a window. Missing/blocked tools, nonzero exit, invalid output,
timeout and output overflow are distinct details, with useful remediation.
Preserve OS execution policy: report blocks rather than bypass them.

Probes have a five-second deadline and a 64 KiB combined output limit. Capture
both output streams concurrently, terminate and reap the direct child on timeout
or overflow, and release capture resources. The implementation uses uniquely
owned OS-temporary capture files instead of pipe-reader threads, so inherited
handles cannot strand blocked reader threads. It checks aggregate file size
while polling and reads at most 64 KiB plus one overflow byte; disk output can
overshoot between polls. Cleanup failures are reported. These internal temporary
captures do not modify project or installation files. This is bounded diagnostics, not a general
process-tree sandbox. Do not perform linking or invoke Cargo build. Report linker
availability as evidence/unknown rather than an actual native-build result.
Reference the existing native build acceptance procedure for full verification.

## Accepted report schema 1

JSON stdout contains exactly one report and no child-tool chatter:

```json
{
  "schema_version": 1,
  "kind": "doctor",
  "target": "interpreter",
  "success": true,
  "compiler": {
    "version": "1.0.0",
    "executable": "C:/Sovra/bin/svr.exe",
    "host_os": "windows",
    "host_arch": "x86_64"
  },
  "checks": [
    {
      "id": "compiler.self_test",
      "required": true,
      "status": "pass",
      "message": "In-memory compiler/interpreter self-test passed",
      "remediation": null,
      "evidence": null
    }
  ]
}
```

Stable check IDs: `compiler.identity`, `compiler.self_test`, `install.integrity`,
`native.runtime`, `tools.node`, `tools.rustc`, `tools.cargo`, `native.linker`,
`desktop.host`. Status is `pass`, `warning`, `fail` or `not_checked`.
Required checks must pass for success. Source-build package integrity is the one
explicit non-required/not_checked case. `evidence` is nullable text, bounded and
JSON escaped; remediation is nullable text. Keep fixed check order as listed.
Do not dump environment variables, credentials, full PATH or arbitrary config.
Human output escapes terminal controls in paths and tool output. JSON preserves
text with normal JSON escaping. Malformed invocation writes a human error to
stderr and emits no report, consistent with current CLI usage failures.

## Implementation milestones after approval

1. Private Rust report/check model, injectable environment and bounded child
   runner. Tests cover deadlines, output limits, missing/blocked tools, nonzero
   status, version parsing, stable ordering and optional-versus-required failures.
2. Shared read-only runtime discovery and strict package integrity validator;
   fixture tests cover corruption, missing/extra/duplicate members, path traversal,
   links and source/installed/relocated identity. Preserve native builder tests.
3. Real environment probes, captured in-memory self-test, CLI parsing and human/
   JSON output. Document verification limits; no stubs reported as passing checks.
4. Installed, out-of-repository and restricted-PATH acceptance: interpreter doctor
   passes without Node/Rust; js/exe fail for missing required tools. Desktop probes
   test assembly loading without launching windows. Add CI only for deterministic
   assertions appropriate to each host. Retain existing compiler/dogfood suites.
5. Rebuild the Windows toolchain and test exact extracted artifacts; update course,
   command reference, spec, help, changelog and development log. Keep course out of
   distributable archives under the existing packaging policy.

## Approval

The user approved the command/options, target meanings, Node floor, exit statuses,
report schema and read-only probe behavior on 2026-10-20.
No language syntax, type semantics, memory behavior or application runtime changes
are proposed. Existing command contracts remain unchanged.
