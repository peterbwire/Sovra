# Version-one implementation handoff

Optimized toolchain candidate (2026-10-23):
`target/toolchain/conditional-write-release-v1.zip` contains the release-profile
compiler, matching runtime source and stronger shipped installation self-tests.
Its installation and conditional Task Manager checks pass locally with developer
tools removed from PATH. The desktop candidate below is a separate artifact;
neither local package is clean-VM or publication approval.

Approved [ADR 0034](adr/0034-conditional-text-file-replacement.md) is implemented:
conditional text replacement coordinates current Task Manager saves in both
engines and native execution. Eight same/mixed-engine snapshot schedules reject
stale saves. Retained crash markers fail closed; recovery requires stopping all
writers first. Unconditional writes and older application binaries remain outside
the guarantee. See the development log for executed checks and artifact identity.

Compiled desktop candidate (2026-10-23): the internal packager's `-CompiledHost`
option produces a C#/WPF `Sovra UI.exe` that owns its window and invokes the
native Sovra application directly, with no PowerShell runtime scripts. Local
qualification now passes 281 checks including native events, keyboard/layout
behavior, bounded 1,000-task pagination, task editing/filtering, startup failures,
responsiveness, duplicate-profile rejection and restart persistence;
the baseline adapter also passes its retained tests. This is a private Windows
candidate requiring compatible .NET/WPF, not a clean-VM or release approval.
See [ADR 0033](adr/0033-compiled-desktop-host.md) and the latest development log
for artifact paths and actual validation. The historical hashes below remain
historical.

The current candidate is `target/compiled-desktop/task-manager-compiled-v11.zip`.
Its lease protects cooperating compiled desktop hosts; a private Windows job
terminates descendants on host exit. Private invocation workers now clean up
descendants after timeouts, transport failures and application completion while
the GUI remains alive. Current Task Manager programs additionally use conditional
writes across hosts. Older/noncooperating writers and atomic writer handoff
remain separate; cancellation does not roll back writes already committed.
See the latest development-log entry for completed checks and artifact identity.
Repository cleanup retained the earlier v4 desktop candidate and the latest
`target/toolchain/project-rollback-v1` toolchain. Older generated artifacts were
removed; historical paths/hashes below describe past evidence, not a promise
that every local build copy remains available. See the cleanup entry in the
development log and `target/repository-cleanup/result.json`.

Current tooling checkpoint (2026-10-21): `svr doctor` and standalone `svr fmt`
are implemented after the historical artifacts below. Formatter writes are
enabled on Windows; package formatting and other-host writes remain deferred.
See [development log](DEVELOPMENT_LOG.md) for current artifact/test evidence;
earlier hashes do not describe the new commands. Clean-VM and hosted qualification
remain external release gates.

Date: 2026-10-14. Status: **Implemented and verified locally; publication pending
external qualification.** The scope is a useful Task Manager CLI and an
installable developer toolchain, as revised by the user. GUI/platform/registry
work outside that scope remains on the existing roadmap.

## Completed local implementation

- `svr new`, source/package checking and execution, IR/JavaScript output and
  accepted ADR 0027 host-native `build --emit exe --output PATH`.
- Trusted Rust interpreter/stdlib embedded in standalone application binaries.
  All 14 current builtins have native conformance cases; existing value isolation,
  nominal identity and typed parameters retain their approved semantics.
- Installed runtime snapshot validation: version, complete file list, hashes,
  safe relative paths, file size/depth/count bounds and link rejection.
- Build failures leave existing output untouched. Linking uses owned staging
  and atomic no-overwrite publication; installed copies cannot fall back to a
  developer checkout when their runtime payload is missing.
- Windows installer/PATH setup, templates, documentation, examples, Task Manager
  source, runtime payload, package integrity and package-contained self-tests.
- Standalone Task Manager archive, binary/ZIP checksums, direct PE import record,
  compiler/source provenance and a package-contained application self-test.

## Verification evidence

The final Rust suite reports **367 unit tests, 48 CLI tests and one example test**
passing. Strict Clippy and formatting pass. The final review added a regression
and fix for the flat-directory runtime file-count boundary.

The general native suite exercises 22 cases and every registered builtin.
Local-package tests cover exported nominal records, private helpers, qualified
signatures and Float widening. The nine dogfood slices and existing interpreter/
JavaScript tests retain their passing local baseline.

Optimized installed acceptance builds the template, Task Manager and package
consumer outside the checkout, then removes installation/source dependencies
before launching. Native Task Manager parity checks 18 command/interactive cases,
output, exit status, corruption preservation and persisted bytes. These checks
are automated in CI, whose exact final-source hosted results remain pending.
The standalone archive has a separate downloaded-artifact job. It runs without
developer tools on PATH, uploads a versioned JSON verification report and tests
corrupted-download rejection. This strengthens hosted evidence while preserving
the separate clean-machine requirement.

Local host: Windows x86_64 GNU, Rust/Cargo 1.98.1. This is not evidence for
minimum Windows, MSVC, minimum Rust or every host target.

## Exact final artifacts

- `target/toolchain/final/sovra-windows.zip`
- `target/toolchain/final/sovra-windows.zip.sha256`
- `target/native-task-manager/final/sovra-task-manager.zip`
- `target/native-task-manager/final/sovra-task-manager.zip.sha256`

Use each ZIP with its matching checksum sidecar. Preserve Task Manager
`build.json`, `imports.json` and binary checksum with the VM results. These
qualification artifacts are not a published release. Earlier retained artifacts
remain historical evidence and should not replace the final files above.

Verified final ZIP SHA-256 values:

```text
cf5224837216b46871c7e8731b0464f0c5c7a4cd34850d6372fd1d69225a7cb2  sovra-windows.zip
0fe10af6658b6649cfb707764045777f2c8812d3f7347ab1dffc80fdc833fc00  sovra-task-manager.zip
```

The compiler hash recorded in Task Manager `build.json` is
`f2bbb4eae43f94aa836bfa334b147e27da8b4e4c246e4df0d1662d198b42d612`.
The code revision present at handoff is
`da5eed0e6d3f3ce1c78439f9f375e7b3d9779684`; final documentation edits remain
in the working tree. Artifact hashes identify the exact tested bytes independently
of repository publication or a later documentation commit.

## Remaining release decisions and evidence

1. Run exact final-source hosted CI on Windows/MSVC, Linux, macOS and Rust 1.74.
   Preserve downloaded-artifact checks from that same source revision.
2. Run [clean Windows toolchain acceptance](TOOLCHAIN_CLEAN_WINDOWS_TEST.md)
   and [standalone application acceptance](NATIVE_TASK_MANAGER_VM_TEST.md)
   using matching exact artifacts. The user will perform this VM test.
3. Record the supported Windows version/architecture and runtime DLL requirements
   from actual host/VM evidence; complete the compatibility/security release review.
4. Review final release notes and approve publication separately. Cargo's 1.0.0
   metadata and local passing tests do not grant publication approval.

Developers need Rust/Cargo and a compatible host linker for native builds. End
users need neither those tools nor Node/Sovra for the application executable.
The first public target is host-only and requires a hard-link-capable output
filesystem. Source-level native stack traces, a separate runtime crate, cross
compilation, `svr test`/`fmt`, package locks/registry, native UI and WebAssembly
remain future capabilities, not first-release promises.

## Subsequent tooling checkpoint

ADR 0028 is implemented; current source passes 427 Rust tests and nine dogfood slices. The updated debug package at target/toolchain/language-tests/sovra-windows.zip passes installation verification. Earlier artifact hashes describe the preceding native baseline and do not include this command. Release rebuild and hosted/clean-VM evidence remain pending.

## 2026-10-14: Refresh optimized Windows toolchain after ADR 0028

- Built `cargo build --release --locked`; the optimized compiler passes the
  complete language-test subprocess acceptance suite, including both reusable
  dependency test packages, malformed input, effects and status isolation.
- Packaged target/toolchain/language-tests-release/sovra-windows.zip and verified
  its exact ZIP checksum, corruption rejection, installation/no-overwrite rules,
  restricted-PATH project creation/check/run/test and Task Manager operations.
- ZIP SHA-256: b99feff319aa97ce1b58371fc28a5e752d3d839a316e57c4b65380be27d882cf.
- Compiler SHA-256: ef67d3988c307ee1c85bcdffd6526d252deb8aba49024929a99ea22e666f648e.
- Formatting and diff whitespace checks pass. This is local release-profile
  artifact evidence, not hosted CI or clean-machine deployment qualification.
  The retained standalone Task Manager ZIP still belongs to the earlier baseline.
- Next: run this ZIP on a clean compatible Windows VM and collect hosted CI
  evidence; formatting/registry/editor tools remain planned public contracts.

### Final pre-VM artifact evidence

- target/toolchain/pre-vm-ready/sovra-windows.zip passed exact-archive installation,
  course exclusion and restricted-PATH create/check/run/test. SHA-256:
  77942828ef471e3d98b460e433b0ac6da710ea6d8e754999fc636cb0904dfca3.
- target/native-task-manager/pre-vm-audit/sovra-task-manager.zip passed archive
  verification without developer tools on PATH, damaged-download rejection and
  all 18 retained-executable parity cases. SHA-256:
  47242e87ea7fa9a9f00e8cf3e3ac302db4ddd32da06cac14bfe2f44cca5408f9.
- verification.json records local Windows evidence and clean_machine_qualified
  false. User VM and hosted execution remain next gates, not completed claims.
