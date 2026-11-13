# Registered package source inventory

Status: **Partial; additive Rust inspection API.** The user approved the
[G12 design](../design/G12_MODULES_AND_LOCKS.md). This first implementation selects
registered paths and observes metadata. It does not compile external modules or
implement locks, locked commands, source snapshots or verified source bytes.

```rust
use sovra::compiler::project::packages::inspect_source_inventory;
let inventory = inspect_source_inventory("path/to/package")?;
// inventory.graph: existing canonical-root package/dependency identities
// inventory.sources: owning roots, source kinds, paths, manifest spans and sizes
```

This opt-in API, also used by the source-unit loader below, admits the approved section:

```toml
[project]
name = "utilities"
entry = "main.svr"

[modules]
arithmetic = "src/arithmetic.svr"
```

Mapping keys use existing ASCII module identifiers. Quoted paths must be relative
`.svr` names with `/` separators and nonempty ASCII letter/digit/underscore/dot/
hyphen components. Dot/dot-dot, empty components, backslashes, absolute/drive/UNC
paths, Windows device basenames and trailing dots/spaces are rejected. ASCII
case-fold-equivalent mappings and entry collisions are errors. Existing manifest
string, duplicate-key/section and bounded diagnostic rules apply.

`PackageSourceInventory` owns the unchanged-shaped `LocalGraph` and ordered
`PackageSource` records. Packages appear in first-discovery depth-first dependency
alias order; each contributes its entry first, then mappings sorted by module name.
Canonical package aliases/diamonds contribute sources once. Equal display names
never merge roots. Existing package-cycle and manifest/edge budgets still apply.

Each record retains canonical owning root, `PackageSourceKind::Entry` or
`Module(name)`, decoded declared path, canonical selected path, the original
manifest assignment-line span and observed file size. A module tag is registration
metadata, not a checked declaration identity. There is no directory scan or
implicit selection of unregistered `.svr` files. Files are not read or parsed:
UTF-8, matching inline `mod` headers, entry/module declaration-name conflicts,
imports, visibility and body validity await the compilation step.

At selection time, canonical files must remain within their package root and be
regular files. Unix device/inode identities reject aliased selected files,
including hard links, across the inventory. Canonical internal symlinks may select
a regular file once; escaping links and repeated identities fail. The new API
reports E4160 on hosts without implemented file-identity checks (currently non-Unix)
rather than asserting unsupported alias detection. Linux tests do not qualify
macOS or Windows behavior.

Limits are 1024 mappings per package, 4096 selected sources including entries,
16 MiB observed size per source and 64 MiB observed total. Count admission happens
before resolving over-budget selections; observed sizes are checked before
retention. Existing graph limits remain 1024 packages, 1 MiB per manifest, 8 MiB
manifest total and 8192 dependency aliases. Exhaustion returns diagnostics with no
partial successful inventory. These are component budgets, not a process sandbox.

Manifest syntax/key errors reuse E4010/E4014/E4015/E4016 and bounded E4018 errors.
E4112 covers count exhaustion, E4113 containment escapes and E1003 observed source
size exhaustion. E4160 covers registration/path/identity failures and unavailable
host checks. Relevant errors attach the owning manifest assignment line.

Paths and metadata can change during/after selection. This is neither an atomic
filesystem snapshot nor authority to compile subsequently reopened paths. The
next stage must capture bounded validated opened-file bytes, retain original
source ownership, enforce actual byte/token/AST budgets and share verified bytes
with checking/lowering and future locks. Observed sizes are not content hashes.

Older Rust `resolve`/`compile`, formatter, context and overlay paths retain strict
admission and reject `[modules]` (E4010). Executable CLI commands now separately
classify the declared graph and route registered inputs through the source-owned
compiler; entry-only paths never silently ignore mappings. Complete formatter/
editor inventory integration and lock workflows remain later approved steps.
Task Manager and G11 application checking remain unchanged.


The next additive [source-unit loader](package-source-units.md) now captures and
parses selected bytes with original file ownership and matching-header checks.
Its guarded read support is currently narrower than inventory metadata support.
Semantic compilation, locks and default CLI/tooling integration remain open.
