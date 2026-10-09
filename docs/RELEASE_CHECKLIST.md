# Production release checklist

Status: **Blocked on production requirements; preview plan superseded.** The user
requires production readiness. Packaging checks remain useful engineering
infrastructure, but cannot authorize a release. The advertised version-one
criteria in [the delivery gates](V1_DELIVERY_GATES.md) must be satisfied first.

## Release gates

Scope revised 2026-10-09: **a production-quality Task Manager CLI built in
Sovra before version 1.0**. Full HTTP/database/WebSocket application behavior and
third-party library registry publishing move to later versions. See
[the version-one delivery gates](V1_DELIVERY_GATES.md) for concrete acceptance.

- [x] Describe implemented versus experimental features in the root README.
- [x] Replace compiler-only release claims with accurate draft status and scope.
- [x] Limit Cargo source archives to code, tests, examples, dogfood apps, docs
  and licenses.
- [x] Configure Linux, Windows and macOS checks, plus Rust 1.74 compile checks.
- [x] Configure manual candidate archives, SHA-256 checksums and commit metadata.
- [x] Configure SHA-256 checksums and isolated offline installation checks for
  downloaded Cargo source archives.
- [x] Remove the Task Manager's three-task ceiling with existing general-purpose
  text operations; validate 25-task workflows and existing state-file migration
  in both engines.
- [x] Support one-command Task Manager operations and approved ADR 0019
  application exit status; verify nonzero failures in both engines.
- [ ] Verify Task Manager behavior, persistence, size bounds and failures in both engines
  from the exact release artifact.
- [x] Require MSRV success before candidate jobs and smoke-test extracted archives
  for exact version/greeting, source JSON diagnostics, generated JavaScript and
  the packaged Task Manager's persistent workflow.
- [ ] Run the updated CI matrix successfully on the exact candidate commit.
- [ ] Verify Rust 1.74 on that commit; a configured job is not evidence of success.
- [ ] Run the manual Release candidate verification workflow on that commit.
- [ ] Extract each platform archive and run the included hello.svr and
  task-manager.svr on that platform.
- [ ] Review and commit the accumulated working-tree changes before tagging.
- [x] Resolve version/history: Cargo now declares 1.0.0 and the changelog
  includes the initial public v1.0.0 entry. This release metadata is aligned
  with the current package identity and no existing published v1.0.0 tag was
  reused.
- [x] Confirm public repository identity: Cargo points to the checked-out
  remote repository at https://github.com/peterbwire/Sovra.git and the package
  metadata now matches that public repository identity. Release links should use
  the same remote URL.
- [ ] Confirm the intended distribution channel and ownership. GitHub binaries
  and crates.io toolchain distribution are distinct from a Sovra library registry.
- [ ] Approve the concrete version, notes and artifacts before publication.

## Candidate verification

Local preparation evidence (2026-09-27): 192 tests executed with strict CLI
execution enabled, formatting/Clippy/doc-test stages passed, and Cargo verified
an offline source package using `--allow-dirty`. The optimized Windows GNU binary
and packaging script were rehearsed locally; the resulting archive was extracted
and its bundled hello-world program ran. This does not verify Windows MSVC,
Linux/macOS, Rust 1.74, or GitHub-hosted workflow behavior. No local tags were
present; remote release history has not been verified. Session attempts to read
GitHub metadata/releases were unsuccessful; absence of results is not evidence
that the repository or releases do not exist.

The expanded archive smoke checks also passed locally: exact packaged version,
interpreter output, JSON schema/success and JavaScript execution output. Hosted
jobs must repeat them on the final committed revision. The candidate workflow
now waits for its own Rust 1.74 job before creating platform artifacts.
On 2026-10-09 a local optimized Windows archive rehearsal also passed the
Task Manager persistence runner against its extracted compiler and source,
including interpreter/JavaScript parity, migration, corruption and size bounds.
It is not evidence for the exact final commit or the other hosts.
The candidate workflow now extracts into the system temporary directory and
runs Task Manager persistence with the extracted source as its working directory;
this prevents repository-relative files from silently supplying runtime inputs.
Local isolated copies of both the debug and optimized release compiler with
the source passed the same persistence suite in both engines after this change.
Hosted qualification remains pending.
The persistence runner now covers 1,000-task restart/edit/delete workflows,
late duplicate corruption and signed-64-bit ID exhaustion. A locally created
Windows tar archive of the current optimized compiler and bundled Task Manager
passed that expanded suite after checksum verification and extraction outside
the repository. The final committed workflow artifacts are still unqualified.
The same extracted-archive checks now live in
`scripts/verify-candidate-archive.mjs` and are invoked by the manual candidate
workflow. A local Windows archive built with the current optimized binary and
Task Manager source passed this verifier. It validates the checksum sidecar,
bundled files, hello-world, JSON diagnostics, generated JavaScript and the full
Task Manager persistence runner; the final hosted artifacts still need review.
The verifier also invokes the extracted binary through a temporary PATH from
an unrelated working directory and checks relative task-file placement. This
is a portable-use smoke test, not a clean-machine installation or upgrade test.
The manual workflow also downloads each uploaded candidate, checks its
commit/target/version sidecar against the job, and reruns the verifier on the
downloaded archive. This closes the workflow's pre-upload-only evidence gap
once the hosted jobs actually pass; no hosted run is claimed here.
It also checksums the Cargo `.crate`, installs the downloaded source package
offline in a fresh temporary prefix, and verifies a new Task Manager file plus
legacy-file migration. A local Windows `--allow-dirty` source-package rehearsal
passed; the hosted final-commit and other-platform runs remain open.
At local commit `23171d5`, Windows GNU Rust 1.98.1 passed formatting, strict
Clippy, 353 unit tests, 45 CLI tests, dogfood and Task Manager persistence;
the installed Windows MSVC toolchain passed `cargo check --locked --all-targets`.
Local MSVC linking was unavailable because `link.exe` is missing; hosted Windows
build and execution evidence remains required.
Rust 1.74 was not installed locally. Remote status could not be verified from
this environment because Git lacked credentials and the web view did not expose
that commit's workflow. These local checks do not close hosted gates.

Node 22 is used by CI. Local verification requires Rust and Node; the released
interpreter binary does not require Node. JavaScript output requires Node or a
compatible JavaScript runtime. For release checks, set
`SOVRA_REQUIRE_CLI_EXECUTION=1`; this makes OS-blocked CLI launches fail instead
of returning early. Never bypass OS policy or count skipped checks as passing.

```text
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cargo test --locked --doc
cargo package --locked
cargo build --locked --release
```

The manual workflow only uploads candidate artifacts. It does not tag, create a
GitHub release, publish to crates.io, sign binaries or change repository settings.
Archives include the host target triple in their name; only successfully tested
targets should be advertised. Checksums detect corruption, not publisher identity.
Signing/notarization is not currently implemented. The source package is verified
by Cargo; CI and release checks must run from a clean committed tree. Local
`--allow-dirty` package verification is preliminary evidence only.

## Scope limits

The current implementation is not a production application platform. Fielddesk cannot run. Full application
typing, service implementations and runtime support remain incomplete.
Records, arrays, control flow and local package resolution are implemented.
Sovra-native tests, locked registry resolution/publishing, formatter,
language server and REPL are not implemented. Do not market reserved CLI commands
as functionality. Preserve milestone numbering and the published JSON version-one
compatibility rules.
