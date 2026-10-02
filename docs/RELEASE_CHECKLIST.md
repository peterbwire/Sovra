# Production release checklist

Status: **Blocked on production requirements; preview plan superseded.** The user
requires production readiness. Packaging checks remain useful engineering
infrastructure, but cannot authorize a release. All acceptance criteria in
[the production-readiness plan](PRODUCTION_READINESS.md) must be satisfied first.

## Release gates

- [x] Describe implemented versus experimental features in the root README.
- [ ] Replace superseded preview notes with evidence-backed production notes.
- [x] Limit Cargo source archives to code, tests, examples, docs and licenses.
- [x] Configure Linux, Windows and macOS checks, plus Rust 1.74 compile checks.
- [x] Configure manual candidate archives, SHA-256 checksums and commit metadata.
- [x] Require MSRV success before candidate jobs and smoke-test extracted archives
  for exact version/greeting, source JSON diagnostics and generated JavaScript.
- [ ] Run the updated CI matrix successfully on the exact candidate commit.
- [ ] Verify Rust 1.74 on that commit; a configured job is not evidence of success.
- [ ] Run the manual Release candidate verification workflow on that commit.
- [ ] Extract each platform archive and run the included hello.svr on that platform.
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
typing, control flow, structured data, service implementations and runtime support
remain incomplete. Sovra-native tests, package resolution/publishing, formatter,
language server and REPL are not implemented. Do not market reserved CLI commands
as functionality. Preserve milestone numbering and the published JSON version-one
compatibility rules.
