# ADR 0007: Bound compiler structural depth

Status: **Accepted option A; implemented for source parsing and application
inspection.** The user explicitly directed “do the 128 then.” Cross-platform
boundary evidence and public-AST/resource work remain outstanding.

## Reproduced failure

The isolated `scripts/probe-compiler-depth.mjs` probe generates grouping, nested
calls and left-associated binary expressions. On the current Windows GNU debug
build, `check` and `build` succeeded at sampled depths 32, 128, 256 and 512, but
all six depth-2048 runs aborted with stack overflow (exit 3221225725). These are
sample points, not an exact crash threshold or a platform-independent bound.
The script records subprocess failures and timeouts; its own successful exit
means the probe finished, not that the compiler passed a safety gate.

The matching optimized build passed all 30 sampled checks through depth 2048.
Both builds used rustc 1.98.1, x86_64-pc-windows-gnu. This difference confirms
configuration sensitivity; it does not establish unlimited release-build safety.
Results are retained locally in target/depth-probe/results-debug.json and
results-release.json. Other operating systems and small-stack threads are unmeasured.

## Accepted option A: bounded source compilation

Enforce a fixed maximum structural depth of **128** for the supported source
pipeline, with a normal diagnostic instead of stack overflow. This is a
conservative starting ceiling below the sampled passing depth; it still requires
boundary verification on supported platforms and test-thread stacks.

- Bound active nested grouping/call parsing before entering recursive work.
  An iterative token preflight allows at most 128 open parentheses; application
  inspection separately allows 128 open braces including the outer body block.
  Strings and comments do not consume nesting depth.
  Follow-up for the implemented compound/control-flow syntax: source parsing
  also bounds open square brackets and braces to 128. Array/record constructors,
  field reads and indexing enforce retained-tree depth; source braces include
  enclosing module/function blocks. The semantic AST preflight visits nested
  branch/loop bodies and assignment targets, with a separate block-depth guard.
  Count source nesting separately from precedence-helper implementation frames.
- Bound retained expression-tree depth as well: leaves count as one; binary
  nodes count one plus maximum child depth; calls count one plus maximum callee
  and argument depth. Empty grouping disappears from AST but still consumes
  parser nesting. A flat binary chain must not evade the retained-tree guard.
- Reject before constructing the over-limit subtree; unwinding/dropping a huge
  already-built tree is too late. Preserve wide parameter/argument/statement
  lists when their depth is within bounds; this is not a file-size budget.
- Use E2007 for source-parser depth rejection, at the offending token, with
  the limit in the message. Stop that parse safely without a diagnostic cascade.
  The failed parse returns one E2007 without cascading recovery errors.
- Apply equivalent accounting to application inspection. Unsupported excessive
  depth must produce an explicit partial-inspection reason and CLI E4096, never
  success with omitted calls. Block nesting requires its own guarded counter.
- Existing parser entry points use the fixed bound. Do not add an unrestricted
  “disable safety” switch. A future configurable ceiling requires measured stack
  margins or iterative implementations, not an untested larger integer.

This changes compatibility for deeply nested sources that previously happened
to compile. Ordinary source syntax and meaning below the limit remain unchanged.
It does not bound total CPU, allocations, output, or runtime call count; the
existing 256 active runtime frames are a separate contract.

## Public AST and remaining production scope

Implemented follow-up: semantic analysis performs an iterative borrowed depth
check before recursive work or cloning, returning E3018 for over-depth ASTs.
`lower_program` inherits this check. No public stage signature changed; direct
`lower` still requires a semantically validated `TypedProgram`.

Manually constructed public ASTs can bypass the parser and have recursive
clone/traversal/drop paths. Source guards must not be advertised as a full public
AST safety guarantee. Audit these APIs separately, using iterative depth checks
before recursive semantic work and an ownership/drop design before promising
arbitrarily deep externally constructed trees are safe. Preserve existing stage
signatures; document any required additive checked entry points before coding.

## Alternative B: iterative compiler structures before imposing limits

Refactor parsing and every downstream recursive AST operation, including drop,
to explicit worklists or arena-backed storage, then define memory/work budgets.
This avoids a fixed source-depth restriction but is a larger architectural change
requiring API/ownership review. Fixing only parser recursion would leave the
measured binary-AST failure path unresolved.

## Required implementation validation

Boundary/boundary-plus-one tests for each depth counter; long flat chains,
nested calls, grouping and mixed shapes; supported shallow wide inputs; JSON
diagnostic locations; debug/release subprocess checks for the reproduced crashes;
Windows/Linux/macOS and MSRV CI. Re-run the probe and require normal diagnostic
exits at excess depth. Keep the full existing suite passing without skips.

## Decision boundary

AGENTS.md requires: “Document and pause before fundamental syntax, memory-model,
type-semantics or compatibility decisions.” Option A rejects previously accepted
deep programs. The user's explicit approval now authorizes option A.
Production-readiness intent alone was not treated as an implicit depth policy.
