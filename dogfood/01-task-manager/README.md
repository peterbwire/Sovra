# 01 — Task Manager CLI

Compiled desktop lifecycle checkpoint (2026-10-23): the private C#/WPF host
installs a Windows process job before launching Sovra. Normal exit or forced
termination kills descendants; inability to install protection is a startup
error. This complements profile ownership without changing task operations or
storage syntax. Private per-invocation workers also clean up descendants on
timeout or transport failure without closing the GUI. Transaction rollback
remains separate work.

Approved [ADR 0034](../../docs/adr/0034-conditional-text-file-replacement.md) is
implemented. Current task, filter and page saves use `std::write_text_if` with
the original snapshot, rejecting stale or busy saves without automatic replay.
`node scripts/test-conditional-text-files.mjs` tests both engines, mixed pairs
and application rejection paths. The unflagged write-conflict probe deliberately
continues to show why unconditional writes do not coordinate.

A crash or failed cleanup can leave `<file>.svr-write-lock`. Stop **all** writers,
inspect your data, then deliberately remove the retained marker before retrying.
Never remove a marker by age alone or while a writer might still be active.
Keep state in a private local directory. Old application binaries, external
editors and unconditional writes must not modify it concurrently. Symlink
destinations are rejected by conditional writes. Multi-file updates are not
transactions and committed data is not rolled back on cancellation.

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
node scripts/test-task-gui.mjs
node scripts/test-gui-bridge.mjs
node scripts/test-ui-prototype.mjs
```

## Local browser GUI

With Node.js 22 or newer and a built Sovra compiler, run from the repository:

```text
node dogfood/01-task-manager/gui/server.mjs --state tasks.txt
```

Open the loopback URL printed by the server (default
`http://127.0.0.1:8787/`). The page adds, completes and deletes tasks using the
same Sovra program and task file as the CLI. By default, the server runs
`svr build --emit js` once at startup, then executes the **Sovra-generated**
JavaScript for list/add/complete/delete. Node hosts HTTP and HTML, while a
private `gui-list` stream from Sovra supplies the view rows; Node does not
parse the task file. Titles containing legacy terminal controls display in
printable notation and are read-only in the GUI; use the CLI for exact edits.
Pass
`--engine interpreter` to execute those operations through
`svr run` instead; the automated GUI test checks both engines produce the same
persisted file. The adjacent `bridge.mjs` module also runs unrelated Sovra
programs and is tested separately. Stop the server with Ctrl+C.

The private `/ui` route is the first Sovra-owned component prototype. Its task
heading, count, rows, Add form, Complete/Delete actions and notices come from
`main.svr`; the Node `ui-view.mjs` module validates and renders generic
heading/text/button/input nodes. Events call Sovra's `ui-event` path and update
the same task file. The normal `/` GUI remains the production-facing interface.
The prototype has a 64 KiB/256-node view bound and no published UI protocol;
large task files should use `/` or the CLI. `/ui` identifies row actions by
stable task ID, so it can delete legacy control-bearing titles without passing
raw titles through browser forms.

Pass `--port 0` to choose a free port, or `--compiler PATH` and `--source PATH`
when running outside the repository. From an extracted release candidate:

```text
node task-manager-gui.mjs --compiler ./svr --source ./task-manager.svr --state tasks.txt
```

On Windows use `./svr.exe` for the compiler. The GUI is a Node-hosted local
browser adapter; it does not imply a Sovra-native HTTP or desktop runtime.
Keep `bridge.mjs` and `ui-view.mjs` beside `task-manager-gui.mjs` when copying files out of the
candidate archive.
It listens only on `127.0.0.1`, uses a per-process form token, and does not
expose a network service. The CLI's single-writer guidance still applies if
another process edits the same task file. The GUI requires Node; the CLI does
not.

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
are sensitive. On Unix, writes request owner-only `0600` mode; on Windows,
custom ACLs are not preserved. A symlink path can be replaced rather than
updating its target. `list` does not create a missing
file. Successful commands exit 0;
argument, validation and I/O errors exit 1. An interactive session exits 1 if
any command failed. New titles cannot contain LF, CR or other C0/DEL/C1 terminal
controls. LF and CR conflict with the line-oriented storage format; other
controls are rejected to keep newly added titles safe for terminal display.
Older files containing control-bearing titles remain readable, and `list`
prints those controls as visible `\u{XXXX}` text without rewriting the file.
Error messages escape untrusted title and command text too. Exact title lookup
still uses the original text stored in the file.
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
# Desktop UI development

The internal desktop packager's `-CompiledHost` switch now builds a direct
C#/WPF host. Its application runtime uses no PowerShell, Node, Rust or Sovra
compiler; compatible Windows x64 and .NET/WPF remain required. Sovra continues
to define the view and execute task operations. Existing profile IDs and state
paths are preserved. The older adapter remains available during qualification;
see the [compiled adapter](../../toolchain/ui/windows/compiled/README.md).

Opt-in grouped desktop packages use Sovra's private `ui-layout-view` command to
emit an input card, task-list card and wrapping action row per task. The older
`ui-view` stream remains available unchanged for existing desktop/browser hosts.
Layout is defined in `main.svr`, not inferred from task text by the renderer.

The active UI work uses a private reusable Windows desktop renderer at
[`toolchain/ui/windows`](../../toolchain/ui/windows/README.md). It creates native
WPF controls from this application's existing Sovra `ui-view` and `ui-event`
functions. It does not open a browser or put task rules in the renderer.
The same controls/events are tested with an unrelated Sovra component.

For a packaged desktop build, double-click **Sovra UI.exe**, keeping its entire
extracted folder together. `application.exe` is the underlying CLI and does not
open a window on its own. The desktop profile uses
`%LOCALAPPDATA%\Sovra\Desktop\task-manager\state.txt`; CLI state files are left
untouched. See the adapter README for dependencies and private-prototype limits.
## Desktop task editing

The grouped desktop view also offers All/Open/Completed filters and a submitted,
case-sensitive substring search over original task titles. Search and status filters
combine; Clear search keeps the current status filter. An empty query matches all
titles. These actions never rewrite task data. Preferences are stored separately
in `<task-path>.view`, shared by windows using that path and retained across restart.
Invalid/unreadable preferences produce an error and are never silently overwritten;
with the app closed, move that sidecar aside to reset preferences. The task file
and existing flat browser/CLI interfaces do not depend on it. Filtering out an edit
field discards its draft under the renderer's existing removed-control policy.

The private grouped desktop view includes a new-title field and Save action for
each task. Enter a replacement title and select Save (or press Enter). Escape
in a focused field cancels its unsaved draft locally; it never changes storage.
Renaming preserves the task ID, completion status and next ID. Empty, duplicate,
multiline and terminal-control titles are rejected, leaving storage unchanged.
Validation errors appear in the status region and retain the draft for correction.
Existing CLI and flat browser views retain their current interface.

## Bounded desktop pages

Grouped desktop views show at most 25 matching tasks per page. Previous/Next
controls and page labels come from Sovra, using the same reusable host nodes.
Navigation persists across restart in `<task-path>.page`; it never rewrites task
rows or IDs. The existing task and `.view` formats are unchanged. Search/filter
changes reset to page one, and deleted/externally removed rows clamp the selected
page to the remaining range. Empty matches show page one of one. Unavailable page
actions and unexpected values are rejected. Leaving a page discards drafts for
its removed task controls; the Add input retains its draft and focus.

To preserve the renderer's 64 KiB/256-leaf bounds, grouped task titles and search
summaries display bounded text. Values longer than 512 UTF-8 bytes (including
terminal escaping) show a length notice instead of the full value. Stored title
bytes, exact lookup, search matching and CLI/flat output are unchanged; use the
CLI to read an omitted full title. This introduces no substring builtin or
renderer-specific task behavior.

The `.page` file uses private `SVR-TASK-PAGE-1` metadata. A missing file starts
on the first page; invalid/unreadable metadata reports an error without replacing
it. With the app closed, move the affected preference sidecar aside to reset it.
Filter/search preferences and navigation reset use separate atomic writes;
a write failure may leave one preference updated, and is reported. These writes
do not change task data and are not a multi-file transaction. Existing single-writer
limitations remain.

The compiled desktop host now rejects a second cooperating instance for the same
profile using a lifetime file handle. This does not block a CLI, browser or older
desktop host from writing that state path. Keep one writer across those entry
points. Current Task Manager programs additionally coordinate conditional saves;
old versions do not. The compiled host's Windows jobs terminate application
descendants on host exit and invocation cancellation. See the compiled adapter
README for the ownership boundary and retained-marker recovery policy.

Run `node scripts/test-task-pagination.mjs` to check the original 62-task failure,
1,000-task bounds, filtered navigation, restart, page clamping, long stored text,
corrupt metadata and interpreter/JavaScript parity.
