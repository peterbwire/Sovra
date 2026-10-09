# ADR 0021: Unix owner-only text-file replacement

Status: Accepted and implemented (2026-10-09).

## Problem

The approved ADR 0015 `std::write_text` implementation creates a new temporary
file and renames it over the destination. On Unix the Rust and Node hosts use
the ordinary creation mode, typically `0666` masked by the process umask. If a
user previously restricted a task file to `0600`, an edit under a common `022`
umask can replace it with a `0644` file. Titles then become readable to other
local users when the containing directory permits access. This is a release
privacy concern for the Task Manager and any other Sovra text-file consumer.
The defaults and creation-mode controls are specified in the
[Rust Unix `OpenOptionsExt` documentation](https://doc.rust-lang.org/std/os/unix/fs/trait.OpenOptionsExt.html)
and [Node 22 `fs.openSync` documentation](https://nodejs.org/download/release/latest-jod/docs/api/fs.html#fsopensyncpath-flags-mode).

The replacement also has distinct symlink and concurrent-writer limitations.
This ADR does not claim to solve those: it changes only the mode of newly
created temporary files. Task Manager documentation already requires a regular
file in a private directory and one writer per file.

## Decision

For `std::write_text`, create each temporary file with owner read/write bits
only (`0600`) on Unix. The same mode applies to creation and replacement. A
restrictive umask may remove bits; the function does not add bits later. On
Unix this means a successful replacement cannot turn an owner-only task file
into a group- or world-readable file through mode bits. Replacing a previously
shared text file intentionally narrows its mode. Users who need group sharing
must arrange it outside this API or use a future explicit permission policy.

Use Rust's Unix `OpenOptionsExt::mode(0o600)` and Node's `openSync` mode argument
`0o600`. Keep exclusive creation, same-directory rename, 16 MiB bound,
result-record shape, error categories and pre-rename failure behavior unchanged.
This is a host file-creation policy, not a Sovra syntax or type change.

On Windows, these mode bits do not define an ACL privacy guarantee. Temporary
files inherit host directory security behavior; users must keep sensitive task
files in a private directory. Do not claim that this change preserves custom
Windows ACLs or prevents races. The source-level `std::write_text` contract
remains available in both engines, with the platform limit documented.

## Alternatives

- Keep the current host-default mode and require a private directory on every
  platform. This leaves the Unix `0600` to `0644` regression possible in a
  traversable directory.
- Preserve an existing file's mode. This can unintentionally grant access to a
  different inherited group after replacement; preserving owner/group and ACL
  metadata consistently across both hosts requires a larger design.
- Add a separate private-write API. This avoids changing `std::write_text`, but
  duplicates the file-write surface before version one is published.

## Acceptance

On Unix, test both hosts under a permissive `022` umask: new text files and
replacements of an existing `0600` file must remain no broader than `0600`.
Confirm contents, result records, and existing atomic failure tests remain
correct. On Windows, run existing text-file and Task Manager parity suites and
document the private-directory requirement. Keep symlink and concurrent-writer
limits explicit. Recheck source packaging and candidate artifact behavior.

## Approval boundary

AGENTS.md requires a pause before compatibility decisions. Narrowing mode bits
on successful `std::write_text` calls changes an observable public file-I/O
contract, even though it improves the default privacy behavior. The user
approved the recommended policy by directing implementation in the following
turn. Rust and Node hosts now request `0600` at temporary-file creation.
