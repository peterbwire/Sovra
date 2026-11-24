# Checked symbolic postfix bodies

`parse_generic_propagation_function` and `check_generic_propagation_bodies` are
opt-in Rust APIs connecting [original postfix syntax](generic-propagation-syntax-v2.md)
to complete symbolic body checking and [exact propagation compatibility](generic-propagation-v2.md).
They do not enable executable source/CLI/native question marks.

The function adapter admits either a generic or nongeneric header with an opaque
postfix-capable body. Parameter/return/type-parameter annotations remain strict;
header parsing never establishes body validity. Check those signatures against a
complete type environment, then pass the signature table to the new body checker
with ordinary `GenericCopyBodyLimits` and cumulative `GenericPropagationLimits`.
The normal `parse_generic_function`, `parse_specialization_root` and
`check_generic_copy_bodies` paths retain their prior strict grammar.

For example, the opt-in checker can type this body:

```svr
fn test(x: std::Option<std::Option<Int>>) -> std::Option<Int> {
    let value = (x?)?;
    return std::Option::<Int>::Some(value);
}
```

Each question mark binds to its actual body-owned operand under the active
parameter/local/match scope. The checker emits a distinct postorder
`GenericCopyExpressionKind::Propagate` with the earlier operand identity,
original question span and sealed `GenericPropagationTypes`. The resulting
expression type is the successful payload; the retained failure contract targets
the enclosing function's exact wrapper. Result error types compare exactly,
while successful payload types may differ from the function's success type.
No implicit error conversion is introduced.

Locals, calls, arithmetic, arrays/indexed assignment, record fields, branches,
loops, matches, aliases and grouped/chained expressions use the existing complete
symbolic checker. Contextual conversions still apply to the extracted value.
Initializers resolve before a new shadowing binding takes effect. Unknown or
shadowed operands, primitive/user wrappers, incompatible errors and incomplete
successful return paths refuse. Unused functions and unreachable statements are
checked too. A propagated value is not a supported callable head or assignment
target; markers in type positions cannot be ignored.

Original source is never rewritten or masked. Group wrappers are recognized from
the original token ownership before peeling an inner question site. Base values
retain their operand ranges; grouped extracted values retain their encompassing
expression range. Questions map uniquely to original tokens, and every marker
must bind to a value before a successful body table is returned. Type refusals
retain the enclosing signature, question span and nested operand provenance.
Public legacy AST fields/variants remain unchanged; temporary AST ownership uses
the existing guarded iterative cleanup.

Whole-table source/token/AST/expression/binding/work allowances remain cumulative.
Virtual question nodes count toward AST/expression admission and a 128-node
expression-depth ceiling, including combinations with calls/operators/constructors.
Their error comparisons consume both propagation and ordinary body comparison
budgets, capped by the ordinary independent comparison profile. Shared work is
admitted before compatibility checking; projections retain immutable type arenas.
`propagation_usage()` records the opt-in context/comparison/site costs. These are
logical compiler controls, not a physical heap bound or hard CPU deadline.

A table containing propagation is explicitly blocked from
`discover_generic_specializations` with `PropagationUnavailable`, including when
its root does not reach the question-bearing function. [Explicit concrete rechecking](generic-propagation-concrete-v2.md) is now available
through a separate opt-in path. Ordinary concrete checking and IR lowering retain
explicit unavailable cases rather than silently skipping a new node. An opt-in table with zero propagation sites may use the existing pipeline.
Ordinary full-source/CLI compilation and native feature profiles 1/3/7 still
reject `?`; no new executable feature bit or fallback is selected.

`tests/generic_propagation_bodies_v2.rs` covers parameter/local/call/field/index/
control/match binding, nested groups and chains, aliases, generic error identity,
shadowing/refusals, unused code, exact cumulative resource boundaries, virtual
depth, source-aware diagnostics, strict headers and downstream refusal. Existing
generic integration/unit tests verify ordinary source and runtime behavior.

Propagation operation layouts and early-return instruction/control flow,
payload snapshot/fuel/temporary cleanup, executable source/native integration,
explicit adapters and remaining G13 work stay open. This is complete symbolic
body checking for the opt-in profile, not executable postfix propagation. G13 E2
and G13 remain incomplete.
