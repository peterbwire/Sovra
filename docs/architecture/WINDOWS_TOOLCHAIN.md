# First installable Sovra Windows toolchain

Status: **Partial implementation**, updated 2026-10-13. This is a compiler development
environment, separate from the Node-free application executable proof in
[ADR 0025](../adr/0025-standalone-executable-strategy.md). A local installation
smoke test passes; a clean Windows VM installation is still required. Use the
[clean-machine procedure](../TOOLCHAIN_CLEAN_WINDOWS_TEST.md) to collect that evidence.

Optimized local checkpoint (2026-10-23): the current release-profile compiler and
matching runtime payload are packaged at
`target/toolchain/conditional-write-release-v1.zip`. Installation under restricted
PATH passes project commands, bundled and independent Task Manager operations,
conditional snapshot behavior, busy-marker preservation and deliberate recovery
of test-owned markers. The shipped self-test includes the busy-save regression.
These checks do not modify user profiles, establish a minimum supported Windows
version, or replace clean-VM/exact-commit hosted qualification. See the latest
development log for native-build acceptance and artifact hashes.

## Architecture audit

Toolchain diagnostics (2026-10-20): accepted [ADR 0031](../adr/0031-toolchain-doctor.md)
adds `svr doctor` with target-specific requirements, bounded probes and human/JSON
reports. It validates installed integrity and reuses native runtime validation.
Installer/self-tests still verify actual project creation and execution; doctor
does not certify native linking or deployment. See the
[report contract](../reference/doctor-json.md).

| Component | Current state | Boundary |
| --- | --- | --- |
| Rust compiler | Implemented subset | Lexer, parser, semantic checker and IR lowering in `src/compiler`; single Cargo package. |
| `svr.exe` CLI | Implemented subset | `new`, `check`, `run`, `test`, `doctor`, `build --emit ir|js|exe`, version/help. Native output requires `--output PATH` and build-time Rust/linker. Project scanning remains partial. |
| Interpreter and process runtime | Implemented subset | Rust `ProcessHost` supports arguments, line input, flushed output, UTF-8 text files and exit codes. |
| JavaScript backend | Implemented, Node-hosted | Emits Node CommonJS and retains parity tests; it is not a browser or standalone native backend. |
| Standard library | Implemented subset | Rust-owned builtins are compiled into `svr.exe`; no separately distributed stdlib source/archive yet. |
| Local packages | Partial | Direct local dependency resolution, checking, linking and execution; no lockfile, registry or publication. |
| Native app output | First host-native CLI slice | Accepted ADR 0027 adds `build --emit exe --output PATH` with embedded IR/interpreter, payload validation and no-overwrite publication. Deployment qualification remains pending. |
| UI adapters | Private prototypes | Node-hosted browser GUI and Windows WPF adapters; optional compiled C#/WPF host removes PowerShell from application execution. No published UI package or mobile renderer. |
| Language tests | Implemented first slice | ADR 0028: checked root Unit test functions, Rust interpreter, human/JSON reports. |

The included release candidate already ships `svr.exe` and Task Manager
source, but its flat archive has no installation, PATH or project-creation
workflow. The new Windows toolchain package establishes those boundaries
without changing the existing candidate archive or compiler commands.

## Smallest working distribution

Build with `cargo build --release`, then run
`powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-windows-toolchain.ps1`.
The script stages `target/toolchain/sovra-windows/`, creates a ZIP and SHA-256
file. The package contains:

```text
bin/svr.exe                 compiler, interpreter, built-in stdlib
bin/svr-new.ps1             compatibility wrapper for svr new
templates/console/          source and manifest template
docs/                      specification, operational and reference docs; course excluded
examples/                  source examples, including tested local packages
runtime-source/            version-matched Rust source for internal native linking
samples/task-manager/      tested Task Manager CLI source
install.ps1                 user-scope copy and PATH setup
verify-install.ps1          package-contained create/check/run/test self-test
imports.json                direct PE imports of the packaged svr.exe
integrity.json              SHA-256 list checked before installation
README.md, LICENSE*         use and licensing
```

The installer validates required files, stages a copy beside
`%LOCALAPPDATA%\Programs\Sovra` by default, then moves it into place. It adds
`bin` to the user PATH once and updates the current process PATH. It rejects
package/install path overlap and refuses to overwrite an existing prefix.
It also verifies the complete extracted package file list and file hashes
before creating the install prefix. This detects damaged or locally modified
files; because the manifest travels inside the ZIP, the external ZIP checksum
must still be verified to authenticate the downloaded artifact.
Project creation also refuses an
existing path. The template can live outside the Sovra source tree; its `main.svr`
checks and runs through installed `svr.exe`. Installation does not invoke Node
or Rust. The package currently has one Windows target built on the host; it
does not yet state a minimum Windows release or CRT deployment contract.
The automated negative tests confirm that an incomplete package creates no
prefix and that a prefix nested inside the package is rejected.
The build records `svr.exe` architecture and direct PE DLL imports in
`imports.json`; the artifact test recalculates these from the extracted binary.
The local GNU build currently reports x86_64 and direct imports from
`api-ms-win-core-synch-l1-2-0.dll`, `bcryptprimitives.dll`, `KERNEL32.dll`,
`msvcrt.dll` and `ntdll.dll`. These names describe that local artifact, not a
supported-Windows or CRT contract. Transitive imports, dynamic loads and
minimum OS support still require clean-machine testing.
CI uploads the ZIP and checksum, then a separate Windows job downloads and
tests those exact artifact bytes without a Rust or Node setup step. Its test
process also narrows PATH to Windows system directories and the installed
Sovra `bin` directory. This is stronger artifact evidence than staging alone,
but the hosted runner is not a clean Windows image.
The package now includes its own `verify-install.ps1`, so a clean-machine
tester can check PATH and create/check/run/test a temporary template project and
the bundled local-package example and Task Manager CLI without checking out
this repository. The
CI artifact test calls that script
and separately exercises a copied Task Manager source.

## Milestones and acceptance

1. **D1 — Installable compiler slice (implemented locally).** Package and
   install `svr.exe`, template, docs and PATH. Automated Windows test stages a
   package, verifies its ZIP checksum (and rejects a damaged copy), installs
   into a fresh temporary prefix, rejects reinstall without changing it,
   then starts a new PowerShell process with only the installed compiler and
   Windows system directories on PATH. It creates an out-of-repository project,
   checks and runs it, then copies the real Task Manager source to a separate
   directory and checks/adds/lists/completes/deletes there. Repeat on a
   clean compatible Windows VM before accepting the first user-facing gate.
2. **D2 — Project creation in `svr` (first slice implemented).**
   [ADR 0026](../adr/0026-project-creation-cli.md) approves `svr new <path>`;
   the Rust CLI embeds the console template and rejects existing destinations.
   The PowerShell script delegates to it for compatibility. `svr init` and
   template version policy remain future decisions.
3. **D3 — Developer commands.** Specify `svr build` artifact paths and package
   mode; language-level `svr test` first slice implemented under ADR 0028;
   standalone `svr fmt` implemented under ADR 0032; package formatting and
   non-Windows writes remain deferred;
   improve file/span diagnostics and JSON reports; add language-server/editor
   tooling. Each public command needs invocation, invalid-input and
   out-of-repository tests. Keep `--emit ir|js` behavior compatible.
4. **D4 — Packages and independent applications.** Add reproducible local
   dependency locks, package identity/version rules, then registry publishing
   and fetch security. Move Task Manager to a separate checkout and build it
   using only the installed toolchain and declared dependencies. The current
   single-file Task Manager is now tested from a separate temporary directory
   through installed `svr.exe`; this is not yet a maintained independent
   package/repository.
5. **D5 — Native application artifacts.** Stabilize an approved public build
   interface around the checked-IR executable proof, isolate runtime from
   compiler, test exact outputs and failure cases, audit binary imports and
   run on a clean Windows machine with no Node, Rust or `svr`. Native machine
   code generation is a later, distinct decision.
6. **D6 — UI.** Package the approved Sovra UI component/renderer contract,
   keep business logic renderer-independent, then add a browser target and
   Node-free desktop host. Android/iOS and WebAssembly need separate target
   capability and packaging contracts. Preserve the working Node GUI until
   replacements pass parity and accessibility tests.

The version string in Cargo is not evidence that these gates have passed.

## Native linking payload checkpoint (2026-10-14)

The package builder now generates `runtime-source/` from the current trusted
Rust source, Cargo manifest/lock, native builder and embedded console template.
Its SHA-256 manifest and the enclosing toolchain integrity manifest cover the
snapshot. The installer requires the payload's core files. This is a build-only
source snapshot, not a separately stabilized runtime crate.

The automated payload test relocates the snapshot outside the checkout, uses
Cargo's offline mode to build Task Manager there, deletes all payload/source/
build directories, then passes 18 native parity cases. A locally staged ZIP
using the debug compiler also passed installation and package-contained checks
with restricted PATH. Release artifact and clean-machine qualification remain
separate. Building native applications still needs Rust/Cargo and a host linker.

[ADR 0027](../adr/0027-native-build-cli.md) approves the public executable build
interface, now implemented with complete payload hashes/file-list/version checks,
owned staging and atomic no-overwrite publication. See the
[native build lesson](../course/native-builds.md) for invocation and limits.

`scripts/test-native-build-cli.mjs` now creates and installs a toolchain outside
the checkout, checks missing/tampered/version-mismatched payload and missing-tool
failures, then builds the console template, Task Manager and local-package
consumer through installed `svr`. It removes installation and source trees
before executing the programs. This passed locally, including 18 Task Manager
parity cases. The acceptance also passed using the release compiler via
`--compiler target/release/svr.exe`. A retained optimized toolchain ZIP passed
its exact-package installation self-test locally. Hosted and clean VM
qualification remain release gates.
