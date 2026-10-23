# ADR 0034: Conditional text-file replacement across execution hosts

Status: **Accepted (2026-10-23); implemented in Rust and generated JavaScript.**
The user approved the proposed API and fail-closed recovery policy. Local Windows
execution is recorded in the development log; hosted/other-platform and clean-VM
qualification remain separate.

## Observed gap

ADR 0015 guarantees bounded whole-file replacement, not serialized updates.
Task Manager one-shot and UI operations load a snapshot and later call
`std::write_text`. Interactive edits recheck their snapshot, but a different
writer can still replace the file between that check and the write. The compiled
desktop lease excludes other cooperating desktop windows only. Adding more host
leases would not protect direct interpreter, generated JavaScript or native calls.

`node scripts/probe-text-write-conflict.mjs` deterministically demonstrates the
existing contract through real Sovra execution. Two processes read `base` before
either writes; the first saves `base|first`, then the second successfully replaces
it with `base|second`. Interpreter/interpreter, JavaScript/JavaScript and both mixed
orders reproduce the lost update. Evidence is written to
`target/text-write-conflict/observation.json`, explicitly labelled an observation
of a limitation, not a passing concurrency gate. No compiler bug is claimed: the
existing API never promised compare-and-replace behavior.

## Recommended decision: additive conditional replacement

Add a general-purpose function, preserving existing unconditional `std::write_text`:

```text
std::write_text_if(path: String, expected_exists: Bool,
                   expected_text: String, replacement: String) -> std::TextWrite
```

This is a callable signature description, not new Sovra declaration syntax. Reuse
the existing nominal result record. Existing error categories remain available;
the new function additionally returns `conflict`, `busy` or `invalid_snapshot`.

- Validate inputs before creating a marker or modifying the destination. Both
  text arguments retain the 16 MiB UTF-8 bound. An absent expected file requires
  empty `expected_text`; otherwise return `invalid_snapshot`.
- Exclusively create a sibling `<filename>.svr-write-lock` using create-new
  semantics. An existing marker means `busy`, with no wait or automatic retry.
  Use the same marker spelling and protocol in Rust and generated JavaScript.
  Neither runtime interprets tasks or application profiles.
- While owning the marker, read the destination using bounded strict UTF-8.
  Require an exact existence/content match with the expected snapshot. A mismatch
  returns `conflict` and leaves destination bytes intact. Ordinary read failures
  preserve their existing categories and never trigger an overwrite.
- On a match, use the existing same-directory exclusive temporary file,
  flush/sync and atomic replacement implementation. Preserve Unix owner-only
  creation, Windows directory-security limits and pre-rename failure behavior.
- Release only this call's marker after completion or a handled failure. Never
  remove a marker merely because it exists or because its timestamp is old.
  Replacement success remains success if subsequent marker cleanup fails: it
  means content was committed, not that later writes are guaranteed available.
  A retained marker blocks subsequent conditional writes with `busy`.

## Approved scope and recovery policy

The first slice is cooperative, fail-closed coordination of one regular file in
a private, local directory. Canonicalize its existing parent directory so ordinary
relative paths and symlinked parent aliases share the marker. Reject a symlink
destination or non-regular existing destination as `invalid_path`; do not change
ADR 0015's existing functions or promise filesystem sandboxing. Network shares,
hostile directory mutation, hard-link object identity and noncooperating writes
are outside the guarantee. Locks are per destination, not multi-file transactions.

Process termination can leave the create-new marker behind, including termination
by desktop cancellation. Return `busy` until an operator has stopped every writer,
inspected the state and deliberately removed the marker. Do not infer death from
PIDs, lock age or a timeout. The user accepted this availability tradeoff before
implementation. A marker is not proof that another process is currently alive.
Automatic crash recovery needs a separately reviewed portable locking strategy.

No open language handles, ownership rules, closures, new syntax or dependencies
are needed for this first implementation. A failed comparison is not retried
automatically: application decisions must be recomputed from fresh data.
Unconditional `std::write_text`, old application binaries and external editors
remain noncooperating and can defeat the guarantee. Migrating only the desktop
host is insufficient; do not describe this as universal file protection.

## Implementation and qualification sequence

1. Add the callable registry/type checks and default interpreter host method,
   Rust implementation, IR call support and generated-JavaScript parity. Keep
   Rust 1.74 and existing stage APIs; add no application-specific builtin.
2. Add focused unit fault tests for compare/commit/cleanup paths, invalid input,
   missing/empty/Unicode/bounded files, conflicts, busy markers and preservation
   of existing data. Retain all existing unconditional-write tests unchanged.
3. Add deterministic same-engine and mixed-engine process gates: two callers
   with the same snapshot must produce one commit and one conflict or busy
   result, never two successful stale commits. Test competing initial creation,
   independent destinations and post-release reuse. Kill an owned holder to
   verify fail-closed orphan-marker behavior rather than claiming auto-recovery.
4. Migrate every Task Manager task-state save, including interactive, direct and
   UI paths, to compare against the actual snapshot read, not re-encoded data.
   Fail visibly on conflict/busy, preserve drafts where applicable, and never
   replay a mutation. Preferences/page files require their own snapshots; no
   cross-file atomicity is promised. Keep the desktop lifetime lease.
5. Synchronize specification, verified course material, stdlib reference,
   dogfood documentation and release gates. Exercise interpreter/JavaScript,
   rebuilt native CLI and packaged desktop behavior before promotion. Verify
   Windows/Linux/macOS separately; local Windows validation does not qualify
   other hosts or a clean Windows VM.

Local validation now covers eight existing/absent same/mixed-engine snapshot
schedules, Rust competing writers and pre-rename fault injection, both-engine
boundaries/BOM/marker/path tests, Task Manager interactive/direct/UI/preference
busy rejection and recovery after deliberate owned-marker removal. Native
runtime tests cover all 15 builtins, and packaged native controls retain drafts
on busy rejection. See the development log for exact executed counts/artifacts.

## Alternatives and approval boundary

Keep the current single-writer policy: no API work, but cross-client updates stay
unsupported. Host-only locking cannot coordinate direct Sovra execution. OS file
locks could release on crash, but portable safe Rust 1.74 and supported Node hosts
need an evaluated implementation/dependency strategy; do not replace this with
racy post-start checks or stale-marker deletion and claim equivalence.

AGENTS.md requires: "Document and pause before fundamental syntax, memory-model,
type-semantics or compatibility decisions." The public function, new error
categories, pathname rules and crash/recovery behavior are compatibility choices.
The user's approval authorizes the additive API and its explicit cooperative,
fail-closed limits, not a general transaction or filesystem security API.
