# Caller-scope explicit type-argument checking

`check_generic_type_arguments` resolves every admitted explicit `::<T,E>` argument
against complete checked type definitions and one original caller parameter owner.
Names, type-head arities, transparent aliases and declared `std::Key` constraints
are checked before returning sealed `CheckedGenericTypeArguments`. Arguments
retain source order, exact caller parameter identities and the checked environment.
Unknown names and invalid arguments fail even when an earlier argument is valid.

The input list's postorder arguments occupy disjoint contiguous subarenas.
A private admitted copy rebases child indices without rewriting source text.
Each argument retains the complete shared original list source and its original
node ranges; its token count describes that admitted original list, not a new
lexer run. Expansion origins continue to use original list or alias-target
sources, accessible with `ExpandedGenericType::node_source`.

`GenericArgumentCheckLimits` independently caps arguments, full input bytes/tokens,
copied syntax nodes/edges/names, generated expanded nodes/edges and semantic work.
Input edges include list-to-root links. Complete copy admission occurs before
syntax copying; copy work charges node, edge and name-byte totals. Per-argument
expansion/depth/input-head limits are additionally reduced by cumulative remaining
node/edge/work budgets. `nodes()`, `edges()` and `work()` report generated/consumed
usage; unused eager alias argument nodes remain retained and charged.

`GenericArgumentCheckError::Argument` identifies the original argument index and
full list-source range while retaining the inner input/alias diagnostic. No
partial checked list is published on failure. Sources and caller identity remain
owned after original inputs are dropped. Logical admission is not physical
process isolation or a hard CPU deadline.

This proves caller-scoped type arguments, **not a checked call**. [Callee parameter count/bound matching](generic-call-binding-v2.md) is now
available as a separate sealed stage. Callee lookup/visibility, signature
substitution, value argument matching, body call facts,
specialization and executable integration remain open. A type-head Key constraint
is checked here; a particular callee's Key requirements remain a later obligation.
No generic CLI execution or implicit type argument inference is enabled.

`tests/generic_argument_checking_v2.rs` covers caller identities, alias/order
preservation, name/arity/Key failures and positives, original Unicode diagnostic
ranges, source retention, every exact/one-under cumulative budget and independent
per-argument caps.
See [argument syntax](generic-type-arguments-v2.md).
