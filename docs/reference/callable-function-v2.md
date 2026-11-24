# Constructor and self-call checking

Status: **Partial E1 semantic/evidence foundation.**
`CheckedEnumTypes::check_callable_function_source` opts one source function into
canonical enum constructors and self-recursion, while retaining bounded call and
conversion-context facts:

```svr
fn value() -> Outcome {
    return Outcome::Number(1);
}
```

The environment must already contain `Outcome` and its payload declarations.
Private signature headers describe each constructor and the current function;
they are checker inputs, never an executable v1 Program or runtime encoding.
Constructors require exactly zero or one payload according to the declaration.
Unit payloads remain distinct from nullary variants. Core arity/type diagnostics
(`E3006`/`E3007`) apply, with existing Int-to-Float widening through arrays.
Nominal enums cannot substitute for each other or be built/projected as records.
Qualified constructors use canonical names, for example `lib::Outcome::Number`;
alias-qualified constructors are unavailable. Module/package access proof remains
outside this local environment API.

Self-calls use the function's validated source signature, including argument and
return types. Calls resolve in the function namespace, independent of local value
shadowing, matching the existing core rule. This proves call typing, not termination.
Other user functions, imports and mutually recursive functions remain unavailable.
The earlier `check_function_source` API retains its standard-only call scope.

`CheckedCallableFunction` retains the checked function/environment and immutable
`CallableEvidence`. Each call head records its original span and a canonical
standard target, current-function target, or environment-local enum/variant ID.
Call order follows semantic visitation. Spans belong to this single retained
source syntax; they are not global structural IDs or filesystem identities.
`CallableContext` records the original value span, existing core context category,
and actual/expected types at explicit core conversion sites. Identity contexts
and existing empty-array Unknown holes are retained. Source-local diagnostics
and contexts preserve Unicode/CRLF byte locations.

These facts are **not complete lowering evidence**. This API does not retain
array-literal element contexts, expression types or scoped declaration/value-use
identities. The separate [typed callable API](typed-callable-v2.md) now records
core value-expression types, call/context identity links and array-element contexts.
The separate [scoped callable API](scoped-callable-v2.md) now retains declaration/
read/write/scrutinee identities; versioned IR mapping and execution remain open. Runtime/backend/native/CLI
integration, nested matches, complete programs and packages/tooling remain open.

`CallableLimits` independently bounds the complete constructor/self-signature set,
owned signature text, resolved calls, conversion contexts and cumulative copied
type nodes/nominal text. All environment variants are admitted even if unused.
Signature text counts header names and both function/target lookup-key copies,
return annotations and parameter names/annotations. Type nodes count every scalar,
named leaf and array wrapper per copied actual/expected context; nominal text is
charged per copy. Temporary canonicalization/string-building and core checker
scratch are outside retained accounting. These are logical limits, not physical
heap or complete-work accounting. Zero allowances are real limits.

Evidence exhaustion stops retention while ordinary semantic checking continues.
Semantic/return errors win; otherwise exhaustion refuses the whole result without
partial evidence or a legacy fallback. Signatures colliding with a standard
callable produce `E3110` with the original enum source label/span, preventing a
constructor from silently resolving to a builtin. Existing compiler stages,
public v1 AST/Value/IR/native formats and default command behavior remain unchanged.
See the [approved completion contract](../design/G13_COMPLETION_CONTRACT.md).
