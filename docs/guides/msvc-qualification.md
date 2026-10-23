# Local Windows MSVC qualification

This developer-host check is separate from hosted CI and clean-machine deployment.
It does not change the default Rust toolchain or install prerequisites.

Prerequisites:

- Windows x64, Node.js for backend tests, and the Rust toolchain
  `stable-x86_64-pc-windows-msvc`.
- Visual Studio C++ x64/x86 tools and a compatible Windows SDK. An installed Rust
  MSVC target alone does not provide Microsoft's linker or SDK libraries. See
  [Rust's MSVC prerequisites](https://rust-lang.github.io/rustup/installation/windows-msvc.html).

From the repository root:

```text
node scripts/test-msvc.mjs
```

While an installer is already active, `node scripts/test-msvc.mjs --wait-for-tools`
waits up to one hour for a physical linker, x64 SDK libraries and installer exit,
then runs the same checks once. `readiness.json` records each readiness check.
`verification.json` distinguishes waiting/running from passed/failed; waiting is
not successful qualification. Process enumeration must be permitted by the host.
It fails if the installer exits without the prerequisites, never stops installer
processes, and registers no service or startup task. Installer repair and test
qualification remain separate operations.

The runner uses `target/msvc-qualification` as an isolated Cargo target directory.
It checks Rust's host triple, runs all Rust targets and documentation tests, builds
the compiler, then tests the formatter, nine dogfood applications and structural
depth limits through that executable. CLI execution is required; ignored tests
do not qualify the host. It also builds and executes a standalone console sample
through the public build command, selecting MSVC for its Cargo subprocess without
changing rustup's global default. Each native proof uses a fresh owned output path.
Per-stage logs and `verification.json` record failure as
well as success; the compiler hash binds passing depth results to that binary.
The runner does not build an optimized release ZIP or claim clean-VM support.

If linking fails, inspect `rust-tests.log` and the Visual Studio Installer's
component state. An incomplete instance can be listed as launchable while its
linker or SDK is missing; even a complete flag can precede physical file readiness
during repair. Installer exit 0 alone is not proof that Rust can link.
After a failed component download, verify connectivity and complete/repair the
existing installation before rerunning. Keep installer security checks enabled;
do not bypass Windows Application Control to obtain passing tests.

Hosted evidence for commit `a555cce7640b8b08e67978bf7e55d1a3b1d56c58`:
[Windows MSVC CI](https://github.com/peterbwire/Sovra/actions/runs/39674084497/job/119479249380)
passed 461 Rust tests, documentation tests, strict Clippy and formatter checks.
The job subsequently failed because runtime packaging could not find
`Get-FileHash`; downloaded-artifact jobs were skipped. This is compiler evidence,
not a passing distribution gate or local-machine qualification. Runtime payload
hashing now uses .NET directly, with a regression that removes the optional hash
command; a hosted rerun is still required for the updated packaging code.
