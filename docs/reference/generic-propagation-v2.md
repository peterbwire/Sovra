# Checked propagation compatibility

`plan_generic_propagations` is an additive Rust API for the symbolic type contract
of future postfix propagation. It accepts a sealed `CheckedGenericCopyBodies`
table, ordered function/operand identity requests and explicit
`GenericPropagationLimits`. It returns a complete immutable batch or a refusal.
It does not parse `?`, modify source, insert returns or execute instructions.
Source `?` remains refused in the source compiler and every native image profile.

Each request selects an existing immutable operand expression from its enclosing
checked body. Function-table and body-arena identities must match exactly;
matching source text or ordinal numbers cannot substitute for ownership. Repeated
source operands refuse. The operand must resolve to a compiler-owned
[standard Option or Result](generic-standard-v2.md), and the enclosing function
must explicitly return the same canonical standard enum. User types with matching
names/shapes, primitive/array/record operands, wrapper-family changes and Unit
return functions refuse. Transparent aliases preserve canonical identity.

The successful payload can differ from the enclosing return wrapper's successful
payload. For example, Option<Int> inside a function returning Option<String> can
plan extraction of Int and early return of None. Result<T,E> inside a function
returning Result<U,E> can plan extraction of T and return of Err(E), even when T
and U differ. Error payloads must compare exactly after alias expansion; Int/Float
widening, unrelated symbolic parameters and nominal substitutions are not implicit
error conversions. [std::Error](generic-error-v2.md) is an ordinary nominal error
payload for this contract.

A sealed plan retains the original symbolic body, exact operand identity,
compiler-owned wrapper/declaration identity, original success/failure variant
indices, projected successful type, optional failure payload type and the caller's
full declared return wrapper. Option failure is nullary; Result failure retains
its exact error context. Type refusals identify the enclosing signature and
original function-fragment operand span. Global quota/ownership refusals do not
invent a source location. The plan preserves source/provenance ownership after
the original host containers are dropped.

`GenericPropagationLimits::DEFAULT` admits at most 4096 selected sites, 12288
retained type projections, 65536 exact comparison pairs, 131072 comparison edges
and 1000000 logical work units. Every request shares remaining allowances;
comparison budgets do not reset per site. Option retains two projected contexts
and Result three. Projections share existing immutable arenas instead of copying
type graphs. Variant lookup, context checks and error comparisons consume work.
An empty batch may succeed with zero allowance; exhaustion of required work
returns no partial plan or unlimited/legacy fallback. These are logical metadata
limits, not physical heap or CPU-time guarantees.

Future source/checking/lowering integration must evaluate an operand once, snapshot
its successful payload under existing VM policy, or create the caller's compatible
failure wrapper and return immediately. Result failure must snapshot its error
payload; no exception, panic unwrap or automatic error conversion is authorized.
Concrete substitutions, runtime layouts/control flow, temporary operand cleanup,
coercion contexts, return-flow obligations and native feature gating still require
integration. This API alone proves none of that executable behavior and adds no
native feature or CLI mode.

`tests/generic_propagation_v2.rs` checks differing success payloads, generic error
identity, aliases and structured errors, invalid wrapper families, foreign tables
and same-signature foreign arenas, duplicate sites, exact cumulative allowance
boundaries, original source retention, error provenance and continued source/image
refusal of `?`. Explicit adapters, executable postfix propagation and remaining
G13 work stay open. G13 E2 and G13 are incomplete.

A separate [syntax-only block adapter](generic-propagation-syntax-v2.md) now
retains original postfix ranges. It does not connect these plans to executable
source checking or change the continued refusal of source/image question marks.

[Opt-in complete symbolic bodies](generic-propagation-bodies-v2.md) now share the
compatibility algorithm while recording body-owned postfix expression facts.
Concrete/executable integration remains blocked.

[Explicit specialization/concrete rechecking](generic-propagation-concrete-v2.md)
now recomputes these contracts per instance. IR emission and runtime propagation
remain gated.
