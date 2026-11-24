# Concrete generic body rechecking

`recheck_generic_concrete_bodies` consumes a sealed finite
[specialization graph](generic-specialization-v2.md), substitutes the concrete
signature/binding/expression types for every instance, and rechecks retained
operation and context edges. It is a compiler library stage before executable
lowering. It does not emit instructions, encode literal values, construct runtime
layouts or enable generic Program/CLI execution.

Every instance receives concrete value-parameter and result types and a concrete
type for each original binding and expression. Caller/alias provenance and nominal
declaration identities survive substitution. Every final substituted graph, including
its eager unused nodes, must be concrete and within the selected depth ceiling. Function and
nominal Key constraints are checked again against Int, Bool and String; Byte
remains outside the current generic primitive implementation.

The rechecker validates these concrete contexts:

- Literal types, binding reads, equality and the retained scalar conversions.
- Numeric, String and Bool operations, with their concrete operand/result types.
- Nested/empty typed arrays, element reads and mutable local/indexed replacement.
- Record construction/complete field coverage and substituted field reads.
- Enum construction and nullary versus Unit-payload shape.
- Exhaustive matching, nominal variants and immutable payload binding types/origins.
- Local initialization, conditional/loop guards, discarded expressions and returns.
- Call arguments/results against the graph target's concrete signature.

A parameter-independent field can retain unused declaration-owned parameters in
its symbolic arena when an alias discards an argument. If an input type contains
foreign owners, a bounded original-root projection copies its reachable semantic
nodes before exact-owner substitution. Unreachable owners are removed; any
reachable foreign parameter still refuses. Projection copies preserve per-node
sources and consume the same type-fact/node/edge/work allowances as substitution.

Stored conversion edges are checked as written. The stage does not silently
recompute a mismatching conversion. All concrete signature/type tables are built
before operation checks, so direct and mutual recursive calls use finite indexed
targets rather than recursively rechecking callees. All retained operations are
checked, including unreachable statements and branches.

Original lexical scopes, source ownership, binding mutability, operand selection
and return-flow coverage remain supplied by the sealed symbolic body proof. The
source and control-flow shape are unchanged. Implicit Unit fallthrough is checked
against the concrete result. Original annotations remain symbolic source metadata;
future lowering must use the concrete type tables for its selected instance.

`CheckedConcreteGenericBodies` retains the graph and `ConcreteGenericBody` results
in graph instance order. `parameter_types()` and `return_type()` expose concrete
signatures. `binding_type(id)` and `expression_type(id)` validate the original
body arena before returning a concrete fact. Multiple instances of one function
share the original symbolic IDs while carrying different types: select the
concrete body by graph instance index before looking up an ID. Clones retain the
original source/table/body owners; foreign body IDs refuse.

`GenericConcreteBodyLimits` bounds instances, generated type facts/nodes/edges,
comparison pairs/edges and logical work across the entire graph. Temporary
substituted field/constructor/payload contexts share these totals. Per-type
substitution and per-comparison profiles are additionally capped by the remaining
allowances. Structural depth must be in 1..=128. An empty Unit root can require
zero generated nodes/edges while still consuming a result type fact and work.
Any refusal publishes no partial rechecked proof. These are logical compilation
limits, not CPU deadlines or a process sandbox.

Tests: `tests/generic_concrete_bodies_v2.rs` and private negative regressions in
`generic_concrete_bodies.rs` that reject inconsistent result facts, stale call
targets, missing call edges, reachable foreign parameter owners and incorrect
scalar conversion edges.

The separate [literal decoder](generic-literals-v2.md) now retains exact primitive
payloads for these concrete bodies. Remaining gates include nominal/runtime layout and constructor/
match plans, executable lowering and backend integration, package/source linking,
standard Option/Result and structured propagation. Runtime bounds, overflow,
short-circuit execution and snapshot allocation policy still require executable
integration. G13 E2 remains incomplete.
