# Version-one delivery gates

Scope revised by the user on 2026-10-09: version one must support a useful,
production-quality Task Manager CLI built in Sovra. Later versions will each
grow the language and platform with their own test application, as capability
allows. The earlier requirement for the full application platform and third-party
library publishing before 1.0.0 is superseded. Status: **not ready for
publication**. Cargo's version is target metadata. M0-M15 retain their existing
numbers and remain roadmap work; unfinished features outside the advertised v1
scope are not v1 release gates.

| Gate | Current state | Required evidence |
| --- | --- | --- |
| Compiler correctness | Partial | Module-local types, iterative alias resolution and approved ADR 0012 compound value parity are implemented. Full retained expression typing, numeric boundaries and malformed-input/IR coverage remain. |
| Task Manager CLI | Partial: batch, interactive, direct commands and restart-safe file state run in both engines; fixed task-count ceiling removed; 1,000-task persistence and corruption checks pass; stale interactive edits are detected; local 5,000-task benchmark completes under 4 seconds per operation | A documented CLI supports add/list/complete/delete across restarts, rejects malformed state without destroying it, handles EOF and I/O failure, and supports a practical number of tasks. Scripted errors use ADR 0019 nonzero status; qualify exact hosted artifacts. Single-writer and private-directory guidance must remain explicit until stronger file APIs exist. |
| Compiler/runtime for advertised scope | Partial | Every language and stdlib feature used by the Task Manager is specified, checked and exercised through interpreter and generated JavaScript. Confirmed mismatches have regressions. Malformed input, overflow, resource limits and failure propagation have reviewed behavior. |
| Local toolchain | Partial; local Windows `.crate` installed offline into an isolated prefix and ran fresh and legacy Task Manager state | A fresh installation can build, check and run the Task Manager without unpublished dependencies or repository-only shortcuts. Commands and supported hosts are documented; exact downloaded packages must pass on supported hosts. |
| Release qualification | Local Windows GNU tests and MSVC compile check pass on committed code; hosted evidence pending | Exact-commit CI on supported platforms and declared minimum Rust version, extracted-artifact Task Manager smoke tests, clean-machine installation/upgrade, compatibility/security review, accurate release notes. |

## Execution order

1. Harden the Task Manager's variable-length text records, which use existing
   general-purpose Sovra text/file operations and require no language-semantic
   decision. Existing version-one task files load and migrate on the next write.
   Exact 16 MiB bounds pass. ADR 0020's general line-uniqueness operation
   preserves duplicate rejection; local Windows benchmark covers 100, 1,000
   and 5,000 rows in both engines. Continue artifact and host qualification.
2. Expand executable dogfood acceptance: many tasks, restart/edit/delete cycles,
   corrupted files, failure paths, and interpreter/JavaScript parity.
3. Harden the compiler, runtime and standard library used by that application;
   preserve the approved general process-status contract in ADR 0019 and record
   any remaining limits honestly in the v1 documentation.
4. Qualify the exact release artifacts and installation path on supported hosts.

HTTP, database, WebSocket, SovraBoard, native `svr test`, full M12-M15
application-platform behavior, and third-party registry publishing remain roadmap
work for later releases. Local packages already work in a bounded form and must
not regress, but registry publication is not a v1 prerequisite.

Fundamental semantics and compatibility decisions follow AGENTS.md; routine
implementation and verification proceed under existing approval. No registry
endpoint, credentials, memory model for resources, scheduler or deployment target
is silently selected by this plan. Publication itself is separate from preparing
reviewable code and artifacts.
