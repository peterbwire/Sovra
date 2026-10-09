# 01 — Task Manager CLI

Status: **Partial**. Batch and interactive workflows execute in both engines.
Optional file-backed state survives restarts. Variable-length task rows remove
the old three-task ceiling without changing language semantics. Current
duplicate checks use the general `std::lines_unique` operation. A local Windows
benchmark covers 5,000-task files in both engines; release qualification remains.

```text
cargo run -- run dogfood/01-task-manager/main.svr
cargo run -- run dogfood/01-task-manager/main.svr -- demo
cargo run -- run dogfood/01-task-manager/main.svr -- interactive
cargo run -- run dogfood/01-task-manager/main.svr -- interactive tasks.txt
cargo run -- run dogfood/01-task-manager/main.svr -- list tasks.txt
cargo run -- run dogfood/01-task-manager/main.svr -- add tasks.txt "Write tests"
cargo run -- run dogfood/01-task-manager/main.svr -- complete tasks.txt "Write tests"
cargo run -- run dogfood/01-task-manager/main.svr -- delete tasks.txt "Write tests"
node scripts/test-dogfood.mjs
node scripts/test-task-persistence.mjs
```

For a candidate archive extracted into the current directory, run
`./svr run task-manager.svr -- interactive tasks.txt` on Unix or
`.\svr.exe run task-manager.svr -- interactive tasks.txt` on Windows.
The optional interactive path stores task state; omit it for an in-memory
session. Direct `list`, `add`, `complete` and `delete` commands require a
path. `add`, `complete` and `delete` also require an exact title. For example,
`./svr run task-manager.svr -- add tasks.txt "Write tests"`.

`main.svr` shows usage with no program arguments; `-- help` does the same.
The deterministic batch is available under `-- demo`. Passing
`interactive` after `--` enters a command loop. It creates, lists, completes
and deletes tasks. Type `add`, `complete` or `delete` on one line and the exact
title on the next; `list` and `quit` need no second line. Helpers accept ordinary
typed arguments. Supplying a path loads the versioned UTF-8 task file, then
atomically replaces it after each successful change. A missing file starts empty;
malformed files and I/O errors are reported without overwriting existing data.
Direct commands use the same file and can be interleaved with interactive
sessions. Interactive commands refresh the file before acting, so a later `list`
shows changes saved by another process. If the file changes during an edit,
the pending save reports `error: task file changed`, exits 1 and preserves the
newer file. This detects stale sessions but is not a multi-writer transaction:
separate processes can still race between the comparison and replacement. Use
one writer per task file. Use a regular file in a private directory when titles
are sensitive; replacement may change file permissions, and a symlink path can
be replaced rather than updating its target. `list` does not create a missing
file. Successful commands exit 0;
argument, validation and I/O errors exit 1. An interactive session exits 1 if
any command failed. Direct titles cannot contain LF or CR because the storage
format is line-oriented.
Without a path, data is discarded when the process exits.

Application rules for this bounded slice:

- Empty and duplicate titles leave state unchanged. The persistent UTF-8 file is
  limited to 16 MiB by the general text-file API.
- IDs increase monotonically; deletion frees a slot but does not recycle IDs.
- Once the next ID reaches the signed 64-bit maximum, add fails without changing
  the file; existing tasks can still be listed, completed and deleted.
- Titles are unique among active tasks so interactive title lookup is exact.
- Completion is idempotent. Missing/nonpositive IDs report an error and preserve state.
- Listing uses insertion order and includes status and count.
- Operations return replacement records; task rows use standard strings and
  `std::split_once`, with no compiler special case. Existing `SVR-TASKS-1`
  three-slot files load and migrate to `SVR-TASKS-2` after a successful edit.

The batch transcript covers empty state, four additions,
empty title, Unicode, completion and repeated completion, missing IDs, deletion,
further addition with a fresh ID, and deletion of every remaining task. Rejection messages
are application output; this batch intentionally continues and exits successfully.
Interactive cases cover commands, Unicode, duplicate titles, missing titles,
deletion, slot reuse, quit, clean EOF and mid-command EOF. Prompts appear before
the program waits for input.

Current constraints: state decoding and rewriting still copy strings as files
grow; there is no binary file support. Success messages follow successful persistence,
and a failed save does not claim the task was added. Conditions ending in identifiers use parentheses
to avoid the current parser's record-literal ambiguity. See
[DOGFOOD_STATUS](../../docs/DOGFOOD_STATUS.md) for approvals and next actions.
