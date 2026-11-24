# Unsaved generic package diagnostics

Enable these trusted user settings and restart the Sovra language server:

```json
{
    "sovra.experimentalGenerics": true,
    "sovra.experimentalGenericPackageDiagnostics": true
}
```

This opt-in checks a discovered generic package as one graph, including existing
registered modules and dependency sources. Open unsaved buffers replace their
corresponding source text in that check. Imports, module visibility, generic
calls, nominal ownership and unused dependency bodies are checked by the actual
generic package compiler. Diagnostics use the owning source's original byte
ranges, converted to UTF-16 editor positions, including Unicode and CRLF.

Changing, closing or rejecting a related buffer invalidates package results.
Closing a buffer restores its disk source on the next check. Save/watch events
refresh the graph too. Errors in closed sources appear as a bounded message on
the initiating document. Duplicate canonical buffers, rejected buffers and
invalid package metadata refuse the check instead of using stale facts or a
legacy compiler fallback. Publication requires the same document generations
that supplied the check, so superseded work cannot overwrite newer diagnostics.

The server uses compiler-selected context schema two and bounded overlay stdin
frames. JavaScript does not parse manifests or infer import/type ownership.
Checks never save buffers, update locks, execute Sovra code or grant host access.
Disk capture must succeed first; overlays cannot create files, modify manifests,
read unrelated sources, or repair a capture that already exceeds read limits.
Discovery stays inside configured workspace roots; outside a discovered package,
the existing standalone generic check remains available.

[Checked package function navigation](generic-package-navigation.md) now supplies
imported function definitions/hover and function outlines from checked identities
and source fingerprints. [Type annotation navigation](generic-package-type-navigation.md) also follows
checked function/record/enum/alias annotations. Body type/field/local references
are not projected;
unavailable facts clear previous results. Standalone mode keeps its existing
navigation. Formatting remains available. The separate
[generic package task setting](generic-package-tasks.md) controls Check/Run task
targets; enabling package diagnostics does not enable package tasks automatically.
Data/collection profiles and host policy remain unsupported in LSP.

## Compiler API and transport

`compiler::v2::check_generic_package_with_overlays(root, overlays, limits)` returns
only success or diagnostics. It captures a guarded package, admits existing
registered overlay targets, preflights aggregate owned text, and checks a logical
overlay input with independent source ownership. Overlay inputs cannot yield a
guarded disk capture or lock proof; no executable object escapes this API.
The original compiler/specialization/work/layout limits still apply.

The CLI endpoint is:

```sh
svr check --experimental-generics --format json --overlays-stdin ./app
```

It uses the existing `SVROVL1\n` framed request format and request IDs. The JSON
response adds `profile: "experimental-generics"`; the LSP refuses mixed profiles.
It publishes optional checked generic function navigation in the fingerprinted
`package_symbols` envelope; no legacy AST supplies these facts. JSON and a package directory
are required; combining overlays with symbols, context, locks, data/collection
profiles, duplicate flags or non-check commands refuses before checking.

Transport/library overlay ceilings are 32 buffers, 512 KiB per buffer and 4 MiB
total; library callers can lower these limits but cannot enlarge them. Package
capture and final overlaid-package byte limits apply separately. Disk snapshots
and logical copies have separate bounded storage; these are not hard RSS/CPU or
transactional filesystem guarantees. Paths must belong to the captured inventory;
unrelated, missing, duplicate or manifest targets refuse. Diagnostic byte offsets
must lie within their supplied buffer and at UTF-8 boundaries.

Linux compiler/LSP tests cover unsaved provider type errors, Unicode file ranges,
fix/close/watch refresh, rejected buffers, unchanged disk source/locks, malformed
transport, profile isolation and exact lowered limits. Actual VS Code Electron
acceptance and additional host runtime qualification remain separate.
