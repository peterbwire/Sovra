# Dogfood application status

Updated: 2026-10-09. Version one is gated on a production-quality Task Manager
CLI; later applications validate later versions. Passing the current bounded
slice does not establish version-one readiness. These applications live in the compiler repository so
failures can become compiler regressions without separate repository coordination.

| Application | Status | Verified scope / blockers | Next action |
| --- | --- | --- | --- |
| 01 Task Manager CLI | Partial; batch, interactive, direct commands and file-backed execution | Add/list/complete/delete by title, restart-safe state, validation, EOF, stable IDs and variable-length task rows; both engines. Direct and interactive commands share files and report status 1 on application errors. Interactive commands refresh external changes and reject edits that become stale before save; simultaneous writers remain unsupported. ADR 0021 implements Unix owner-only file mode; Windows custom ACLs and symlink identity are not preserved. Version-one files migrate on write. ADR 0020 preserves duplicate rejection and local Windows benchmark completes 5,000-task list/add/restart-list in under 4 seconds per operation in both engines. | Qualify Unix mode behavior and exact hosted release artifacts; continue compiler/runtime hardening. |
| 02 File Processor | Partial; streams, bounded copy and column extraction | Numbers/filters stdin lines, copies UTF-8 files, and extracts delimited first columns with validation before replacement in both engines. No binary I/O. | Expand text transforms and design binary/streaming APIs separately. |
| 03 HTTP API | Planned | HTTP hosting and application lifecycle absent. | Follow the real HTTP application plan and approved runtime decisions. |
| 04 Database Notes | Planned | Persistence, database drivers and transaction contracts absent. | Build on approved I/O, failure and resource contracts. |
| 05 Package Consumer | Implemented local-package slice | A separate library exports nominal records and functions; the app constructs records, reads fields and runs two scenarios in both engines. Registry distribution is unfinished. | Add publication/install cases when registry contracts and tooling exist. |
| 06 WebSocket Chat Server | Planned | Networking, concurrency, cancellation and connection lifecycle absent. | Design on the HTTP/runtime foundation. |
| 07 SovraBoard | Planned | Requires the preceding application and ecosystem capabilities. | Define end-to-end acceptance after foundational applications run. |

## Capability assessment

The Rust compiler implements lexer, parser, semantic analysis, IR, interpreter and
JavaScript emission. Executable sources support typed functions, nominal records,
aliases, mutable locals, arrays, conditional/loop control flow and scalar/string
operations. The stdlib registry exposes print/println, string length, conversion,
program arguments, UTF-8 line input (ADR 0014 Stage 1), bounded text files
(ADR 0015), and pure text decoding and line uniqueness (ADRs 0016, 0020).
The CLI runs source files and emits IR/JavaScript; source checking is used here,
not the separate partial application wiring scanner. Rust unit and subprocess CLI
tests remain separate from dogfood. `svr test` remains reserved.

There is no binary/handle-based filesystem API or dynamic collection API,
HTTP/database/WebSocket runtime, or completed registry distribution. The initial
task manager therefore uses application-owned variable-length text records. It accepts interactive
commands and optionally preserves them across processes in a validated versioned
file. The compiler changes are general standard-library operations, not
application-specific shortcuts.

## Approval boundaries

ADR 0014 Stage 1 is accepted and implemented: argument delimiter, explicit EOF
record, UTF-8 input and immediate printed output. ADR 0015 Option A is accepted
and implemented for bounded UTF-8 file reads and replacement writes with result
records and stable error categories.
ADR 0021 is accepted and implemented: Unix text-file writes request owner-only
`0600` mode, including replacements. Windows ACL privacy still depends on the
containing directory.
ADR 0016 is accepted and implemented: pure text splitting and strict, fallible
integer parsing enable application-owned file formats. The Task Manager format
is bounded by the 16 MiB text-file API rather than a fixed task count.
ADR 0012 Option A is accepted and implemented: arrays and records transfer
logical value snapshots, and compound equality is recursive with nominal record
identity. Dynamic collections still need a separate API proposal.

## Findings and regressions

- Initial source parsing exposed the existing ambiguity between unparenthesized
  identifier-ended conditions and record literals. Parenthesized conditions compile
  and execute. This slice does not alter grammar; a minimal parser diagnostic case
  and explicit disambiguation decision are follow-up work, not a claimed bug fix.
- The new standard record initially allowed a source module to redeclare
  `std::InputLine`. A failing semantic regression reproduced the collision;
  E3008 now rejects it. For future bugs: retain the failing application, minimize
  a compiler test, reproduce, fix, and rerun both levels. Never change golden
  output merely to hide a compiler regression.

## Automated acceptance

`cargo build --locked` followed by `node scripts/test-dogfood.mjs` uses the local
binary, checks the source, emits IR/JS and compares actual interpreter and Node
output with a reviewed transcript. Failures are nonzero, including blocked/missing
executables and timeouts. There are no skipped executions. JSON reports and emitted
artifacts are retained under `target/dogfood/run-*`; CI uploads them per platform.
The task-manager batch workflow covers additions, completion and deletion with
reviewed output. Interactive cases cover commands, EOF, Unicode and failure
status. Direct-command acceptance covers independent processes and file-sharing.
`node scripts/test-host-input.mjs` checks process
arguments, invalid UTF-8, long lines and prompt visibility in both backends.
The package-consumer cases verify a direct local dependency, exported record
construction, field access, public operations, a private helper called inside
its package, and immutable record reconstruction across the package boundary.
The stream-processor cases cover Unicode, empty lines, filtering, line numbering
and immediate EOF. Its output counts UTF-8 bytes through the existing `std::len`
contract. A separate differential runner tests its file-copy path in both
engines, including replacement, invalid UTF-8, missing paths and 16 MiB limits.
The same runner tests column extraction with Unicode/multicharacter delimiters,
malformed rows, destination preservation, empty files and application bounds.
Local Windows execution passes; hosted Linux/macOS/Windows CI outcomes must be
observed after push rather than inferred from workflow configuration.
Local Windows execution passes both backends; hosted CI results must
be observed after these changes are pushed, not assumed from workflow configuration.
