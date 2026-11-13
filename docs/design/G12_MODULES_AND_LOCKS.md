# G12: explicit multi-file packages and local source locks

Status: **Approved by the user; modules, explicit locks and package formatting partially implemented.**
The additive [inventory API](../reference/package-source-inventory.md) admits
registrations. [Source-owned parsed units](../reference/package-source-units.md)
now capture selected bytes and validate registered headers. The additive
[primitive callable compiler](../reference/registered-package-functions.md) now
checks/links all bodies for primitive interfaces and executes an independent
consumer. The [record compiler slice](../reference/registered-package-records.md)
now supports nominal records, including visible record-valued fields, with
private/public type boundaries and finite canonical field references. Remaining
type contexts and full editor integration remain open. [Explicit source locks](../reference/package-source-locks.md) and inventory-aware package formatting now work on the documented hosts. [Executable CLI routing](../reference/registered-package-cli.md)
now selects complete checked registered graphs for check/run/build/test.
G12 may proceed alongside partial G11. This document does not complete either
contributor task or the established M12 milestone.

## Current source and approved boundaries

[ADR 0010](../adr/0010-local-library-foundation.md) already approves local path
dependencies, canonical-root graph identity, deterministic alias traversal,
`use dependency::module;`, exports and package-cycle rejection. Current
`project::packages::resolve` implements that graph; `packages::compile` loads one
entry source per canonical package and links checked IR. Exported records,
checked lowering, resource budgets and package editor evidence have since grown
around that entry-only contract.

At design approval, there was no executable module-file inventory or Sovra lockfile; both now have bounded implementations.
Application dotted imports are a separate partial checker and must stay separate.
Existing local directories are mutable source locations, not locked artifacts.
G11's application models/UI/policies are outside this executable package slice.

## Recommended decision A: explicit manifest registration

Retain `[project]`, existing dependencies and inline source grammar. Add an optional
`[modules]` section mapping one module identifier to one package-relative source:

```toml
[project]
name = "utilities"
version = "1.0.0"
entry = "main.svr"

[modules]
arithmetic = "src/arithmetic.svr"
```

The mapped source uses existing syntax, rather than synthesizing a source wrapper:

```svr
mod arithmetic {
    fn product(a: Int, b: Int) -> Int { return a * b; }
    export fn square(value: Int) -> Int {
        return arithmetic::product(value, value);
    }
}
```

These snippets are **approved compilation fixtures, not executable multi-file
examples today**. Registration is available to the additive inventory/source-unit APIs; semantic
older Rust package compilation remains strict; executable CLI commands now route
to the source-owned compiler when registrations are present.
A library entry may be empty. The source consumer remains unchanged:

```svr
use utilities::arithmetic;
fn main() { print(utilities::arithmetic::square(7)); }
```

Rules for the first multi-file slice:

- Every mapping key is one identifier under the existing module-name rules.
  Values are quoted relative `.svr` paths under the existing manifest string
  grammar. No recursive scanning, globs or implicit filename-to-module inference.
- A mapped file contains exactly one existing inline `mod` declaration matching
  its key, plus optional existing dependency `use alias::module;` declarations
  and comments. Reject other modules and top-level functions/types/records there.
  This deliberate first-stage restriction can be expanded separately later.
- The entry retains its ordinary functions, types, records and inline modules.
  Duplicate module names across entry/mapped sources fail; never merge declarations
  or pick a winner by file order. A file cannot be both entry and mapped module,
  or supply multiple registrations through filesystem aliases.
- Same-package qualified calls keep existing `arithmetic::square(...)` spelling
  and inline-module visibility rules. There is no new `self`/`package` import
  keyword or newly reserved dependency alias. Private helpers remain private;
  qualified module-local private calls keep the existing rule.
- Existing dependency imports expose exported members of explicitly imported
  modules only. Imports belong to their original source file; an entry import
  does not grant imports to a mapped file. No transitive dependency access,
  re-exports, nested module hierarchy, unqualified sibling capture or new type
  semantics is introduced.
- Register all selected module declarations before checking bodies. Check every
  registered source, including unused/private declarations. Dependency entry
  points never run. Reject package cycles as today; same-package function calls,
  including recursion, keep existing inline-module rules. Alias cycles retain
  existing diagnostics; do not confuse call cycles with package cycles.
- Presence of `[modules]` selects executable package loading even without path
  dependencies. Manifests without it retain existing unlocked behavior.

**Alternative B (not recommended):** infer modules from filenames and accept
`mod arithmetic;`. This adds source grammar, search precedence and discovery
rules, and needs separate compatibility decisions. Do not implement it implicitly.

## Identity, ownership and containment

Keep canonical package roots as invocation-local graph identity. Equal display
names or equal source bytes never unify distinct roots; two aliases of one
canonical root still refer to one package. A module identity is its owning package
identity plus module name, and a declaration adds its owning source/declaration
identity. Absolute paths remain locations, not portable published package IDs.

Parse each file independently with original byte/scalar spans. Carry source-unit
ownership through semantic interfaces, checked evidence, IR linking, diagnostics
and navigation; never concatenate files or graft foreign spans onto the entry.
Aggregate resource accounting must include every unit and interface copy.

Resolve mappings from their declaring package root. New mapping paths use `/`,
contain nonempty ASCII `[A-Za-z0-9_.-]+` components, and reject `.`/`..`, absolute,
drive/UNC paths, backslashes, Windows device basenames, and trailing dots/spaces.
Reject ASCII-case-fold-equivalent mapped paths and collisions with the entry's
relative path in multi-file mode. These rules apply to new registrations, not a
retroactive standalone-source filename restriction. Existing builtin `std` collision
rules remain unchanged. Broader Unicode module-path support is a later decision.

Canonical selected files must remain inside their owning package, including
symlinks. Explicit dependency sibling roots remain allowed. Reject nonregular
sources and aliased duplicate source identities. A hostile rename/reparse race
must not bypass containment or substitute bytes after verification. Use validated,
bounded opened-file snapshots and fail closed when the claimed host cannot
establish the required checks. Record actual host qualification, rather than
inferring Windows/macOS behavior from Linux. A collection of verified file reads
is not an atomic snapshot of a changing filesystem.

## Recommended decision C: versioned local-source lock

Add `sovra.lock`, covering the entire selected local dependency graph, including
root manifest, entries and explicitly mapped sources. This is source/graph
integrity for offline local builds, not a registry, immutable cache, signature,
publisher authentication or a guarantee of bit-identical binaries across toolchains.
Assets and files outside selected compiler inputs are not locked by this slice.

Version 1 is strict UTF-8 JSON with these exact properties:

| Record | Fields |
| --- | --- |
| Lock root | `format: "sovra-lock"`, `version: 1`, `root: "p0"`, `packages: [...]` |
| Package | `id`, `name`, `version` (string or null), `manifest`, `entry`, `modules`, `dependencies` |
| Manifest | `sha256` (64 lowercase hex characters), `bytes` (nonnegative integer) |
| Source | `path` (package-relative `/` spelling), `sha256`, `bytes` |
| Module | `name`, `source` (Source record) |
| Dependency | `alias`, `path` (declared manifest-relative spelling), `target` (package ID) |

Source paths record the selected manifest-declared paths normalized to relative
`/` spelling; they are not absolute canonical host paths. Existing entry spellings
are normalized without weakening canonical containment. New mapped paths obey the
stricter registration rules above. Dependency `path` values preserve the declared
manifest text because they are part of the approved dependency locator contract.

`entry` is a Source record. `modules` and `dependencies` are arrays sorted by
module name and dependency alias respectively, using UTF-8 byte order. Package
IDs `p0`, `p1`, ... are assigned on first discovery in deterministic depth-first
alias order, deduplicating canonical roots. They are graph-local references, not
permanent identities or a global name registry. Renumbering after a graph change
is a reviewed lock change, not type unification. Package arrays use numeric ID order.

Do not serialize absolute package roots. Resolve declared dependency paths from
their owning manifests, then verify alias edges, canonical deduplication and every
node against the lock. Relative dependency layouts relocated together produce
the same lock. Absolute dependency paths keep their existing meaning and cannot
claim relocation independence. Two distinct roots with identical contents remain
two nodes; a diamond/shared root remains one node. Host-specific identity/deduplication topology changes
must fail locked verification rather than silently selecting a different graph.

Hash raw manifest/source bytes with SHA-256, including comments and line endings;
LF/CRLF changes intentionally change a digest. Size/mtime alone are insufficient.
Reuse the trusted Rust hash implementation after its usual known-answer tests;
no shell hashing subprocess or new dependency is required. Verify UTF-8 and all
existing grammar/semantic requirements independently of hash equality.

Reject unknown versions/fields, duplicate object keys, duplicate IDs/aliases/
modules, missing/extra graph records, malformed hashes, invalid paths and dangling
references. The writer emits compact deterministic JSON plus one LF. The reader
may accept JSON whitespace/property reordering but must validate the complete
schema and compare normalized content, never accept a matching prefix or ignore
unsupported records.

## Recommended decision D: explicit lock workflow first

Approved command plan, **not implemented**:

```text
svr lock PACKAGE_DIR
svr lock --check PACKAGE_DIR
svr check --executable-package --locked PACKAGE_DIR
svr run --locked PACKAGE_DIR
svr build --locked --emit js PACKAGE_DIR
svr test --locked PACKAGE_DIR
```

`lock` resolves locally and validates the graph/source inventory before publishing
one lockfile; it never executes package code or fetches dependencies. `lock --check`
compares without writing. Generation need not make a semantically invalid program
valid; locked check/run/build/test still perform full executable checking.

Locked compilation requires the existing lock to match the complete graph and
all selected input snapshots. Missing, stale, malformed or over-budget locks fail
before execution/emission, without unlocked fallback or automatic lock rewrites.
Hash and parse the same captured manifest/source bytes; a separate verification
pass followed by ordinary path rereads is insufficient. Never execute IR from a
partially verified graph. Backend/native-host routes must use those verified
inputs or explicitly reject locked mode until integrated and qualified.

In the first slice, unlocked commands keep their behavior even if a lock exists;
`--locked` is explicit. This preserves existing workflows but requires documenting
that lock presence alone does not enforce a build. Automatic default enforcement
would be a separate compatibility decision. A lock command is not implementation
of reserved install/update/publish commands.

Reject locked mode for standalone file arguments, partial application scanning,
unsaved source overlays and context-only requests. JSON executable checking may
report locked failures through the existing diagnostic-report contract; any
schema extension needs an explicit versioned design. Ordinary compiler errors
retain existing codes/spans. Allocate lock/module error codes after checking the
registry during implementation, rather than reserving conflicting numbers here.
Existing success/failure exit conventions remain.

Publish only the explicitly requested `sovra.lock`, with staged bounded output,
opened-file/parent conflict validation and atomic replacement on qualified hosts.
Refuse link/reparse targets and unsafe/conflicting publication; leave the previous
lock intact on failure. No source edits, build scripts or network access.

## Admission limits and existing tooling

Retain existing 1024-package, 1 MiB-per-manifest, 8 MiB-total-manifest and 8192-alias
budgets. Approved multi-file additions: at most 1024 mappings per package and
4096 selected canonical sources across the graph, including entries; 16 MiB per
source and 64 MiB total selected source bytes; 1,000,000 tokens per source and
4,000,000 total non-EOF tokens. Check counts/budgets before resolving/retaining an
excess item. Existing cumulative AST/interface/string/checked-evidence/IR limits
still apply to all units and repeated interface copies, with no legacy fallback.

Lock input/output is bounded to 8 MiB, 16 JSON nesting levels, 16 KiB per decoded
string and 8 MiB total decoded strings. Package/edge/source counts above also
bound records. Count raw bytes before allocation; reject exhaustion without a
partial successful inventory, verified graph, linked IR or published lock.
These are component budgets, not a complete compiler CPU/memory sandbox.

Package formatter, compiler-owned context, overlays, LSP navigation and editor
tasks currently assume entry-only sources. They must either understand the new
inventory or explicitly refuse multi-file mode before claiming complete graph
coverage, saving, formatting or navigation. Do not silently use an entry-only
fallback. Manifests remain disk-owned under ADR 0042; unsaved module overlays are
a later explicit integration, not an automatic extension of that contract.

## Approved implementation sequence

1. Add bounded manifest module records and a deterministic source inventory API,
   preserving old APIs/modes. Test containment, names, duplicate identities,
   complete counts and unchanged entry-only behavior. No execution claim yet.
2. Compile selected files as source-owned module units through checked semantic
   interfaces/lowering/linking. Add an independent consumer/library fixture;
   check, interpreter and JS must all produce `49`, and reject private access.
3. Add bounded lock parsing/canonical generation and snapshot integrity validation
   as Rust APIs. Test deterministic relocation, aliases/diamonds and mutations.
4. Integrate explicit CLI generation/locked execution with shared snapshots and
   qualified publication. Unsupported tool/host routes fail explicitly.
5. Integrate package formatter/context/editor/navigation against the inventory,
   qualify exact-host CI/MSRV behavior, and update G12's actual supported scope.

## Acceptance matrix

| Case | Required result |
| --- | --- |
| Old inline modules/local consumers | Existing behavior and diagnostics preserved |
| Registered external module | Original spans; entry and consumer execute public exports |
| Private/unused invalid body | Private access fails; unused bodies still checked |
| File-local dependency imports | No sibling source or transitive import leakage |
| Entry/mapping duplicate name, header mismatch or aliased file | Deterministic rejection; no merge/winner |
| Equal display names/equal bytes/distinct roots | Distinct nominal package/module identities |
| Multiple aliases/diamonds | One canonical node and shared identity; all edges retained |
| Package cycle versus ordinary recursion | Package cycle rejected; existing function recursion preserved |
| Traversal, links/reparse escapes, special files, open conflicts | Refused within claimed host contract; no partial execution |
| Lock regeneration without changes | Byte-identical canonical output |
| Relative layout relocation | Same lock; same behavior and owned diagnostics |
| Manifest/source/comment/line-ending mutation, added mapping or changed alias | Locked failure, no rewrite/fallback/emission/execution |
| Missing/duplicate/unknown/trailing nonwhitespace lock data | Full-schema failure, not ignored metadata |
| Mutation between hash and parse | Compiler uses exactly the verified captured bytes |
| Exact/exceeded count, byte, token and metadata budgets | Exact accepted; excess rejected without partial success |
| Lock write conflict/failure | Prior lock preserved; no source edits |
| Formatter/LSP/task entry-only assumptions | Updated complete inventory or explicit unavailable mode |
| Linux/Windows/macOS/MSRV | Actual evidence per claimed host, not inferred qualification |

## Approval boundary

[AGENTS.md](../../AGENTS.md) requires: "Document and pause before fundamental
syntax, memory-model, type-semantics or compatibility decisions."
The user approved A (explicit registration), C (lock schema/identity) and D
(explicit locked workflow), together with these scope/limits. Continue their
incremental implementation without requesting the same approval again. B is an alternative, not part of the recommendation.
Existing G11 remains partial and Task Manager feature work remains on hold.


## Remaining work after locks and formatter integration

- Context discovery must expose every selected registered source under a versioned
  compiler-owned schema; existing entry-only consumers must continue to refuse
  unsupported graphs, without silently accepting a partial source list.
- Unsaved module buffers need an explicit overlay contract, cumulative admission,
  owning-file parse/diagnostics and complete failure invalidation. Existing ADR
  0042 entry-only framing is not implicitly widened.
- Checked navigation must retain original source declarations, identities and
  fingerprints for registered functions/records and refuse stale metadata. Attached
  synthetic checking aliases/shapes are not original source declarations.
- LSP/editor tasks must discover, save/watch/invalidate the complete selected graph;
  test actual closed/unsaved module failures and definitions, plus installed tools.
- Review remaining type contexts with G13's array/interface design. Unsupported
  contexts must continue to fail explicitly with no unchecked compilation fallback.
- Collect exact-host Windows/macOS/MSRV and native evidence before extending guarded
  capture/publication claims. Existing Linux tests and native smoke do not close
  those platform gates. General registry/publishing remains separate work.
