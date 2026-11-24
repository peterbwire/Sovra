# Symbolic expression statements

`check_generic_copy_bodies` accepts a supported expression followed by `;`.
The expression is fully checked with its original result type, binding/operand
identity and source span. `GenericCopyStatement::Expression` retains its checked
expression ID. The result is discarded without an implicit conversion or binding.

Opaque parameter reads, existing literal/equality/numeric/Boolean/String
expressions qualify. Calls, uncontextualized construction, other unsupported
expressions still fail. Discarding a result never bypasses name or type checking,
including after a return or on the skipped side of a logical expression.

Expression statements do not satisfy a non-Unit function's return requirement.
Unit fallthrough and explicit returns retain their existing distinction.
Every statement and expression consumes existing cumulative admission, including
semantic work. An assignment remains a separate statement with mutability checks.

This completes discarded-result handling for the supported straight-line profile;
it does not add call effects, generic execution, specialization
or executable IR. Future lowering must preserve evaluation and runtime failures
even when an expression result is discarded.

`tests/generic_expression_statements_v2.rs` covers result/source retention,
return coverage, invalid and unreachable expressions, shadowing/mutable binding
identity and exact/one-under quotas.
See [symbolic bodies](generic-copy-bodies-v2.md).
