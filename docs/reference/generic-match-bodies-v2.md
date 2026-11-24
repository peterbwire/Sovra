# Symbolic generic match bodies

`check_generic_copy_bodies` now checks exhaustive enum matches in original generic
function bodies. This is a symbolic type-checking API; it does not enable generic
execution through Program, CLI, native images or any backend.

```svr
enum Choice<T> { End, One(T) }
fn choose<U>(value: Choice<U>, fallback: U) -> U {
    match value {
        Choice::End() => { return fallback; },
        Choice::One(item) => { return item; },
    }
}
```

The scrutinee must be an identifier bound in the active lexical scope with a
checked nominal enum type. One checked read represents its evaluation. Every
variant must occur exactly once, with its exact zero/one payload shape. Unit
payloads still require a binding. Root enum names are required; aliases, qualified
heads, wildcards, guards, tuples, nested patterns and expression scrutinees are
outside this profile.

Each arm begins with a copy of the parent lexical scope. Payloads introduce an
immutable `MatchPayload` binding with the scrutinee expression ID and nominal
variant index. Exact substituted types preserve caller parameter and annotation/
alias provenance. Ordinary local shadowing is allowed. Payload bindings and local
shadowing do not escape an arm or become visible to siblings. Mutable parent
locals retain their fixed types; payload replacement and indexed writes through
payload bindings are refused. These are typing facts; runtime snapshots have not
been implemented for generic bodies.

All arm bodies are checked, including unused functions and unreachable statements.
They support the existing body profile, including explicit generic calls,
constructors, conditionals, loops and nested matches. A match proves return
coverage only if every arm guarantees a return. A loop still may run zero times.
An incomplete returning match can be followed by another return.

`GenericCopyStatement::Match` retains the single scrutinee ID, nominal declaration,
original range and source-order `GenericCopyMatchArm` facts. Each arm contains its
variant index, optional arena-owned payload binding and a flat statement range
including nested facts. Body IDs remain isolated across separate checks; clones
preserve their arena.

The private parser adapter uses original tokens and a flat match overlay with
exact-range guarded internal statement markers. It does not rewrite source or
publish markers as typed expressions. Contextual `match` remains usable as an
ordinary value/function identifier. Public Program parsing and the standalone
[pattern-header API](generic-enum-patterns-v2.md) keep their earlier scopes.
Temporary arm bodies drain iteratively on success and refusal.

Whole function source, token and structural-depth admission precede body checks.
Every arm's actual AST is admitted once; the private statement marker contributes
one statement and one expression node for the header. Retained statements,
expressions, bindings and copied binding text use the cumulative body allowances.
The shared coverage checker caps each match at 256 arms and 32 KiB of header
names. Payload graphs share `max_field_nodes` and `max_field_edges` with field
reads and constructors, and use the selected per-payload `field_expansion` profile.
Coverage/name/substitution work, scope copies and arm traversal consume body work;
coverage additionally has the finite default pattern profile. Limits apply across
all supplied functions and never authorize partial body proofs.

Tests: `tests/generic_match_bodies_v2.rs`. Remaining work includes finite
specialization, executable generic lowering/runtime/backend integration and
structured error propagation. G13 E2 remains open.
