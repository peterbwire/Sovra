# Source-driven enum function checking

Status: **Partial E1 semantic foundation.**
`CheckedEnumTypes::check_function_source` checks exactly one source function
against an existing checked enum/type environment:

```svr
fn number(input: Outcome) -> Int {
    let local = input;
    match local {
        Outcome::Number(value) => { return value; },
        Outcome::Empty() => { return 0; },
    }
}
```

This API derives annotated parameters, inferred locals and mutability from source;
it does not accept caller-supplied outer local facts. The environment supplies
canonical aliases, records and nominal enums. Match scrutinees must resolve to
bound enum values. Every match requires exact nominal variant coverage and
correct binding arity. Each arm gets an isolated scope with an immutable typed
payload binding; arm locals never enter the function continuation.

The function body is a separate `EnumFunctionSyntax` with
`EnumFunctionStatement::Core` and `::Match`. Matches are never encoded as v1
statements or returned as successful v1 programs. Original spans are retained
through private admitted-token parser entries. Ordinary syntax reuses the core
parser; `match` followed by an identifier and `{` selects a direct match.
Ordinary contextual identifiers remain compatible. As in the core parser, an
exported function's span starts at `fn`.

Core statements retain existing scope, type, numeric widening, standard-call and
return rules. Required parameter annotations, duplicate parameters, unresolved
types, builtin function-name collisions and `main` signature restrictions are
checked. A direct exhaustive match proves return only if every arm returns.
A partial match can be followed by a returning continuation; a loop alone does
not prove return. Non-Unit functions without guaranteed return receive `E3013`.
Unreachable statements still receive checks, consistent with the core checker.

`EnumFunctionLimits` bounds original source/tokens/parser diagnostics, parameters,
direct statements, cumulative match arms/header text, cumulative initial bindings
copied into arm scopes, and source-label bytes. Arm/header counters are shared
across all direct matches; copied scopes include source parameters and preceding
locals plus payload bindings. Parameters are capped after syntax parsing but
before semantic scope construction. Nested ordinary statements use frontend
source/token admission and the existing depth bound rather than the direct
statement counter. Source labels are opaque provenance, not filesystem identity.
Core semantic diagnostics retain their existing fixed ceiling. These are logical
admission limits, not physical memory or complete-work accounting.

`CheckedEnumFunction` retains immutable syntax, the enum environment and match
coverage plans, with conservative return coverage. It does not retain all typed
nodes, coercions or call-target facts needed for lowering. This API remains
standard-only. A separate [callable-source API](callable-function-v2.md) now supports
canonical constructor calls, self-recursion and bounded call/context facts. Other
user calls, nested matches
(in ordinary branches or match arms), modules/multiple functions in one source,
package visibility, executable IR/runtime/native/JavaScript and tooling remain
open. This single-function API does not establish whole-program validity or
change ordinary CLI/v1 stage behavior. See the
[approved completion contract](../design/G13_COMPLETION_CONTRACT.md).
