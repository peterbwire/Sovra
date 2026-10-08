# Dogfood application status

Updated: 2026-10-08. Passing a bounded slice does not establish full application
or version-one readiness. These applications live in the compiler repository so
failures can become compiler regressions without separate repository coordination.

| Application | Status | Verified scope / blockers | Next action |
| --- | --- | --- | --- |
| 01 Task Manager CLI | Partial; batch and interactive execution | Add/list/complete/delete by title, validation, EOF, stable IDs, three slots; both execution engines. No persistence or scalable storage. | Decide Stage 2 file I/O and growable collections, then durable application state. |
| 02 File Processor | Planned | No source-level file I/O or recoverable I/O contract. | Specify file access, encoding, errors and resource limits. |
| 03 HTTP API | Planned | HTTP hosting and application lifecycle absent. | Follow the real HTTP application plan and approved runtime decisions. |
| 04 Database Notes | Planned | Persistence, database drivers and transaction contracts absent. | Build on approved I/O, failure and resource contracts. |
| 05 Package Consumer | Planned as dogfood app | Local dependency consumption already has compiler integration coverage; registry distribution is unfinished. | Add an independently maintained local-library application, then publication/install cases. |
| 06 WebSocket Chat Server | Planned | Networking, concurrency, cancellation and connection lifecycle absent. | Design on the HTTP/runtime foundation. |
| 07 SovraBoard | Planned | Requires the preceding application and ecosystem capabilities. | Define end-to-end acceptance after foundational applications run. |

## Capability assessment

The Rust compiler implements lexer, parser, semantic analysis, IR, interpreter and
JavaScript emission. Executable sources support typed functions, nominal records,
aliases, mutable locals, arrays, conditional/loop control flow and scalar/string
operations. The stdlib registry exposes print/println, string length, conversion,
program arguments and UTF-8 line input (ADR 0014 Stage 1).
The CLI runs source files and emits IR/JavaScript; source checking is used here,
not the separate partial application wiring scanner. Rust unit and subprocess CLI
tests remain separate from dogfood. `svr test` remains reserved.

There is no filesystem API, dynamic collection API,
HTTP/database/WebSocket runtime, or completed registry distribution. The initial
task manager therefore uses bounded record storage. It accepts interactive
commands but does not preserve them across processes. The compiler changes are
general process I/O builtins, not application-specific shortcuts.

## Approval boundaries

ADR 0014 Stage 1 is accepted and implemented: argument delimiter, explicit EOF
record, UTF-8 input and immediate printed output. Stage 2 remains proposed.
File permissions, atomic replacement, persistence errors and limits have a
concrete proposal in [ADR 0015](adr/0015-text-file-io-and-atomic-replacement.md).
File I/O alone will not decode task state: text splitting and fallible numeric
parsing also need an approved general-purpose design before persistence is claimed.
ADR 0012 remains Proposed: compound copy/equality semantics must be approved before
relying on mutable collection copies. The current immutable record reconstruction
does not select that policy. Dynamic collections need a separate API proposal.

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
The batch workflow covers 29 output lines. Additional interactive cases cover
commands, EOF and Unicode. `node scripts/test-host-input.mjs` checks process
arguments, invalid UTF-8, long lines and prompt visibility in both backends.
Local Windows execution passes both backends; hosted CI results must
be observed after these changes are pushed, not assumed from workflow configuration.
