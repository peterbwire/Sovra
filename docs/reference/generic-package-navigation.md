# Checked generic package function navigation

Enable [generic package diagnostics](generic-package-diagnostics.md) and restart
the Sovra language server. Successful package checks now provide:

- Go to Definition for checked user-function calls, including imported generic
  functions and same-module/private calls admitted by the compiler.
- Hover containing the destination's declared function header.
- Function declaration outlines and declaration hover for open package sources.

No additional navigation flag is needed. References cover the compiler-retained
callee range, including its written qualification; explicit type arguments are
not part of that range. Hover shows the declared generic signature, not an
invented concrete specialization. Body type/field/parameter/local references and
constructor-pattern navigation are not projected by this package profile yet.
[Checked type annotation navigation](generic-package-type-navigation.md) now
covers function signatures, record fields, enum payloads and alias targets.
Existing standalone generic navigation keeps its own supported scope.

Targets come from sealed caller/callee signature bindings and canonical package
declaration identities. Identical function names in different files remain
distinct. The renderer checks original declaration/name ranges against owned
source, and does not resolve a target from spelling, import text or the legacy
AST. Failed package checking publishes no navigation.

Every optional batch includes the owning source paths, UTF-8 byte lengths and
SHA-256 fingerprints. Open sources must match their current buffer bytes. Closed
sources are read as bounded regular files and must match the checked fingerprint.
A missing, changed or unreadable file disables the whole optional batch. Existing
document-generation checks prevent publication into replaced buffers. Failed
checks and unavailable metadata clear previous facts instead of reusing stale
definitions.

Navigation is separately bounded: at most 32 source files, 512 KiB per file,
4 MiB source text, 8192 function declarations/references, 4096-byte paths/headers
and a 512 KiB JSON envelope, plus the selected `GenericSymbolLimits` work budget.
Quotas can narrow these limits. Exceeding a quota yields the complete
`{"schema_version":1,"unavailable":true}` envelope while the source check can
still succeed. No partial declaration/reference arrays are published. These
logical limits do not promise hard RSS/CPU isolation or atomic filesystem reads.

The library API
`check_generic_package_overlays_with_symbols(root, overlays, overlay_limits, symbol_limits)`
returns optional JSON metadata or compiler diagnostics; it exposes no executable,
guarded disk capture or lock authority. The original check-only API is unchanged.
The generic overlay CLI response now includes optional `package_symbols` using
the existing fingerprinted package-navigation envelope and an explicit generic
report profile. It never changes source or executes Sovra code.

Tests exercise distinct same-named destinations, exact original call ranges,
original source headers, optional quota refusal, real imported function
definitions/hover/outlines, fingerprint mismatches and clearing facts after an
unsaved provider becomes invalid. Legacy package navigation and standalone
generic behavior remain covered. Actual VS Code Electron acceptance and
additional host runtime evidence remain separate from Linux compiler/LSP tests.

[Type navigation](generic-package-type-navigation.md) also covers checked explicit local annotations, preserving aliases and per-function type scopes. [Local value navigation](generic-package-local-navigation.md) covers checked bindings independently.
