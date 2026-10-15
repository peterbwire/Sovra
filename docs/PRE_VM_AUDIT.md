# Pre-VM functionality audit

Date: 2026-10-14. Scope: the current working tree, not a published or immutable
release commit. Existing user changes are retained. This audit combines inspection
of CLI/test dispatch, package compilation, native linking, project creation,
installation and packaging with the repository's executable acceptance suites.
It is not a claim that every possible defect has been found.

## Confirmed issue and fix

The previous Windows ZIP bundled `docs/course/`, contrary to the user's
repository-only course requirement. Added an artifact regression that rejected
the preceding ZIP, then changed the package builder to omit the course before
hashing and archiving. The replacement ZIP passes that regression and the full
installation/self-test. The course remains intact in the main repository.

## Verification

| Area | Local result |
| --- | --- |
| Rust unit, subprocess and example tests | 377 + 49 + 1 passed; zero ignored/failing |
| Strict Clippy and formatting | Passed |
| Rust documentation tests | Completed; no documentation test cases exist |
| Windows MSVC all-target compile check | Passed; execution not established by this check |
| Language-test command | Human/JSON, signatures, testless input, preflight, isolation, failure continuation, templates and dependency packages passed |
| Nine dogfood applications | Source/IR/JS and interpreter/JavaScript behavior passed |
| Process input and text files | UTF-8, EOF, output flushing, bounded I/O and error cases passed |
| Task Manager | Persistence, corruption preservation, browser GUI, controls and CSRF checks passed |
| Reusable GUI/UI and File Manager | Bridge, Sovra component events, renderer validation, path checks and file behavior passed |
| Notes and HTTP | Persistent CRUD/corruption and loopback routing/limits passed |
| Runtime parity | 20 reviewed cases and 131 finite Float values passed |
| Archive safety and compiler nesting | Unsafe archive members rejected; supported limits diagnosed without crashes |
| Relocated native runtime payload | Offline linking, hash checks and source-free execution passed; 18 Task Manager cases matched |
| Installed optimized native CLI | Failure isolation, console, Task Manager and package-consumer builds passed; compiler-free execution passed |
| Native package linking | Private helpers, exported nominal records and Float widening passed after source removal |
| General native runtime | 22 cases covering all 14 registered builtins, compound values, runtime errors and bounded I/O passed |
| Windows toolchain artifact | ZIP checksum/corruption, integrity, no-overwrite installation and restricted-PATH create/check/run/test passed |

## Boundaries for the VM test

The retained toolchain is a locally built x86_64 Windows GNU release-profile
artifact. Hosted CI, current Rust 1.74 verification, clean Windows installation
and minimum Windows/CRT deployment support remain unqualified locally.
Direct PE imports were audited; this does not cover all transitive/dynamic loads.

Checking/running/testing through `svr.exe` need no Node or Rust. Native builds
still need Rust/Cargo and a compatible linker on the developer machine. Native
application execution uses the embedded Rust interpreter and needs no compiler
installation. Browser/HTTP hosts currently require Node. Registry, formatter,
LSP, native desktop UI and WebAssembly remain planned.

Task Manager persistence remains bounded UTF-8 replacement-file storage with
single-writer/private-directory guidance. There is no database transaction or
concurrent-writer guarantee. Language tests have real file effects, unavailable
stdin and no sandbox/rollback/timeouts. Partial application scanning is not full
executable validation. These are documented capabilities, not newly changed rules.

See [clean Windows testing](TOOLCHAIN_CLEAN_WINDOWS_TEST.md) for your next step.

## Retained artifacts for your test

- Toolchain: `target/toolchain/pre-vm-ready/sovra-windows.zip`.
  SHA-256: `77942828ef471e3d98b460e433b0ac6da710ea6d8e754999fc636cb0904dfca3`.
  Exact-archive installation/self-test and course-exclusion checks passed.
- Standalone Task Manager: `target/native-task-manager/pre-vm-audit/sovra-task-manager.zip`.
  SHA-256: `47242e87ea7fa9a9f00e8cf3e3ac302db4ddd32da06cac14bfe2f44cca5408f9`.
  Its exact archive passed restricted-PATH verification and corrupted-download
  rejection. The retained executable passed all 18 interpreter parity cases.
- Native verification evidence:
  `target/native-task-manager/pre-vm-audit/verification.json` explicitly records
  `clean_machine_qualified: false` and the executable/compiler hashes.

These supersede preceding local packages for VM testing. The archive documentation
is a snapshot from packaging; this final evidence is retained in the repository.
