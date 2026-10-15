# Install Sovra and create a project

The Windows toolchain includes `svr.exe`, its built-in standard library, a console
template, documentation, examples and a Task Manager CLI sample. Checking,
running and testing need neither Node nor Rust. Native executable builds need
Rust/Cargo and a compatible host linker on the developer machine.

## Install

Verify the downloaded ZIP against its provided SHA-256 checksum. Extract it into
a new directory and open PowerShell there:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\install.ps1
```

The default installation is `$env:LOCALAPPDATA\Programs\Sovra`. Installation
checks package integrity, refuses an existing destination and adds `bin` to your
user PATH. Open a new PowerShell window afterward:

```powershell
svr --version
powershell -NoProfile -ExecutionPolicy Bypass -File "$env:LOCALAPPDATA\Programs\Sovra\verify-install.ps1"
```

For a custom installation, pass `-Prefix <directory>` to the installer and
verification script. See the packaged README and the
[clean Windows procedure](../TOOLCHAIN_CLEAN_WINDOWS_TEST.md). These steps describe
a locally verified package; clean-machine deployment qualification remains pending.

## Create, check, run and test

In a writable directory, choose a project path that does not exist:

```powershell
svr new .\hello
svr check .\hello\main.svr
svr run .\hello
svr test .\hello
svr test --format json .\hello
```

The generated project contains `sovra.toml` and `main.svr`. Its greeting helper
returns `"Hello, Sovra!"`; `main` prints it, and `test_greeting` checks it.
Expect one greeting from run and one passing test from test. Existing project
paths are never overwritten. Project names use the existing ASCII name rule.

Edit `main.svr` and repeat check/run/test. Every function parameter requires a
type annotation; local `let` bindings can infer types. An omitted return
annotation means Unit. Continue with [functions](functions.md).

`svr test` checks all bodies before executing tests and does not automatically
run `main`. Follow [testing](testing.md) for failures and real file effects.
For machine-readable checking, use `svr check --format json .\hello\main.svr`.
See the [check JSON](../reference/check-json.md) and
[test JSON](../reference/test-json.md) contracts.

## Build targets

```powershell
svr build --emit ir .\hello\main.svr
svr build --emit js .\hello\main.svr > .\hello.js
```

IR and JavaScript are written to stdout. Generated JavaScript requires Node;
it is not a browser UI module. PowerShell versions differ in redirection encoding;
use UTF-8 when saving generated code. Follow [native builds](native-builds.md)
for `--emit exe --output PATH`, developer dependencies and deployment limits.

Compiler contributors can run `cargo build --locked` in the repository and use
`target/debug/svr.exe`, or replace `svr` with `cargo run --`. That source-development
workflow requires Rust and is separate from installing the packaged toolchain.
