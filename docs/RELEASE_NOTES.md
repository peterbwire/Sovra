# Sovra developer preview — draft release notes

**Superseded: do not publish these notes.** The release target is now production
readiness under `docs/PRODUCTION_READINESS.md`. The text below is historical draft
material, not an approved release scope. Candidate workflows remain verification
infrastructure only.

Version and publication date: pending release review. The current development
package version is 0.1.0; this document does not announce a published release.

Sovra provides the `svr` compiler and interpreter for a small, explicitly typed
language subset. This preview is for experimentation and feedback; language and
experimental Rust APIs can change before a stable release.

Implemented features include source-file checking/execution, typed function
parameters, inferred locals, inline modules, a small Rust-owned standard library,
text IR output, JavaScript generation and versioned JSON diagnostics. Parameters
require annotations; bare `print` remains supported.

Project checking is partial manifest/import/wiring validation. Opt-in
`check --service-calls` inspects a limited body subset and reports unsupported
files explicitly. Its JSON output includes coverage and member-call references.
Inspection success does not imply application type checking or execution.

## Try the binary

Extract the archive for your operating system and architecture. From its directory:

```sh
./svr --version
./svr run hello.svr
./svr check --format json hello.svr
./svr build --emit js hello.svr > hello.js
node hello.js
```

On Windows use `./svr.exe` (or `.\svr.exe` in PowerShell). Node is needed only
for running generated JavaScript, not for `svr run`. The included licenses apply
to the toolchain. Archives are currently unsigned candidate builds.

## Known limits

- Fielddesk demonstrates planned application syntax and is not runnable.
- Project `run`, Sovra-native `test`, package installation/publication, formatter,
  REPL and language server are not implemented.
- General application types, control flow and service implementation scopes are
  incomplete. Unsupported inspection syntax reports E4096 rather than success.
- Local service module identities currently use canonical filesystem paths;
  published library identities and package dependency resolution remain planned.
- Building Sovra from source declares Rust 1.74 minimum; release acceptance
  requires the dedicated MSRV and platform CI jobs to pass.

No release artifacts or package registry entries have been published by the
preparation workflow. Final notes must replace this draft status after review.
