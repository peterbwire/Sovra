# 01 — Task Manager CLI

Status: **Partial**. Batch and interactive in-memory workflows execute in both
engines. Persistence and scalable task storage remain unfinished.

```text
cargo run -- run dogfood/01-task-manager/main.svr
cargo run -- run dogfood/01-task-manager/main.svr -- interactive
node scripts/test-dogfood.mjs
```

`main.svr` contains the application and a deterministic batch in `main`. Passing
`interactive` after `--` enters a command loop. It creates, lists, completes
and deletes tasks. Type `add`, `complete` or `delete` on one line and the exact
title on the next; `list` and `quit` need no second line. Helpers accept ordinary
typed arguments. Data is discarded when the process exits.

Application rules for this bounded slice:

- Capacity is three active tasks. Empty titles and overflow leave state unchanged.
- IDs increase monotonically; deletion frees a slot but does not recycle IDs.
- Titles are unique among active tasks so interactive title lookup is exact.
- Completion is idempotent. Missing/nonpositive IDs report an error and preserve state.
- Listing uses storage-slot order, not ID order, and includes status and count.
- Operations return replacement records; no array-copy, record-equality or shared
  mutation behavior is assumed. Capacity three is an application limit, not a
  compiler special case.

The batch transcript covers empty state, three additions, rejected overflow,
empty title, Unicode, completion and repeated completion, missing IDs, deletion,
slot reuse with a fresh ID, and deletion of every remaining task. Rejection messages
are application output; this batch intentionally continues and exits successfully.
Interactive cases cover commands, Unicode, duplicate titles, missing titles,
deletion, slot reuse, quit, clean EOF and mid-command EOF. Prompts appear before
the program waits for input.

Current constraints: no persistence, dynamic collection growth or structured
recoverable file I/O errors. Conditions ending in identifiers use parentheses
to avoid the current parser's record-literal ambiguity. See
[DOGFOOD_STATUS](../../docs/DOGFOOD_STATUS.md) for approvals and next actions.
