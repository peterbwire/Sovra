# Dogfood application status

Task Manager pagination checkpoint (2026-10-23): the private grouped view emits
25 matching tasks per page with Sovra-owned Previous/Next actions and separate
`.page` state. Bounded long-title/search notices keep output inside existing
renderer limits. Task bytes, old `.view` metadata, CLI and flat view interfaces
are unchanged. The 62-task overflow has a failing-before-fix regression; 1,000
tasks, filtered pages and restart run through both engines and native controls.
See the [application README](../dogfood/01-task-manager/README.md) for limits.

Task Manager search/filter checkpoint (2026-10-20): grouped views use ordinary
inputs/buttons for Sovra-owned matching and persisted private view preferences.
The reusable host renders these controls without application-specific branches.
Case-sensitive search, combined status filtering and clear-search leave task bytes
intact; malformed preferences fail closed. Hidden editors discard drafts according
to existing control-removal rules. Flat browser/CLI interfaces remain independent.

Desktop editing checkpoint (2026-10-20): Task Manager's opt-in grouped view now
provides Save fields using ordinary Sovra rename actions. Validation and storage
remain Sovra-owned; IDs/completion status survive edits. Escape cancels a local
draft without persistence. Two-engine rejection/parity, native editing and visible
packaged keyboard tests pass alongside browser and 18 native CLI regressions.
Reusable capability: generic renderer input submission, failed-draft retention and
Escape cancellation; public UI packaging and clean-host qualification remain open.

Current UI checkpoint (2026-10-15): the user prioritizes Sovra UI in a desktop
window for Task Manager. A private Windows WPF renderer now builds native
controls from the same Sovra view/event code, with real-control event and
persistence tests plus unrelated component reuse. See
[desktop adapter](../toolchain/ui/windows/README.md). The existing browser and
CLI suites remain intact. The bounded flat prototype is not a public framework
or clean-machine-qualified desktop release.

Updated: 2026-10-12. Version one is gated on a production-quality Task Manager
CLI and its requested local browser GUI; later applications validate later versions. Passing the current bounded
slice does not establish version-one readiness. These applications live in the compiler repository so
failures can become compiler regressions without separate repository coordination.
Nine runnable slices now appear in the shared suite: Invoice, Log Report, File
Processor, Task Manager, File Manager, Package Consumer, Sovra Notes, Sovra
Budget and Sovra HTTP Server. Their capability
ladder is documented in `dogfood/README.md`; unnumbered additions do not change
the established seven-application roadmap. New applications must expose a
reusable local package or host adapter, declared in `suite.json` and enforced
by the dogfood runner.

The Task Manager now has a Node 22 loopback browser adapter in
`dogfood/01-task-manager/gui/server.mjs`. It builds the Task Manager to
Sovra-generated JavaScript and executes that code for operations by default;
`svr run` remains an optional parity path. It shares the same file and passes
a local add/list/complete/delete and
request-safety test. This is a host-side GUI, not a Sovra-native HTTP runtime;
hosted and downloaded-artifact qualification remain open.
The adjacent `bridge.mjs` module runs a second, unrelated Sovra program in
both engines, establishing an application-independent execution boundary.
The GUI now renders a private versioned view stream from the Sovra program
instead of opening the task file in Node. Its protocol is internal and not a
general published UI contract.
The private `/ui` route now uses a reusable heading/text/button/input renderer
for Sovra-emitted task rows, Add/Complete/Delete controls and notices. Browser
tests prove these actions change persisted state through both execution engines.
The existing `/` interface remains the full production-facing GUI; the
prototype's 64 KiB/256-node bound makes it unsuitable for large task files.
ADR 0023 remains proposed.

A separate, unnumbered `dogfood/file-manager` browser prototype now lists
regular files through a Node host, renders a Sovra-defined private UI view,
and performs bounded UTF-8 Preview, Copy and empty-file creation through Sovra in both engines.
Its 80-file pages and trusted-workspace rule are documented. It does not renumber the established 01–07
dogfood sequence. Directory enumeration, move and delete are not Sovra APIs;
ADR 0024 proposes a general capability contract before that expansion.

| Application | Status | Verified scope / blockers | Next action |
| --- | --- | --- | --- |
| 01 Task Manager CLI | Partial; batch, interactive, direct commands and file-backed execution | Add/list/complete/delete by title, restart-safe state, validation, EOF, stable IDs and variable-length task rows; both engines. Direct and interactive commands share files and report status 1 on application errors. Interactive commands refresh external changes and reject edits that become stale before save; simultaneous writers remain unsupported. ADR 0021 implements Unix owner-only file mode; Windows custom ACLs and symlink identity are not preserved. Version-one files migrate on write. ADR 0020 preserves duplicate rejection and local Windows benchmark completes 5,000-task list/add/restart-list in under 4 seconds per operation in both engines. A title containing ESC is escaped before display under approved ADR 0022; new control-bearing titles are rejected while legacy bytes and exact lookup are preserved. | Qualify Unix mode behavior and exact hosted artifacts. |
| Invoice CLI | Implemented bounded slice | Validates 1–50 labeled whole-unit amounts, totals through typed records, escapes labels and reports bad input with status 1; both engines. | Extend money semantics only with an explicit currency and rounding design. |
| Log Report CLI | Implemented bounded slice | Streams structured INFO/WARN/ERROR lines, counts levels and rejects malformed lines with status 1; both engines. | Add larger log formats only when streaming and error contracts are specified. |
| 02 File Processor | Partial; streams, bounded copy and column extraction | Numbers/filters stdin lines, copies UTF-8 files, and extracts delimited first columns with validation before replacement in both engines. No binary I/O. | Expand text transforms and design binary/streaming APIs separately. |
| File Manager browser | Partial; trusted local workspace Preview/Copy/Create | Node lists regular files in 80-file pages; Sovra defines private view labels, navigation and actions, and reads, previews, copies and creates empty UTF-8 files. Browser tests cover both engines, empty workspaces, pagination, escaping, CSRF, stable form-to-file mapping, vanished files and path checks. Create rejects existing names in the host but is not exclusive against concurrent writers. No recursive browsing, binary operations, move or delete. | Observe configured cross-platform CI; review ADR 0024 before general directory and file lifecycle APIs. |
| Sovra Notes | Partial; persistent CLI | Separate local library exports note validation, parsing, encoding and complete store validation. CLI supports list/get/put/delete on a versioned bounded UTF-8 file. Both engines pass persistence, Unicode, update, deletion and corrupt-file preservation tests. Single-writer only. | Qualify hosted runs; design transactions only through a separate general decision. |
| Sovra Budget | Implemented bounded numerical CLI | Separate local library exports checked whole-unit evaluation and nominal plan records. Both engines cover within/over budget, truncated percentage and invalid amount cases. No currency or fractional money contract. | Add money semantics only after a currency/rounding design. |
| Sovra HTTP Server | Partial; Node-hosted loopback | A reusable private Node adapter accepts real HTTP requests and calls a Sovra package for routing/response data in both engines. Tests cover routes, Unicode echo, 404, oversized and invalid UTF-8 bodies. No native Sovra socket API, TLS or deployment contract. | Review the general HTTP runtime plan before native networking claims. |
| 03 HTTP API | Planned | Native Sovra HTTP hosting and application lifecycle absent; the unnumbered HTTP dogfood app uses a private Node host. | Follow the real HTTP application plan and approved runtime decisions. |
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
native HTTP/database/WebSocket runtime, or completed registry distribution. The initial
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
The Task Manager persistence runner exercises both engines, including creation
and replacement mode checks under a `022` umask on Unix. Candidate archives and
downloaded Cargo source packages use that runner against their packaged compiler
and application source. Local Windows execution passes; hosted Linux/macOS/Windows
results must be observed after push rather than inferred from CI configuration.
