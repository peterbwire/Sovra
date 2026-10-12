# ADR 0024: Filesystem capabilities for a full Sovra File Manager

Status: **Proposed; no new filesystem API approved or implemented.**

## Context

Approved ADR 0015 supplies bounded UTF-8 `std::read_text` and atomic
replacement `std::write_text`. Those operations support the existing File
Processor and the new local File Manager Preview/Copy slice. The browser host
currently enumerates a user-selected trusted directory with Node.js; Sovra
executes file reads and copies. Sovra cannot enumerate directories, inspect
file kinds, move or remove files, or access binary content. Claiming a full
file manager would be inaccurate.

Directory enumeration also exposes a language-design gap: executable Sovra
arrays exist, but array types are not nameable in public function signatures.
A returning `DirectoryRead { entries: [DirEntry] }` API therefore cannot be
added as a routine standard-library function without a type-system decision.
An open-directory cursor would instead require a resource/ownership and
lifetime model, which is likewise undecided. Platform filename encodings,
symbolic links, race behavior and error categories need explicit contracts.

## Proposed sequence

1. Keep the browser host's directory listing as a **private adapter** for the
   current trusted-workspace prototype. It passes selected native paths to
   Sovra's already approved text APIs. It is not a sandbox or public Sovra API.
2. Before introducing `std::read_dir`, select a general way to return a
   bounded collection of nominal records, or approve a resource-backed cursor.
   Define sorting, maximum entries/bytes, Unicode/non-Unicode filenames,
   symlink representation, mutation during enumeration, path errors and both
   execution engines' behavior. A string-delimited filename protocol is not
   acceptable because filenames can contain separators.
3. Separately review `move` and `remove` operations: file versus directory,
   replacement policy, symlink handling, atomicity/durability, cross-device
   moves, permission failures and irreversible deletion. The File Manager must
   not implement these by silently composing read/write in a way that claims
   atomic move or safe deletion.
4. Only after those general contracts and cross-platform tests are accepted,
   move enumeration and lifecycle rules from the Node adapter into Sovra and
   expand the dogfood application to navigation, move and delete. Binary file
   I/O needs a separate type and resource proposal.

## Decision required under AGENTS.md

Approval is required before changing public type syntax, array-return
semantics, resource/memory behavior or filesystem compatibility rules. The
first choice is **bounded collection return versus resource cursor** for
directory enumeration. This ADR records the dependency and does not approve
either design. A concrete API and failure matrix must be reviewed before
implementation. The existing private browser prototype may continue using
Node directory listing within its documented trusted-workspace limit.
