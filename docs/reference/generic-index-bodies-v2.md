# Symbolic read-only array indexing

`check_generic_copy_bodies` accepts `array[index]` when the target resolves to an
array and the index resolves to concrete `Int`. The result retains the exact
expanded element type, including owner-scoped generic parameters, nested arrays,
nominal values and transparent aliases. No numeric-index widening, truthiness,
String indexing or opaque array/index assumptions are introduced.

`GenericCopyExpressionKind::Index` stores both checked operand IDs and the
original full-source expression range. Array element projection privately selects
an already-validated array edge; it shares the immutable expanded arena and
retains the original annotation, parameter and alias provenance. It does not
copy nodes, substitute parameters, publish an arbitrary-root constructor or
change the expansion's admission history. Root-based equivalence continues to
compare the selected element, not unused nodes elsewhere in the shared arena.

Index facts consume existing cumulative expression and semantic work admission.
Any subsequent type comparison consumes existing pair/edge/work budgets. Original
source/token/AST limits remain active, and expression traversal is iterative.
[Context-typed array construction](generic-array-literals-v2.md) is now supported; [indexed assignment](generic-index-assignment-v2.md)
now supports direct mutable array locals.

Typing is not bounds verification or execution: an enormous Int literal index
can type-check. Future lowering must preserve target/index evaluation order,
existing bounds failures, value ownership and runtime fuel. These facts do not
prove a read succeeds or remove runtime checks. Generic calls, specialization
and executable integration remain open.

`tests/generic_index_bodies_v2.rs` covers original Unicode source/operand identity,
opaque and nested elements, typed locals, numeric contexts, exact parameter
identity, transparent array aliases, invalid operands, invalid writes,
non-evaluation and exact/one-under cumulative quotas.
See [symbolic bodies](generic-copy-bodies-v2.md).
