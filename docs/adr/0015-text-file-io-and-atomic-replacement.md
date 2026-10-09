# ADR 0015: Text-file I/O and atomic replacement

Status: Accepted and implemented (2026-10-09). Builds on approved
[ADR 0014 Stage 1](0014-cli-input-and-io-foundation.md), whose input and output
APIs are implemented. This decision applies to general Sovra programs, including
the task manager and future file processor; no application-specific host calls.

## Problem

Sovra programs can now receive process arguments and stdin, but cannot persist
state or read a text file. A one-shot task manager loses tasks at process exit.
Directly truncating an existing file before a write finishes can also destroy
the last good state. Both Rust interpreter and generated Node.js need the same
source API, result types and failure categories.

## Decision: Option A, bounded UTF-8 operations with replacement

Add two Rust-owned standard-library functions:

```svr
let loaded: std::TextRead = std::read_text("tasks.txt")
if (loaded.ok) { print(loaded.text) }

let saved: std::TextWrite = std::write_text("tasks.txt", "content")
if (saved.ok == false) { print(saved.error) }
```

`std::TextRead` is a public nominal record with `ok: Bool`, `text: String`, and
`error: String`. `std::TextWrite` has `ok: Bool` and `error: String`. On success,
`error` is empty. On failure, `ok` is false, returned text is empty and `error`
is a stable category: `not_found`, `permission_denied`, `invalid_path`,
`invalid_utf8`, `too_large`, or `io`. A successful empty file is distinguishable
from failure. Invalid UTF-8 is a read failure, never lossy conversion.

Paths are host-native and relative paths resolve against the process working
directory. No sandbox or implicit project-root chdir is introduced. Empty paths,
NUL and platform-invalid path encodings return `invalid_path`. Parent directories
must already exist. Symlinks follow host behavior; the API makes no security
boundary. Reads and writes are limited to **16 MiB of UTF-8 bytes** per call.

`std::write_text` writes a new, uniquely created file in the same directory,
flushes and syncs it, then renames it over the destination. A failure before the
rename leaves an existing destination intact. Temporary files are removed on a
handled failure. If the host cannot replace the destination with a rename, the
call returns `io`; it does not delete the old file and retry. This guarantees
whole-file replacement to other readers on supported filesystems under normal
operation. It does **not** promise durability across sudden power loss or across
unsupported/network filesystems. Neither function creates directories.

No open handles, append, locks, transactions, binary files or async I/O are added.
The task manager will eventually store a versioned line format in a single text
file. This API alone does **not** complete that application: Sovra still lacks
general text splitting and fallible numeric parsing needed to decode the file.
Those pure string/number APIs need a follow-up design before persistence is
claimed. The file format belongs in the app; the compiler knows only UTF-8 files.

## Alternative Option B: direct overwrite

Use the same API but truncate and write the destination directly. This is easier
to implement, but a write error may leave a partially updated task file. The
task manager cannot claim durable state under this option without another
recovery protocol. It is not recommended for the first production-grade file API.

## Implementation and acceptance gate

The standard function registry, semantic checker, application checker, IR call
execution, interpreter host and generated Node.js backend must agree. Tests must
cover empty/missing/Unicode/invalid-UTF-8 files, relative paths, failed parent
directories, size boundaries, read-only/permission cases where reliably testable,
existing-file replacement, and errors after prior printed output. Run equivalent
subprocess tests in both backends and on Windows, Linux and macOS. Fault injection
must verify an existing destination remains intact on a pre-rename failure.
The first file-processor dogfood slice can exercise these APIs immediately.
Task-manager restart and malformed-storage tests become required when the
separately designed parsing helpers and persistence implementation land. Preserve
the existing batch and stdin transcripts throughout.

## Approval boundary

The user approved ADR 0015 Option A on 2026-10-09, after Stage 1 of ADR 0014.
AGENTS.md requires documenting
and pausing before fundamental type-semantics and compatibility decisions. These
new nominal result records, stable error categories and replacement guarantees
are public contracts. Approving this ADR authorizes general text-file operations;
it does not decide text parsing, binary I/O, networking or a broader resource/
ownership model. It removes a necessary file-I/O blocker without falsely marking
the task manager persistent.
