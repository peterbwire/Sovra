# ADR 0042: Borrow unsaved package entry sources

Status: **Partial: compiler overlays, CLI context/transport and LSP package
diagnostics implemented, 2026-11-12.** Checked cross-file function navigation is implemented; package editor task
targeting and package-specific editor-host acceptance remain pending. No language syntax or type rules
change. Existing standalone LSP behavior remains unchanged. Implementation
checkpoints below are chronological; later navigation notes supersede earlier
pending/unavailable statements.

## Compiler contract

`project::packages::compile_with_source_overlays(root, buffers, limits)` compiles
an existing disk-resolved graph through the ordinary checked package pipeline,
using supplied source buffers instead of reading those entry files. Each
`SourceOverlay` supplies an existing path and caller-owned UTF-8 String. The
compiler borrows buffer text while parsing/checking and returns ordinary checked
package output (IR, unit count and remaining metadata allowances). It neither
writes source files nor executes input programs, manifests or build scripts.

Disk manifests, canonical package entries and approved resolver behavior remain
authoritative. Entry targets must already exist and belong to this graph. This
slice cannot overlay manifests, introduce new/missing files, add multi-file modules
or silently redirect a source to an unrelated package. Canonical duplicate targets
are rejected even when spelled through different lexical paths or aliases. A
provider with several dependency aliases still compiles only once.

`SourceOverlayLimits` has explicit per-request allowances. Its initial DEFAULT
matches current standalone LSP open-buffer limits: 32 buffers, 512 KiB UTF-8 bytes
per buffer and 4 MiB total. These limits bound supplied overlay admission, not
caller allocation, temporary capacities, total compiler memory or CPU. Check count
before path-map growth and all payload lengths before overlay-target canonicalization (the disk graph has already resolved).
Per-source frontend, aggregate graph/source/AST/interface/string, active metadata
and linked IR budgets remain independently enforced. Oversized overlays cannot
bypass normal source admission. Buffer storage is not copied into snapshots.

E4117 reports missing, unrelated or duplicate canonical overlay targets. E4118
reports overlay count/per-buffer/aggregate admission. These have no fabricated
source range/file. Ordinary parse/semantic diagnostics keep canonical entry
identity and offsets into the actual unsaved buffer, not the disk version.
Mandatory metadata exhaustion remains E3025. No partial IR is returned after any
failure. A fresh invocation cannot reuse previous successful facts.

Manifests and non-overlaid entries are still read from disk during one invocation;
this is not a content lock, a transactional filesystem snapshot or a persistent
workspace cache. The LSP must eventually associate responses with its own complete
buffer-generation set and discard stale results after any dependency edit/close.
An overlay path must never be accepted as evidence that its text matches disk.

## Verification and next slices

Tests cover simultaneous unsaved provider/consumer type changes, canonical aliases,
exact IR/retry behavior, unchanged disk sources, Unicode/CRLF diagnostic ranges,
exact and one-below independent quotas, duplicate/missing/unrelated targets and
restoration to ordinary compilation when overlays are absent. Existing package
compilation regression tests continue to exercise the same emitter/resolver.

The bounded versioned CLI request/report contract now carries buffers without
writing a temporary copy of an entire dependency graph (checkpoint below). Next
integrate LSP workspace roots, entry selection, dependency-aware document generations,
cancellation and cross-file diagnostic ownership. Cross-file navigation must use
checked declaration/call provenance and source-owned ranges, never string matching
or foreign signature spans. Rejected dependencies must not expose earlier facts.
Keep the standalone fallback path for documents outside a supported package.
Package-aware LSP remains a partial contributor task until those gates pass.

## Implemented CLI transport checkpoint

`check --format json --overlays-stdin` accepts one bounded versioned stdin frame
and returns check JSON with request correlation. It reuses the checked overlay API
and does not run code/write sources. Strict count/path/source/total admission,
malformed/truncated/trailing/UTF-8 failures, EOF behavior and report replacement
are documented in the [wire contract](../reference/source-overlay-wire.md).
No JSON-parser dependency was added. E4119 rejects malformed/oversized requests;
E4120 replaces oversized reports, preserving a valid request ID. Ordinary check
modes remain separate. Next integrate workspace roots/dependency generations and
cross-file diagnostics; package-aware navigation still requires checked provenance.

## Workspace diagnostic integration checkpoint

`check --format json --package-context` resolves canonical graph membership only,
with explicit `scope:package_graph`/`source_check:false`; source validity is never
inferred from context success. Reports include bounded UTF-8 canonical root/entry
paths, version one, up to the existing 1,024-package resolver ceiling and a 1-MiB
published cap. It cannot combine with overlay/symbol/service check modes.

The LSP discovers nearest manifests only inside initialized workspace folders,
then submits relevant existing open entry buffers as one correlated overlay check.
It invalidates all workspace document objects on edits/close/watch notifications,
not merely the edited document, so dependency changes cancel stale consumer work.
Rejected related buffers block analysis; duplicate canonical URI aliases are not
arbitrarily resolved. Correct open-file UTF-8-to-UTF-16 ranges retain URI/version
ownership. Closed-source errors become explicit operational summaries rather than
fabricated consumer ranges. Package metadata/diagnostic decoders fail closed.

Single-entry optional local hover is preserved after successful package checking;
imported-package navigation remains unavailable until retained cross-file evidence
is projected. No new AST/source is guessed from signatures. Existing standalone
LSP lifecycle, caps, cancellation and local navigation stay covered independently.
The VS Code extension supplies workspace roots, forwards source/manifest events
and restarts on folder changes. Workspace-wide invalidation is deliberately
conservative; no atomic filesystem snapshot or exact incremental cache is claimed.

Regression coverage includes real compiler unsaved provider/consumer checks,
dependent invalidation without consumer version changes, close/rejected-buffer
restoration, Unicode diagnostic ownership, closed dependency summaries, generation
cancellation, unrelated-package isolation, POSIX/Windows containment rules and
single-entry local hover. The same suite runs through relocated installed tooling.
Cross-file navigation, package task targeting and real installed editor-host
workspace acceptance remain next; G09 is not complete.

## Checked function navigation checkpoint

Successful overlay checks now project bounded function definitions/call references
from immutable checked structural/import identities and exact resolver-linked
targets. Each declaration uses its own unit's raw source tokens, with source
fingerprints captured during that compilation. No foreign signature spans or
client name search select target locations. Two aliases share a target; identical
names in distinct providers stay separate. Failed compilation publishes no facts.

The [navigation contract](../reference/package-navigation.md) documents optional
all-or-unavailable quotas, strict schema/index/UTF-8 validation, open/closed source
size/SHA verification and bounded readonly closed-file reads. Definition/hover/
outlines now support imported functions. Generation invalidation clears links on
dependency edits, failures, rename and close. Navigation exhaustion does not alter
ordinary checking/IR. Package task targeting and real package-specific editor-host
acceptance remain; this does not add type/field navigation or a rename operation.

## 2026-11-12: package task and real-host acceptance

Compiler-owned task descriptors now select package roots/cwd, including nested
entries. Bounded preparation saves related source buffers and refuses dirty
manifests. Package Check explicitly compiles dependency-free entry graphs with
`--executable-package`; ordinary scanner behavior remains. Real Linux VS Code
1.106.0 accepts unsaved dependency diagnostics/navigation, manifest refusal,
Check/IR/JS/Run tasks and restart. Its global task save policy remains user-owned;
the unrelated-draft test selects `task.saveBeforeRun=never`. See
[task contract](../reference/editor-package-tasks.md)
for bounds and limits. G09 bounded existing-entry scope is complete for portable
behavior/Linux; G10 formatter qualification is next. Windows/macOS, MSRV, release
and independent security qualification remain separate. Task Manager stays on hold.
