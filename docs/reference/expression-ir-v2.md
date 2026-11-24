# Versioned expression register IR

Status: **Partial E1 instruction-emission foundation.**
`MappedCallable::emit_expression_ir` emits and validates backend-neutral register
instructions from the [checked structural map](callable-mapping-v2.md). This is
an expression graph, not an executable function/program or a CLI/native backend.

Operations cover literals, source-resolved binding snapshots, binary operators,
canonical standard/self/enum-constructor calls, arrays, nominal records, field and
index reads, and explicit checked type-context conversions. Each instruction
has one graph-owned result register and a borrowed checked type. Intrinsic values
retain their mapped source and checked expression ID; conversion temporaries
retain the original value source and checked context index. No new type inference,
lexical name resolution, copied literal text or copied type graph is introduced.

Operand trees are emitted iteratively, left to right. Each operand's conversion
is emitted immediately after that operand, before the next sibling is evaluated.
Array items, call arguments, record fields and mixed numeric operands consume
their corresponding checked contexts. Identity contexts remain explicit alongside
scalar/recursive widening. These instructions do not yet define or enforce
runtime materialization charges; runtime policy integration remains required.

Independent roots preserve structural source order, with operand dependencies
before their consumer within each tree. Branch conditions, arm bodies and loop
bodies remain distinct roots: the graph must **not** be executed as if all roots
run unconditionally. Initialization, assignment, indexed-assignment and return
contexts are retained explicitly as deferred indexes for statement emission.
The separate [function IR layer](function-ir-v2.md) now emits stores, scope
cleanup, branches/loops, match dispatch/extraction and returns, with bounded
reachable-flow validation. Expression IR alone remains non-executable.

Validation checks graph-local register ownership, unique ordered definitions,
backward operand dependencies, checked output/operand types, source operation
shape, binding/call target identities, conversion context placement and complete
expression/context/root coverage. No partial graph or legacy lowering fallback
is returned. Public getters are immutable; graph IDs from another emission are
refused, while clones preserve identity. The graph borrows its structural map,
which in turn borrows the immutable checked function.

`ExpressionIrLimits` bounds instructions, values, cumulative operand references,
roots, deferred boundary contexts and validation steps. A validation step counts
an instruction, input reference, root or deferred context. Graph indexes and
traversal scratch are bounded by the already admitted structural map; validation
steps do not charge initial index construction. Operation text/types are borrowed;
temporary operand lists and compiler scratch are outside retained graph counters.
These are logical admission/work counters, not physical memory or total-work
bounds. Zero caps are real; even an empty function can emit an empty expression
graph with zero expression allowances.

Function-level IR emission, executable Rust/JavaScript/native runtimes, fuel/
control integration, nested matches, cross-function/program/package and tooling
support remain open under the [approved completion contract](../design/G13_COMPLETION_CONTRACT.md).
Existing public v1 IR/Value/native formats and CLI behavior remain unchanged.
