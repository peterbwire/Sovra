# Checked package function navigation, version 1

Implemented G09 slice: `check --format json --overlays-stdin` adds optional
`package_symbols` after successful checked graph compilation. Diagnostics, ordinary
source/AST/interface/evidence/IR admission and request framing stay unchanged.
Failed compilation publishes no navigation. This does not implement type/field
navigation, references/rename operations or imported local-variable inference.

`compile_with_source_navigation` returns ordinary `CheckedPackage` plus readonly
`PackageNavigation::as_json()`. Ordinary compilation and `compile_with_source_overlays`
do not collect this optional projection. Each unit's raw source is captured from
the exact read/overlay used to parse it; there is no post-check source reread to
manufacture compiler locations. Source-derived function name tokens/signatures
belong to that declaration's unit. Imported signatures never become fake local
AST declarations or supply target source spans.

## Identity and envelope

Function declarations use checked structural IDs in their owning unit. Local-call
uses resolve through those IDs; imported-call uses resolve through retained import
IDs and the resolver's canonical linked function targets. These are the same
canonical targets selected for IR emission. After the whole graph succeeds,
references become indices into the resulting declaration table. Two aliases of
one provider share a definition; same-named functions in different providers do
not. No client name lookup selects definitions.

The successful envelope has `schema_version:1`, `unavailable:false`, and:

| Table | Fields |
| --- | --- |
| `files` | Canonical UTF-8 `path`, original source `bytes`, lowercase SHA-256 `sha256`. |
| `declarations` | `file` index, qualified source `name`, original source `signature`, byte `start`/`end` and `name_start`/`name_end`. |
| `references` | Source `file`, call-head `start`/`end`, checked `definition` index. Registry builtin calls instead have null `definition` and bounded `signature`. |

Indices are private to one response/generation, never persistent symbol IDs.
Spans remain UTF-8 byte offsets. The LSP converts to UTF-16 only against verified
source bytes and links closed targets to file URIs; open targets retain the
client's matching URI. Outlines include only declarations from that document,
not copied imports. Function hover is plaintext source-owned signature text.

## Bounds and unavailable evidence

The initial optional profile permits 32 captured graph units, 512 KiB per source,
4 MiB total source bytes, 8,192 declarations and 8,192 references, 4,096-byte
paths/names/signatures, 512 KiB retained string payload and 512 KiB serialized
navigation. These are separate admission/publication limits, not total memory/CPU
bounds. Source capture is temporary; returned JSON does not contain full buffers.
The existing CLI published report remains below 1 MiB with the added projection.

Any projection exhaustion or unresolved projected target discards all tables:

```json
{"schema_version":1,"unavailable":true}
```

Valid compilation remains successful. There is no partial table, fallback name
resolution or mandatory compiler-limit relaxation. Graphs beyond navigation
budgets still receive diagnostics. Exact/overflow source-file-count, per-source
and serialized-output regressions compare unchanged IR with ordinary compilation.

## LSP source verification and lifetime

The strict decoder checks table sizes, indices, graph membership, byte boundaries,
source names and URI ownership before installing links. Open buffers must match
recorded size/SHA-256. Closed targets are read through bounded readonly regular-file
handles, at most the declared size plus one byte, and must match their fingerprints.
No-follow/nonblocking flags are used where the host exposes them. Missing, changed
or unreadable source disables the entire optional projection. Malformed metadata
never becomes hover/navigation. SHA-256 checks identity of bytes, not authorization.

The existing workspace generation guards still discard results after any relevant
open/edit/close/watch event. A failed check or rejected buffer clears old links.
Closed-source verification is not a filesystem lock: changes after verification
need normal watch/invalidation events, and unknown external disk changes can remain
unobserved until another check trigger. No persistent cross-edit IDs or atomic
filesystem snapshot is claimed.

Package-aware function definition/hover/outlines are now supported. Standalone
and single-entry optional local navigation remain separate. Package editor task
targeting and real editor-host package-specific acceptance remain G09 gates.

## Package task qualification update

G09 existing-entry package tasks and real VS Code workspace acceptance are now
Linux-qualified. See [editor package tasks](editor-package-tasks.md) for save
policy, cancellation and platform limits. `check --executable-package` performs
full package entry compilation without execution; ordinary directory checks keep
the partial application scanner. This flag cannot combine with overlay, symbols
or package-context service modes.
