# Sovra

Sovra is a programming language and toolchain for `.svr` applications. Its
current Rust implementation provides a compiler, interpreter, built-in standard
library, local package linking, and a Node-targeted JavaScript backend. The
canonical command is `svr` (`svr.exe` on Windows).

Use `svr doctor` to check installation health and optional tools, or
`svr doctor --target exe --format json` for native-build prerequisite diagnostics.
It reports fixes without applying them; see [doctor reports](docs/reference/doctor-json.md).

Compiler diagnostics now recover at declaration boundaries, avoiding per-token
error floods while retaining later independent errors and original source ranges.
Invalid source still fails compilation; the accepted language grammar is unchanged.
Nested block recovery also retains `else` branches and later statement diagnostics.

**Active focus: Sovra UI powering the Task Manager in a desktop window.**
The private Windows renderer maps Sovra views and events to WPF native controls.
Task rules and persistence stay in `.svr`; an unrelated component uses the same
renderer. A compiled C#/WPF adapter now provides a PowerShell-free application
runtime, selected with the internal packager's `-CompiledHost` switch. It requires
compatible Windows x64 and .NET/WPF, with no Node, browser, Rust or Sovra
installation needed by a packaged native app. The previous PowerShell adapter
remains the packaging default during parity qualification.
Grouped Task Manager views now paginate 25 matching tasks at a time and bound
long-text display; 1,000-task navigation runs through actual Sovra execution.
See the [desktop adapter](toolchain/ui/windows/README.md) and
[UI architecture](docs/architecture/SOVRA_UI.md) for tests and current limits.
The standalone CLI executable alone does not include a window; a desktop
package supplies **Sovra UI.exe**. In a compiled-host package this executable
owns the window and renderer directly.

**Version 1.0.0 is not yet approved for publication.** The version-one goal is
a production-quality Task Manager CLI and a qualified installable toolchain.
The Task Manager already supports interactive and scripted add/list/complete/
delete operations with persistent UTF-8 state. A local browser GUI works through
a Node host and Sovra-generated application code. Release qualification and a
clean-machine installation test remain. See the
[version-one gates](docs/V1_DELIVERY_GATES.md) and
[Windows toolchain audit](docs/architecture/WINDOWS_TOOLCHAIN.md).
The [implementation handoff](docs/V1_IMPLEMENTATION_HANDOFF.md) identifies the
final local artifacts, verification evidence and remaining release gates.

## Try Sovra from source

Development requires Rust. The JavaScript backend's execution tests and the
browser GUI require Node.js 22. Checking and interpreting Sovra source do not
require Node.

```text
cargo build
cargo run -- check examples/hello-world/main.svr
cargo run -- run examples/hello-world/main.svr
cargo test --all-targets
```

`svr check` validates a source file or project; project checks have different
coverage depending on the project type. `svr run` executes a `.svr` file or a
local dependency package. `svr build` prints human-readable IR by default;
`svr build --emit js` prints Node CommonJS JavaScript.
`svr build --emit exe --output app.exe <source-or-package>` links a host-native
standalone executable; the developer needs Rust/Cargo and a compatible linker.
See [native builds](docs/course/native-builds.md). For automation,
`svr check --format json <path>` emits a versioned diagnostic report. See
[automated checks](docs/guides/automated-checks.md) for its scope and exit codes.

`svr new <path>` creates a console project with a runnable test.
`svr test [--format human|json] <source-or-package>` runs root-level Unit test
functions; see [testing](docs/course/testing.md). `svr fmt [--check | --write] file.svr`
formats standalone source; write mode is currently enabled on Windows only.
See [formatting](docs/reference/formatter.md). `svr install` and `svr update`
remain reserved. Local packages work without a registry; lockfiles,
third-party publishing and a language server remain planned.
The [developer tooling plan](docs/DEVELOPER_TOOLS.md) tracks reusable execution
support and the proposed language test command.

## Windows developer toolchain

The first toolchain slice packages `svr.exe`, a project template, Task Manager
CLI source, documentation
and a PowerShell installer into a ZIP with a SHA-256 checksum. To build and
smoke-test that package on Windows:

```powershell
cargo build --release
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-windows-toolchain.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-windows-toolchain.ps1 -Package target/toolchain/sovra-windows.zip
```

After extracting the ZIP, follow the included
[toolchain installation guide](toolchain/README.md). It installs to the current
user's profile by default, adds `svr.exe` to PATH, and provides
`svr new <path>` for creating a console project. The bundled `svr-new.ps1`
remains as a compatibility wrapper. The package includes `verify-install.ps1` for a
repository-free create/check/run self-test. The automated smoke test uses the
installed compiler
from a fresh PowerShell process with Node, Cargo and Rust absent from PATH. It
also checks and runs a copy of the Task Manager outside this repository. A
clean compatible Windows VM test is still required before release.

An [internal standalone executable proof](docs/architecture/NODE_FREE_APPLICATIONS.md)
embeds checked Sovra IR and the Rust runtime into a Windows executable. Its
18-case Task Manager regression compares output, exit status and persistent
state against the interpreter. A Windows proof ZIP includes checksums, an
import record and a self-test for the later clean-machine test. It is
now complemented by the approved public `build --emit exe --output PATH`
command. Clean-machine application-distribution qualification remains pending.
The existing interpreter and JavaScript backend
remain supported development paths.
The [executable capability matrix](docs/NATIVE_CAPABILITY_MATRIX.md) records
verified language/runtime support and the remaining native toolchain gates.

## Applications and documentation

The [dogfood suite](dogfood/README.md) currently exercises nine runnable
application slices in both execution engines. They cover arguments and records,
streamed logs, text files, persistent tasks and notes, a browser File Manager,
local package consumption, bounded numerical work, and a Node-hosted HTTP
server. Their exact status and limitations are in
[dogfood status](docs/DOGFOOD_STATUS.md); a passing slice does not imply a
complete platform feature.

Start with the [language specification](docs/spec.md),
[architecture](ARCHITECTURE.md), [roadmap](docs/roadmap.md), and
[course](docs/course/README.md). Contributors should read
[CONTRIBUTING.md](CONTRIBUTING.md) and
[production upgrade flow](docs/production-upgrade-flow.md).

## License

Sovra is dual-licensed under the [MIT license](LICENSE-MIT) or the
[Apache 2.0 license](LICENSE-APACHE).
