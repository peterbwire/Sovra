# Versioned enum type facts

Status: **Partial E1 foundation.** `compiler::v2` provides checked nominal enum
metadata beside unchanged v1 compiler stages. It does not execute enums, check
function bodies, or establish whole-program success. A separate
[match syntax API](match-syntax-v2.md) retains ordinary arm bodies, and a separate
[closed-scope body checker](match-bodies-v2.md) validates them in supplied scopes.

`check_enum_types` admits root declarations; `check_scoped_enum_types` places each
fragment in an existing inline module. Each fragment contains exactly one enum.
Payloads resolve through context aliases, records, arrays and nominal enum names.
Empty enums, duplicate variants, unresolved types and alias cycles are refused.
Module facts use qualified names; an unavailable module never falls back to root.
Source labels are opaque diagnostic provenance, not package permissions.

Returned immutable environments own their facts. Enum/type IDs belong to one
environment; IDs from another environment are refused. Clones retain the same
identity. Recursive nominal references do not unfold their payloads recursively.

`check_constructor` accepts closed argument type annotations and returns a type
plan. It distinguishes nullary variants from a Unit payload, preserves nominal
identity, and records only existing Int-to-Float widening, including array depth.
It does not infer an expression's type or construct a runtime value.
`check_match` checks supplied arm shapes for exact coverage, duplicates, variant
ownership and payload binding arity. Its plan does not validate arm bodies,
scoped binding immutability or return flow.

`EnumTypeLimits` bounds declarations, aggregate source bytes, context AST nodes,
retained metadata nodes/text and per-fragment syntax admission. Limits include
private type probes, retained type-name bindings and real record-field tables. Refusal returns no partial
checked environment. Diagnostic spans map to their owning original fragment or
context. These logical admission limits do not measure process heap/RSS, core
checker scratch allocations or impose hard interruption.

Executable source syntax, checked bodies, versioned IR/values, runtime fuel,
package visibility, JavaScript/native execution and tooling integration remain
required before E1 is complete. See the [approved completion contract](../design/G13_COMPLETION_CONTRACT.md).
