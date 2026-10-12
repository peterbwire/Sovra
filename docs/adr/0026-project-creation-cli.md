# ADR 0026: Public console project creation command

Status: **Accepted and implemented (first slice).**

## Context

The first Windows toolchain ZIP installs `svr.exe` and a `svr-new.ps1` script.
That script creates a one-file console project, while `svr new` and `svr init`
are reserved and still report “not implemented.” The package-contained
self-test proves that the generated source checks and runs, including through
`svr run <directory>`. An installed compiler should eventually create projects
without a separate script or template path. This is a public CLI compatibility
decision; it does not change `.svr` syntax or runtime semantics.

## Proposed first contract

Implement only `svr new <path>` in the Rust CLI. Keep `svr init` reserved until
its separate in-place behavior is designed. Require exactly one path argument;
no default path, force flag, overwrite or interactive prompt in this slice.
Resolve relative paths against the process working directory. Reject an
existing destination, including an empty directory, before writing anything.
The basename becomes the manifest project name and must match
`[A-Za-z][A-Za-z0-9_-]*`, preserving the installed script's current naming
rule. Parent directories may be created; a creation failure must not leave a
partially generated project. Do not follow an existing destination symlink.

Generate exactly the existing console template:

```text
<path>/sovra.toml  [project] name = basename, version = "0.1.0", entry = "main.svr"
<path>/main.svr    fn main() { print("Hello, Sovra!"); }
```

The Rust binary should embed the template so `svr new` works from a standalone
compiler or Cargo source installation. Keep the packaged PowerShell script as
a compatibility wrapper that delegates to `svr new`; it should not maintain a
second generator. Successful creation exits 0; invocation errors exit 2;
I/O and project-creation errors exit 1. Errors go to stderr and name the path
without claiming that a partial project is valid. `svr --help` and
`svr new --help` must describe the implemented form. No change is proposed to
`svr check`, `svr run` or `svr build` behavior.

## Required tests and documentation

- Rust unit tests for project-name/path validation and no-partial-output on
  failures; CLI subprocess tests for help, missing/extra arguments, invalid
  names, existing destinations and successful creation.
- Check and run the generated source using the new binary; run the generated
  directory as a package; repeat through the installed Windows ZIP and the
  external-checkout test.
- Keep template bytes synchronized, or use one embedded source; update the
  spec only if any language-visible source changes (none proposed).
- Update `README.md`, toolchain guide, command help, roadmap, status and
  release qualification notes only after implementation passes.

## Decision

Approved: `svr new <path>` uses the existing ASCII name rule, refuses to
overwrite, embeds the template, and leaves `svr init` for a later decision.
The implementation lives in `src/project_new.rs` and is called by `src/cli.rs`.
