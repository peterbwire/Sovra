# ADR 0014: General-purpose CLI input and persistence foundation

Status: Stage 1 Accepted (2026-10-08); Stage 2 remains Proposed. The user
explicitly approved Stage 1 including the argument delimiter, explicit EOF
record and immediate output flushing. File persistence still needs review.

## Problem and existing behavior

The executable Sovra compiler has `String`, `Int`, records, functions, loops and
`print`, but programs cannot receive command-line arguments or stdin, and cannot
read or write files. `svr run <source>` consumes exactly one source path. The Rust
interpreter and JavaScript emitter both buffer Sovra `print` output; the Rust CLI
prints the collected lines only after successful execution. An interactive prompt
would therefore need a deliberate output and error contract. The repository-owned
task manager currently runs a fixed in-memory sequence and exits.

This decision is about general host APIs for any Sovra program. No task-manager
operation or storage format is a compiler primitive. The existing bare `print`
alias, source parsing and application syntax stay intact.

## Proposed staged contract

### Stage 1: process arguments and line input

- `svr run <source> -- <arg>...` passes arguments verbatim to the Sovra program.
  The delimiter separates source options from program data. The generated JS
  program uses its ordinary Node command-line arguments, so backend behavior is
  comparable. `svr build` and `svr check` do not execute or accept program data.
- `std::arg_count() -> Int` counts program arguments after `--` and
  `std::arg(index: Int) -> String` reads a zero-based argument. Invalid indices
  cause a defined runtime error in both engines; count is bounded by the host's
  process argument capacity. These names are Rust-owned standard-library entries.
- `std::read_line() -> InputLine` reads one UTF-8 line from stdin, removing one
  trailing LF and optional preceding CR. `InputLine` is a public nominal standard
  record with `eof: Bool` and `text: String`. An empty entered line is
  `{ eof: false, text: "" }`; EOF is `{ eof: true, text: "" }`. Invalid UTF-8,
  read failures and excessive line length are runtime errors with matching
  interpreter/JS categories. The initial line-size limit is **1 MiB of UTF-8**.
- Interactive output is emitted and flushed when `print` executes, in both
  engines. Earlier printed lines remain visible if a later read or computation
  fails. This changes the current CLI's buffered-output behavior and therefore
  needs explicit compatibility approval. `std::print` and `std::println` continue
  to produce one line per call as currently documented.

Stage 1 does not require a general `Option`, `Result`, file handle, ownership or
async model. The CLI and generated JS each own their process streams; the core
interpreter receives a host interface that tests can replace with deterministic
input. Source checks reject unknown names as before. No silent dynamic values.

### Stage 2: minimal file persistence

- `std::read_text(path: String) -> TextRead` and
  `std::write_text(path: String, contents: String) -> TextWrite` use public nominal
  standard records. `TextRead` has `ok: Bool`, `contents: String`, `error: String`;
  `TextWrite` has `ok: Bool`, `error: String`. File-not-found, permission failure
  and invalid UTF-8 return `ok: false`; other host failures do likewise unless
  resource limits are exceeded. No ambiguous empty-string sentinel.
- Paths resolve relative to the process working directory. This API does not
  provide a sandbox boundary or elevated permissions. Writing replaces the whole
  file. The first implementation must specify an atomicity policy before use for
  durable task storage; partial writes can corrupt data. The recommendation is
  same-directory temporary file, flush, then rename/replace with explicit
  cross-platform failure handling, but the exact durability guarantee requires
  implementation evidence.
- File content uses UTF-8 and a bounded byte count. The initial cap proposed
  for each read/write is **16 MiB**. Existing stdout limits remain separate.

Stage 2 is intentionally separate so argument/input support can be validated
before filesystem behavior is chosen. Binary files, append, directories,
transactions, async networking and persistent handles remain future API design.

## Alternatives and tradeoffs

1. **Recommended:** approve Stage 1 with explicit EOF record and streaming output;
   keep Stage 2 proposed until its atomic-write contract is reviewed. This
   unlocks a genuinely interactive but still in-memory task manager and permits
   backend-parity tests before persistence.
2. Approve arguments only. A one-shot command can receive data, but without
   persistence it cannot manage tasks across invocations. This also avoids the
   current buffered-output compatibility change.
3. Preserve buffered output and add input without prompts. Programs could consume
   piped lines but an interactive CLI would not show a prompt before reading.

## Acceptance tests after approval

The Stage 1 implementation needs source syntax, semantic, IR and both runtime
paths covered with real subprocess tests for Unicode, zero/multiple arguments,
`--` delimiters, empty line versus EOF, CRLF, invalid UTF-8, long input, visible
prompt before input, failure after output and repeatable input in test hosts. The
task manager then needs interactive add/list/complete/delete behavior with
deterministic stdin transcripts. The existing batch dogfood scenario stays as a
regression. Stage 2 needs missing/permission/invalid-encoding tests and evidence
for interrupted and successful writes on all supported platforms before task
persistence is claimed.

## Approval boundary

AGENTS.md says to “Document and pause before fundamental syntax, memory-model,
type-semantics or compatibility decisions.” The new delimiter, nominal standard
records and output timing are public syntax/type/compatibility choices. The
user approved Stage 1 on 2026-10-08. Stage 2 filesystem guarantees and APIs
remain unapproved and must not be inferred from the Stage 1 decision.
