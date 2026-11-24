# Symbolic while body checking

`check_generic_copy_bodies` accepts `while` with a concrete `Bool` condition,
including transparent Bool aliases. Use parentheses around identifier conditions,
as in `while (running) { running = false; }`, to disambiguate record literals in
the existing parser. There is no truthiness or opaque Key-to-Bool assumption.

The body has an isolated lexical scope. Loop-local declarations and shadowing
do not escape; outer mutable locals retain their binding identity and fixed type.
The condition is checked in the enclosing scope before body declarations.
Every body is checked, including literal-false loops and loops after a return.

A loop never adds guaranteed return coverage, including `while true` with a
returning body. A non-Unit function still needs enclosing all-path return coverage.
An earlier guaranteed return remains valid after checking an unreachable loop.
This is deliberately conservative and makes no termination claim.

`GenericCopyStatement::While` records the condition expression ID, full original
source span and flat body statement range including nested facts. Semantic
checking traverses the body once using an explicit heap worklist; it never runs
the loop. Source/token/AST/parser bounds remain active. Loop statements, condition
expressions, body facts, bindings and semantic work use existing cumulative
budgets. Scope entries are charged to work before copying shared identities;
restoration moves the saved scope back. Proof storage remains flat.

These are symbolic typing facts, not executable backedges or a runtime budget.
Future lowering must re-evaluate the condition each iteration, select whether to
enter the body, preserve lexical scope and returns, and apply existing runtime
fuel/cancellation checks. Sequentially consuming every flat fact would be wrong.
Generic calls, specialization and executable integration remain open. No break,
continue, new loop syntax or runtime reference semantics are introduced.

`tests/generic_loop_bodies_v2.rs` covers source and binding identity, flat nested
ranges, conservative return coverage, restored scopes, immutable writes, opaque
and non-Bool conditions, dead/unreachable errors, nested traversal and exact/
one-under cumulative quotas.
See [conditionals](generic-conditional-bodies-v2.md) and
[symbolic bodies](generic-copy-bodies-v2.md).
