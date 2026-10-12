# First installable Sovra Windows toolchain

Status: **Partial implementation**, 2026-10-12. This is a compiler development
environment, separate from the Node-free application executable proof in
[ADR 0025](../adr/0025-standalone-executable-strategy.md). A local installation
smoke test passes; a clean Windows VM installation is still required.

## Architecture audit

| Component | Current state | Boundary |
| --- | --- | --- |
| Rust compiler | Implemented subset | Lexer, parser, semantic checker and IR lowering in `src/compiler`; single Cargo package. |
| `svr.exe` CLI | Implemented subset | `check`, `run`, `build --emit ir|js`, version/help. `check` on dependency packages uses full package compilation; shallow application project scanning is partial. |
| Interpreter and process runtime | Implemented subset | Rust `ProcessHost` supports arguments, line input, flushed output, UTF-8 text files and exit codes. |
| JavaScript backend | Implemented, Node-hosted | Emits Node CommonJS and retains parity tests; it is not a browser or standalone native backend. |
| Standard library | Implemented subset | Rust-owned builtins are compiled into `svr.exe`; no separately distributed stdlib source/archive yet. |
| Local packages | Partial | Direct local dependency resolution, checking, linking and execution; no lockfile, registry or publication. |
| Native app output | Experimental internal proof | `examples/native_builder.rs` embeds checked IR into an executable with the Rust interpreter. No public `svr build` native target or deployment qualification. |
| Browser UI | Experimental | Task Manager Node host and Sovra UI view/event prototype; no installed UI framework or Node-free desktop renderer. |
| Language server, debugger, formatter, language test runner | Missing/planned | Reserved CLI names do not implement them. |

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
bin/svr-new.ps1             console project creator
templates/console/          source and manifest template
docs/                      language spec, JSON diagnostics, local package lesson
install.ps1                 user-scope copy and PATH setup
README.md, LICENSE*         use and licensing
```

The installer copies the package to `%LOCALAPPDATA%\Programs\Sovra` by default,
adds its `bin` to the user PATH once, and updates the current process PATH.
It refuses to overwrite an existing prefix. Project creation also refuses an
existing path. The template can live outside the Sovra source tree; its `main.svr`
checks and runs through installed `svr.exe`. Installation does not invoke Node
or Rust. The package currently has one Windows target built on the host; it
does not yet state a minimum Windows release or CRT deployment contract.

## Milestones and acceptance

1. **D1 — Installable compiler slice (implemented locally).** Package and
   install `svr.exe`, template, docs and PATH. Automated Windows test stages a
   package, verifies its ZIP checksum, installs into a fresh temporary prefix,
   then starts a new PowerShell process with only the installed compiler and
   Windows system directories on PATH. It creates an out-of-repository project,
   checks and runs it, then copies the real Task Manager source to a separate
   directory and checks/adds/lists/completes/deletes there. Repeat on a
   clean compatible Windows VM before accepting the first user-facing gate.
2. **D2 — Project creation in `svr`.** Design and approve the public `svr new`
   and `svr init` syntax, manifest naming, overwrite rules and template version
   policy. Move creation into the Rust CLI only after contract approval; retain
   the script or an alias for existing installed packages.
3. **D3 — Developer commands.** Specify `svr build` artifact paths and package
   mode; implement language-level `svr test`; implement syntax-aware `svr fmt`;
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
