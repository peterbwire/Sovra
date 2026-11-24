# Exhaustive generic enum pattern headers

`check_generic_enum_patterns` checks standalone match pattern headers against an
explicitly supplied binding ID in a checked body arena. The selected binding must
have a known nominal enum type, and the parsed scrutinee identifier must match its
name. Foreign arena IDs, non-enum bindings, foreign/qualified enum patterns,
unknown/duplicate/missing variants and wrong payload binding counts are refused.
The source is parsed internally; caller-built public raw match ASTs are not proofs.

Each variant must occur exactly once. Nullary patterns bind nothing; a declared
Unit payload still requires a binding and produces a Unit type context. Payload
contexts use the exact selected enum arguments and retain opaque caller parameter
IDs and alias provenance. `node_source` selects each generated node's source.
Binding names may shadow outer names and repeat across different arms; they are
expected immutable arm-local snapshots when a body checker allocates them.
No fake body binding IDs or scrutinee read expressions are created here.

Sealed `CheckedGenericEnumPatterns` retains the original standalone match source,
selected checked body/binding, exact nominal declaration and source-order
`GenericEnumPatternArm` records. Each arm retains original match-source range,
variant index and optional payload name/span/type. Binding declaration spans remain
in the original body's function source, while pattern spans belong to `source()`.
Clones retain these owners after original bodies and parsed input are dropped.

`GenericEnumPatternLimits` reuses bounded original match syntax admission for
source bytes, non-EOF lexer tokens, arm count and header-name bytes. Temporary
ordinary arm AST construction is bounded by source/tokens and parser depth.
Bodies are discarded with iterative statement/expression cleanup; no arm ASTs
are retained as checked evidence. Coverage lookup, name-copy and payload context
work is cumulative across arms, as are generated nodes/edges. Independent payload
node/edge/work/depth caps remain active. Failure publishes no partial coverage.
These logical limits are not physical process isolation or hard CPU deadlines.

This is **pattern/type coverage, not a checked match body**. The binding is selected
by the caller; the API does not prove it is active at a particular source site.
Arm statements are parsed with ordinary grammar but deliberately not typed, so an
unknown name in a body can pass this stage. Active lexical scope association,
immutable payload ID allocation, arm body typing, all-arm return coverage and
nested matching are supplied by the separate generic match-body checker below.
Executable match plans remain open. Generic execution through CLI is not enabled.

`tests/generic_enum_patterns_v2.rs` covers exhaustive order/types, missing/duplicate/
foreign/unknown and arity errors, nullary/Unit/shadowing, exact arena/name/type checks,
Unicode ranges, alias sources, deliberately untyped bodies, retained owners,
exact/one-under syntax/context/work caps and unsupported pattern forms.
See [enum constructors](generic-enum-construction-v2.md).

The separate [generic match-body checker](generic-match-bodies-v2.md) now uses
these shared coverage rules at active source sites and checks arm scopes, bodies
and return coverage. This standalone header API retains its narrower contract.
