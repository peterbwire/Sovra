# Sovra

Sovra is a programming language and toolchain for `.svr` applications. Its
current Rust implementation provides a compiler, interpreter, built-in standard
library, local package linking, and a Node-targeted JavaScript backend. The
canonical command is `svr` (`svr.exe` on Windows).

**Version 1.0.0 is not yet approved for publication.** The version-one goal is
a production-quality Task Manager CLI and a qualified installable toolchain.
The Task Manager already supports interactive and scripted add/list/complete/
delete operations with persistent UTF-8 state. A local browser GUI works through
a Node host and Sovra-generated application code. Release qualification and a
clean-machine installation test remain. See the
[version-one gates](docs/V1_DELIVERY_GATES.md) and
[Windows toolchain audit](docs/architecture/WINDOWS_TOOLCHAIN.md).

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
`svr build --emit js` prints Node CommonJS JavaScript. For automation,
`svr check --format json <path>` emits a versioned diagnostic report. See
[automated checks](docs/guides/automated-checks.md) for its scope and exit codes.

`svr new`, `svr test`, `svr fmt`, `svr install` and `svr update` are reserved,
not implemented commands. Local packages work without a registry; lockfiles,
third-party publishing and a language server remain planned.

## Windows developer toolchain

The first toolchain slice packages `svr.exe`, a project template, documentation
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
`svr-new.ps1` for creating a console project while the public `svr new`
contract is still pending. The automated smoke test uses the installed compiler
from a fresh PowerShell process with Node, Cargo and Rust absent from PATH. It
also checks and runs a copy of the Task Manager outside this repository. A
clean compatible Windows VM test is still required before release.

An [internal standalone executable proof](docs/architecture/NODE_FREE_APPLICATIONS.md)
embeds checked Sovra IR and the Rust runtime into a Windows executable. It is
not a public native build command and has not passed the clean-machine
application-distribution gate. The existing interpreter and JavaScript backend
remain supported development paths.

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
