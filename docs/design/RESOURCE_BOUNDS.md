# Resource-bound audit

Status: structural depth 128 approved and implemented under ADR 0007; broader
resource-budget and public-AST safety work remains in progress.

Measured follow-up: see [ADR 0007](../adr/0007-compiler-structural-depth.md).
The reusable `node scripts/probe-compiler-depth.mjs [path-to-svr]` probe runs
isolated check/build subprocesses with timeouts. Windows GNU debug builds crashed
at all three sampled depth-2048 shapes; optimized builds passed those samples.
The probe's `--assert-limits` mode now requires normal diagnostic exits at excess
depth and successful compilation at the supported boundary. The 128 bound is
implemented; it is not a universal host-stack/memory safety guarantee.

## Findings from implementation review

- Source grouping and call arguments recursively enter the expression parser.
  Long binary chains also create deeply nested ASTs even when parser recursion
  stays shallow. Semantic traversal, AST cloning, lowering and drop behavior must
  be considered together; a parentheses-only guard is insufficient.
  Lowering now uses a worklist, and semantic analysis preflights caller-built
  depth before recursive validation/cloning. Public AST ownership/drop and
  caller-invoked cloning remain unresolved.
- Application inspection recursively traverses blocks, expressions and scopes.
  It requires independent protection until unified structured project checking
  replaces the separate subset.
- Source discovery used one native stack frame per directory. It now uses a
  pending-directory worklist; source paths remain sorted and E4006 I/O diagnostics
  are retained. This removes recursion, not filesystem/path or memory limits.
- Both engines already enforce 256 active user-function frames. That bounds
  nesting, not total work, captured output, string growth or memory consumption.
- Public-IR argument counts are checked against the operand stack before argument
  allocation. Other allocations still use ordinary Rust/JavaScript allocation;
  allocation failure is not a fully recoverable Sovra runtime contract.
- The interpreter collects output in memory and JavaScript uses svrOutput. A
  shallow program can still grow data/output substantially. There is no instruction
  budget, wall-clock cancellation or output-byte budget yet.

## Next design work

The approved policy now covers source and application parsing, including flat
binary chains, nested calls, grouping and application blocks. Validate the fixed
threshold on all supported platforms before claiming cross-platform stack safety.
Boundary, boundary-plus-one and large-input tests cover normal diagnostic exits;
the large crash regressions run in subprocesses to isolate failures.
Diagnostics identify the limit and source location without cascading errors.

Separately specify runtime execution budgets, output limits and cancellation.
Do not infer a production-safe memory model from an instruction/depth limit.
Public APIs must preserve existing entry points or provide a documented migration.

Adding rejection limits changes compatibility. AGENTS.md requires:
“Document and pause before fundamental syntax, memory-model, type-semantics or
compatibility decisions.” Present a concrete threshold/API proposal with measured
platform evidence for review before enforcing it. Iterative implementation changes
that preserve accepted programs can proceed without choosing new language limits.
