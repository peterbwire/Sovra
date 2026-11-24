# Bounded generic type equivalence and normalized keys

`compare_generic_types(left, right, limits)` compares two complete alias-expanded
type roots. `generic_type_key(type, limits)` constructs an exact normalized key
usable in an in-process Rust HashMap/HashSet. Both take existing immutable
`ExpandedGenericType` proofs; neither resolves untrusted raw syntax itself.

## Exact type meaning

Comparison follows explicit roots rather than the last node or complete arena.
Unused eager argument nodes and source provenance do not change type equality.
Shared and duplicated subgraphs compare by their structure. Primitive kinds,
ordered nominal arguments and array elements participate; Int and Float remain
distinct without an implicit conversion. Nominal declaration registry IDs and
parameter-owner IDs remain significant. Identical local node indices or written
names in separate contexts cannot establish equality. Parameter alpha-renaming
and generic-call inference are not introduced.

The comparator uses an iterative pair worklist with a memo of visited source-node
pairs. A successful `GenericTypeComparison` exposes the exact decision and logical
pair/edge/work counts. Exhaustion returns an error, never an assumed decision or
fallback. Default caps are 4,096 distinct pairs, 8,192 queued child-pair edges and
32,768 pair/queue operations.

## Normalized compiler keys

Key construction traverses the reachable root in deterministic left-to-right
postorder and interns identical structural subtypes. Canonical node indices refer
only to the normalized key. An index and output share immutable node allocations;
no type names, source text or provenance are copied into keys. Input memo entries
are separately bounded, so graph duplication cannot hide behind a small canonical
result. Traversal and ownership/drop are flat rather than recursively owned trees.

`GenericTypeKey` equality compares the complete normalized structure and opaque
owner IDs. Hash collisions cannot establish equality. Construction statistics
are separate and do not affect key equality. HashMap randomization is not used
to choose output order; keys for equal structures remain equal across repeated
normalization calls with different hash-table seeds.

Default caps are 4,096 visited input nodes, 4,096 canonical nodes, 8,192 canonical
edges, 8,192 pending visit/finish frames and 65,536 traversal/normalization steps.
Pending admission also bounds temporary argument width. Exact and one-under tests
cover each dimension. Memo/index/output growth is admitted before retention; no
partial successful key escapes. Logical limits are not CPU/RSS isolation.

These keys are **in-process compiler metadata**, not a persistent cache format,
serialized native type codec or independent proof of executable type validity.
Registry/parameter identities are request/process-local. Keys do not keep source
contexts alive; code-generation caches must retain the relevant checked contexts
and function/instance identities. No cross-process/version compatibility is claimed.
The compiler type DAG introduces no runtime references or ownership changes.

## Remaining work

This supplies equality and specialization-key prerequisites. It does not itself
parse/check bodies, instantiate code, construct values or enable generic execution.
The separate [initial body checker](generic-copy-bodies-v2.md) now uses exact
equivalence for symbolic copies, returns and equality. Symbolic body checking, explicit call/type-argument plans,
bounded specialization, runtime constructor/match plans and remaining G13 phases
stay open. Existing CLI/source execution still refuses unsupported generics.

See [checked signatures](generic-signatures-v2.md),
[alias expansion](generic-alias-expansion-v2.md) and the approved
[G13 contract](../design/G13_COMPLETION_CONTRACT.md).
