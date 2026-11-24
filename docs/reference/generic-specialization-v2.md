# Finite concrete generic call discovery

`discover_generic_specializations` discovers a bounded concrete function-call
closure from a nongeneric root in `CheckedGenericCopyBodies`. Every supplied body
must have passed symbolic checking first. This is a compiler library stage; it
produces neither rechecked concrete bodies nor executable IR.

`parse_specialization_root` admits an ordinary nongeneric function into the same
source-owned header representation as generic functions. It shares the function
source/token/name/annotation/body limits and allocates a distinct empty parameter
owner. Its `parameter_range()` is an empty original-source range at `(`. It keeps
the body opaque until body checking, requires explicit value annotations, and
refuses a type parameter list. The existing `parse_generic_function` still
requires a nonempty type parameter list. A plain `main` can enter this library
pipeline; generic `main` still refuses. Application entry convention checking and
public Program/CLI generic execution are not enabled by these APIs.

For example, checked source headers/bodies for these functions discover four
instances: `root`, `identity<Int>`, `outer<String>` and `identity<String>`:

```svr
fn root() {
    identity::<Int>(1);
    identity::<Int>(2);
    outer::<String>("hello");
}
fn identity<T>(value:T)->T { return value; }
fn outer<U>(value:U)->U { return identity::<U>(value); }
```

The root ID must belong to the retained checked signature table and have no type
parameters. Root value parameters are permitted; graph discovery does not invoke
the root or supply runtime values. All source call expressions in each discovered
instance are visited, including unreachable branches and statements. Uncalled
generic bodies remain symbolically checked but do not receive concrete instances.

Each explicit call's type arguments are substituted using the current function's
exact parameter owner and concrete argument vector. Generated arrays/nominal
applications preserve original caller and alias provenance. All generated nodes,
including unused eager nodes, must be concrete and within the structural depth
limit. Nominal fields/layouts are not expanded by this function-call discovery.

The cache combines exact original function identity with normalized source-order
type arguments. Transparent aliases reuse the same instance; different nominal
heads and argument order remain distinct. Exact flat keys are compared with
charged linear lookups, without relying on hash-only identity or source spelling.
An instance is registered before its calls are processed, allowing direct and
mutual recursion with unchanged arguments. Expanding recursion must close within
the configured limits or refuse without a partial result or fallback.

`CheckedGenericSpecializations` retains the complete symbolic bodies and a flat,
deterministically ordered instance list. The root is index zero. Each instance
retains its original function ID, concrete type arguments and source call edges.
Edges carry a symbolic body expression ID and a target index local to this graph.
Clones preserve the source/table/body arena owners; indices have no cross-graph
identity contract.

`GenericSpecializationLimits::DEFAULT` admits at most 1,024 instances, including
the nongeneric root. `PACKAGE` selects 4,096 instances but does not load or link a
package graph. Both retain finite argument/call/generated-type/key/work limits.
Structural depth must be in 1..=128; per-argument substitution and key profiles
are additionally capped by the remaining cumulative allowances. Duplicate call
candidates consume generated argument/type/key/work quotas before cache reuse.
`usage()` reports exact totals, including duplicate candidates. These are logical
compiler allowances, not CPU deadlines or a process sandbox.

Tests: `tests/generic_specialization_v2.rs`. The separate
[concrete body rechecker](generic-concrete-bodies-v2.md) now checks signatures,
types and operation/context edges for this graph. Remaining gates include
constructor/match/runtime layout plans,
executable lowering and backend integration, package/source linking, standard
Option/Result and structured error propagation. G13 E2 remains incomplete.
