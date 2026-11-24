# Bounded transparent generic alias expansion

```rust
use sovra::compiler::v2::*;
let echo = parse_generic_alias("type Echo<T>=T;", GenericAliasLimits::DEFAULT)?;
let keys = parse_generic_record("struct Keys<K:std::Key>{key:K}", GenericRecordLimits::DEFAULT)?;
let registry = register_generic_types(
    &[GenericTypeDeclaration::Alias(&echo), GenericTypeDeclaration::Record(&keys)],
    GenericRegistryLimits::DEFAULT,
)?;
let environment = prepare_generic_aliases(
    &registry, &[&echo], GenericAliasEnvironmentLimits::DEFAULT,
)?;
let scope = parse_generic_parameters("<T>", GenericParameterLimits::DEFAULT)?;
let input = parse_generic_type("Keys<Echo<Int>>", GenericTypeLimits::DEFAULT)?;
let expanded = environment.expand(&input, &scope, GenericAliasExpansionLimits::DEFAULT)?;
```

Preparation requires exactly one target for every alias header in the registry.
Alias target ownership must match that header's original parameter IDs; another
parse with the same name is not interchangeable. Missing, duplicate, foreign,
unknown and wrong-arity targets refuse. Parameter/source/type metadata is shared
through immutable owners rather than copied wholesale.

An iterative dependency scan rejects direct, indirect, nested-argument and growing
cycles before expansion. Nominal record/enum definitions are not expanded through
their fields/payloads. Each alias target then undergoes symbolic expansion and Key
checking under its own parameter constraints. For example,
`type Bad<T>=Keys<T>;` refuses because T does not prove Key, even if a later
concrete use would supply Int. `type Good<K:std::Key>=Keys<K>;` can pass.
Preparation publishes no partially validated environment.

## Expansion and identity

Arguments are expanded first and substituted by declaration-local parameter ID.
No textual source rewriting or matching by parameter spelling is used. Expanded
arguments can therefore prove Key through transparent aliases. Existing
Int/Bool/String and explicitly Key-constrained symbolic parameters satisfy the
initial bound; Float, Unit, arrays and nominal compound types refuse.

`ExpandedGenericType` retains the input syntax, input parameter owner and prepared
alias environment. Its immutable postorder arena contains primitive/parameter/
array/non-alias declared heads. Alias applications disappear; nominal header IDs
remain registry-owned. An explicit `root()` selects the result: it need not be the
last generated node, because unused eager arguments remain admitted and retained.

Substitution reuses immutable type argument roots, producing an acyclic compiler
metadata DAG. This does not introduce runtime references, shared mutable values
or a change to Sovra's owned snapshot value model. Child indices are local to this
result; the separate [equivalence/key API](generic-type-equivalence-v2.md) now
compares roots and constructs exact normalized in-process keys. Node provenance identifies either the original input annotation or
an alias target fragment, with an original span. `alias_source(id)` retrieves a
retained target only for an owned alias identity.

## Finite admission

Environment defaults permit 1,024 aliases, 8,192 referenced-plus-resolved target
nodes, 8,192 referenced-plus-resolved edges, 1 MiB target source and 64 KiB target
name text. Syntax and resolved-head arenas are counted separately. Registry
header/parameter retention keeps its independent limits. Symbolic target checking
admits at most 65,536 cumulative generated nodes, 65,536 generated edges and
1,000,000 logical preparation/checking steps. Dropping temporary validation
results does not refund those cumulative allowances.

Each symbolic target also keeps the independent per-expansion profile: 4,096
generated nodes, 4,096 retained edges, 65,536 input/expansion/checking steps and
depth 128. Input annotation head-resolution limits remain independent. Dependency
walk depth, recursive expansion walk depth and actual substituted structural
depth are checked; their fixed ceiling is 128 even for explicit callers. Thus a
shallow alias cannot hide a too-deep substituted type. Generated unused arguments
still consume admission. No exhaustion has a legacy or unchecked fallback.

Checking steps bound logical visits/lookups/substitution/bound/edge work; they are
not a CPU-time or physical-memory isolation guarantee. Error origins distinguish
input annotations and alias targets. Nested resolution `ParameterScope` failures
refer to the corresponding parameter-list fragment instead. No provenance label
or type identity grants host authority.

## Remaining work

Prepared alias targets now have name/arity/cycle/symbolic-bound validation. This
is not generic function/body checking, constructor/match checking, code
specialization or executable generic support. The separate
[type-definition checker](generic-declarations-v2.md) now checks all root record
fields/enum payloads and requires complete registered definition coverage.
Byte, nongeneric/module/package type contexts, Option/Result and propagation
remain open. Existing source/CLI paths still refuse unsupported generic programs.
The earlier header-only resolver deliberately still refuses alias Key arguments;
use this separate prepared environment when transparent proofs are required.

See [type resolution](generic-type-resolution-v2.md) and the approved
[G13 contract](../design/G13_COMPLETION_CONTRACT.md). G13 remains partial.
