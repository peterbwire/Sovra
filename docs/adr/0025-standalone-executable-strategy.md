# ADR 0025: First standalone executable strategy

Status: **Accepted for N1 implementation (2026-10-12).** The user approved the
embedded checked-IR direction; public build syntax and deployment policy remain
separate decisions.

## Context

Sovra's Rust interpreter already executes checked IR with process I/O and
text-file operations. Generated JavaScript requires Node. The Task Manager
CLI works, while its GUI is Node-hosted. A user-installed Task Manager must
eventually run on compatible Windows without Node, Rust or `svr` installed.
See [the technical plan](../architecture/NODE_FREE_APPLICATIONS.md).

## Proposed decision

First emit a native Windows executable containing a bounded, checked Sovra IR
image and a linked Rust execution runtime. Compilation and linking happen on
the developer machine. The program is interpreted *inside* its standalone
native executable; this milestone does not claim native machine code for each
Sovra function. Reuse current `ProcessHost` and IR semantics, preserve all
current execution targets, and prove exact-artifact behavior on clean Windows.

This choice establishes a distribution and runtime boundary before selecting
Cranelift or LLVM. A subsequent codegen backend must share conformance tests
and receive separate value, ownership, error and host ABI approval. The private
IR image is not a public cross-version serialization contract.

## Alternatives and tradeoffs

- Embed source and run the full compiler at every launch: easiest packaging,
  but does not establish an ahead-of-time compiled program and carries more
  compiler code and startup work. Rejected for N1.
- Translate IR directly to Rust: creates native code but requires complete
  lowering/parity before the first executable. Deferred as an experiment.
- Introduce Cranelift or LLVM immediately: both can emit native objects but
  still need Sovra value/stdlib/runtime design. Deferred until the smaller
  executable proof validates the host boundary.
- Bundle Node with generated JS: avoids a separate Node installation but
  retains a JS runtime dependency and does not meet the standalone-language
  direction. Not selected.

## Acceptance before claiming implementation

The generated Task Manager executable must perform persistent add/list/
complete/delete on a clean compatible Windows image without Node, Rust or
`svr`; match current interpreter output, state bytes and exit statuses; and
pass malformed-state and I/O-failure cases. Inspect imports and document
all OS/runtime dependencies. Keep JS and interpreter regression suites green.
The GUI remains Node-hosted until a separate UI host decision is approved.

## Approval requested

Approve N1's AOT checked-IR executable container and Rust runtime reuse as
the first implementation direction. This does **not** approve the illustrated
CLI syntax, a public IR format, native codegen ABI, GUI toolkit, WebAssembly
host contract or any change to Sovra language semantics. Those require
concrete proposals before implementation under `AGENTS.md`.
