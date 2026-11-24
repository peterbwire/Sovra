# Generic callee type-argument matching

`bind_generic_type_arguments` matches checked explicit arguments to a root callee
in an original checked signature table. It verifies both caller/callee IDs belong
to that table, the prepared type/alias environment is shared, and the arguments
were checked under the caller's exact parameter identities. Matching names or
bounds from a freshly parsed scope are insufficient.

Explicit argument count must equal the callee's declared type parameter count.
Every callee Key requirement must be proved by the expanded argument: concrete
Int/Bool/String or a caller-owned parameter with a Key bound. Float, Unit,
arrays and unconstrained opaque parameters refuse Key requirements. Aliases use
their expanded type. No inference or unchecked generic capability is introduced.

Sealed `CheckedGenericCallBinding` retains the original table, caller/callee IDs,
checked argument list and source-order callee parameter-to-argument mappings.
`GenericCallBindingLimits` independently bounds mapping count and matching work;
argument resolution has its own earlier cumulative budgets. Every owner/scope/
parameter/bound check consumes matching work. No partial result is published.
Bound failures retain the original explicit-list argument range and callee
parameter ID. Results remain owned after original tables/arguments are dropped.

This proves type-parameter matching, **not a checked value call**. [Signature substitution](generic-signature-instantiation-v2.md) is available as
a separate bounded stage. Matching performs no name-based callee lookup or module
visibility resolution,
value argument matching, body call planning, recursion specialization or execution.
The current profile is one supplied root signature table; module/package calls
need their own checked scope/visibility support. Runtime and CLI behavior remain
unchanged. Future body integration must meter repeated bindings cumulatively.

`tests/generic_call_binding_v2.rs` covers concrete/alias/opaque Key proofs,
foreign table/environment/caller-scope rejection, exact arity and parameter order,
original Unicode error ranges, exact/one-under mapping/work caps and retained owners.
See [caller-scope argument checking](generic-argument-checking-v2.md).
