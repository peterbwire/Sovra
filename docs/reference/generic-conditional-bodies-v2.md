# Symbolic conditional body checking

`check_generic_copy_bodies` accepts `if`/`else` with a concrete `Bool` condition.
Use existing parenthesized identifier-condition syntax, for example
`if (flag) { return x; } else { return y; }`, to disambiguate record literals.
Transparent Bool aliases qualify; opaque parameters, including Key parameters,
do not. Both branches are checked regardless of a literal condition or an earlier
return. [While loops](generic-loop-bodies-v2.md) are now checked symbolically too;
matches and generic execution remain outside this stage.

Each branch gets its own lexical scope. Bindings introduced or shadowed in a
branch do not leak into its sibling or enclosing block. Existing mutable locals
can be reassigned with the same fixed-type rules. Types do not merge or change
based on branch assignments.

A conditional guarantees a return only when an explicit else exists and both
branches guarantee a return. Nested conditionals follow the same rule. An `if`
without else never proves a return by itself, even with a literal true condition.
A later return or an earlier guaranteed return still covers the enclosing block.
Unit fallthrough remains distinct from explicit return coverage.

`GenericCopyStatement::If` stores the condition expression ID, original full-source
range, and flat source-order statement ranges for then/else branches. Missing
else differs from an empty else. Branch ranges include their nested facts.
These facts are not executable jumps: consuming every flat statement sequentially
would incorrectly execute both branches. Future lowering must preserve branch
selection and local scope.

Semantic traversal uses an explicit heap worklist, not recursive checker calls.
Existing source/token/AST/parser limits bound admission and nesting. Every branch
statement and expression consumes the existing cumulative budgets. Scope snapshots
copy shared name/identity entries, charging each copied entry to semantic work
before allocation, including restoration for the sibling branch. Proof storage
remains flat and no recursive proof destructor is introduced. These limits are
logical admission, not physical memory isolation or hard CPU deadlines.

`tests/generic_conditional_bodies_v2.rs` covers nested return coverage, branch
scope and shadowing, fixed mutable parent identity, invalid dead branches,
concrete conditions, empty/missing else, original Unicode source ranges, nested
traversal and exact/one-under resource quotas.
See [symbolic bodies](generic-copy-bodies-v2.md).
