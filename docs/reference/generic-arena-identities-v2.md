# Symbolic body arena identity

`GenericCopyBindingId` and `GenericCopyExpressionId` now identify both an original
checked signature and one immutable body arena. A fresh successful body check
has its own arena even when it reuses the exact signature table and produces
identical source-order ordinals. `binding()` and `expression()` validate both
owners before indexing.

This prevents accidental cross-arena references when a later stage introduces
separate call-source expression facts. Matching signature names, IDs, ordinals
or types is insufficient to transfer binding/expression IDs between arenas.
Existing value-argument matching delegates to the supplied body's arena-aware
lookup and rejects foreign values from another check of the same caller.

Cloning a checked body shares the original immutable arena and preserves its IDs.
Dropping the original owner does not invalidate the clone. Function/signature
ownership and source ranges remain unchanged; ordinals are labels within an
arena and do not provide a constructor for private IDs.

The process-local atomic allocator uses checked compare/exchange and refuses
identity exhaustion without wrapping or reusing an owner. Failed checks can
consume an identity; no partial body is published. These opaque IDs are not
persistent artifacts, source-stable keys, runtime addresses or executable handles.
This stage changes compiler proof identity only; no runtime/CLI/ownership behavior
or generic execution is added.

Verification includes same-signature/different-arena binding/expression rejection,
cloned arena retention, call-value rejection across rechecked callers, and an
isolated exhaustion test that does not mutate the live global allocator.
See [symbolic bodies](generic-copy-bodies-v2.md) and
[value matching](generic-call-values-v2.md).
