# Initial symbolic generic copy bodies

```rust
use sovra::compiler::v2::*;
let registry = register_generic_types(&[], GenericRegistryLimits::DEFAULT)?;
let environment = prepare_generic_aliases(&registry, &[], GenericAliasEnvironmentLimits::DEFAULT)?;
let types = check_generic_declarations(&environment, &[], GenericDeclarationLimits::DEFAULT)?;
let syntax = parse_generic_function(
    "fn identity<T>(value:T)->T{let copy=value;return copy;}",
    GenericFunctionLimits::DEFAULT,
)?;
let signatures = check_generic_signatures(&types, &[&syntax], GenericSignatureLimits::DEFAULT)?;
let checked = check_generic_copy_bodies(&signatures, GenericCopyBodyLimits::DEFAULT)?;
```

The additive semantic API checks **every** body in a complete signature table
under an explicit initial profile. Supported statements are `let` bindings,
returns, [mutable locals/direct assignment](generic-mutable-bodies-v2.md) and
[expression statements](generic-expression-statements-v2.md), plus
[conditionals](generic-conditional-bodies-v2.md) and
[while loops](generic-loop-bodies-v2.md);
[explicit local annotations](generic-local-annotations-v2.md)
are now supported too. Expressions are value reads, existing primitive
literals, `==`/`!=`, [concrete numeric arithmetic and ordering](generic-numeric-bodies-v2.md),
[Boolean logical operations](generic-logical-bodies-v2.md), and
[concrete String operations](generic-string-bodies-v2.md). Opaque parameters can be copied, returned and compared
structurally when their owner-scoped types agree. Arrays and nominal values can
participate through existing parameter/local values; this profile does not
construct records implicitly. [Explicit generic record construction](generic-record-construction-v2.md)
now validates complete field initialization.
[Explicit enum construction](generic-enum-construction-v2.md) checks nullary and
single-payload variants without executable tag/layout plans. [Context-typed array literals](generic-array-literals-v2.md)
now support explicit expected array types. [Read-only array indexing](generic-index-bodies-v2.md)
now retains exact element types; [indexed replacement](generic-index-assignment-v2.md)
is supported for direct mutable array locals.
[Explicit root generic calls](generic-body-calls-v2.md) now check original
source arguments in active lexical scopes.
[Parameter-independent record field reads](generic-field-bodies-v2.md) are supported too.

Binding initializers resolve before their new declaration, preserving ordinary
shadowing. Reads retain the exact previous binding identity rather than a later
same-named declaration. Distinct opaque parameters do not become interchangeable
because their names/constraints resemble one another. Concrete scalar Int-to-Float
widening is retained as explicit return/comparison-edge metadata; no numeric
behavior is assumed for an opaque parameter, including a Key-constrained one.
Integer literal ranges and floating literal finiteness are checked.

Every supported statement is checked, including unreachable statements after a
return. Straight-line non-Unit bodies must return; explicit void returns and Unit
fallthrough remain distinct. Unit fallthrough creates no synthetic source return
statement or literal expression.

## Original-source parsing and sealed facts

The existing bounded lexer scans the original function source. The core block
parser receives the original EOF-terminated token suffix at the owned body range;
no synthetic source wrapper, top-level Program or rewritten span is manufactured.
The consumed closing range must match the original body's ownership boundary.
Private temporary ASTs are disposed of iteratively on success and failure.

`CheckedGenericCopyBodies` retains all checked signatures/type contexts. Its
category-specific body proofs have private constructors and expose flat statement,
expression and parameter/local binding facts with original full-source ranges.
IDs are scoped to the original checked signature and immutable body arena;
[separate checks](generic-arena-identities-v2.md) cannot exchange IDs. Foreign body/function/table
identities refuse lookup. Source-order statements and postorder expression
operands preserve the source's value-use identities. Facts/owners are immutable
and shared; literal roles record types and source ranges, not an executable
literal payload encoding.

## Admission

Default cumulative caps across all bodies are 128 functions, 4 MiB scanned original
source, 262,144 tokens including EOFs, 65,536 body AST nodes, 16,384 statements,
65,536 expression facts, 16,384 bindings, 64 KiB copied binding-name text,
65,536 comparison pairs, 131,072 comparison edges and 1,000,000 logical semantic/
comparison steps. Each comparison also retains its independent caps, reduced by
remaining cumulative allowances. Scope/fact name labels share each allocation.

AST node admission occurs after bounded parsing and before semantic fact
retention; lexer source/token limits bound temporary parser construction. AST
counting, expression checking and cleanup use heap worklists. Existing parser
structural limits remain active. Logical semantic work does not meter parsing,
string-helper CPU, cleanup or physical RSS; these are not process isolation
or hard deadlines. Exact and one-under tests cover every cumulative dimension.

## Explicit profile boundary

Qualified, implicit-type and function-value calls, opaque numeric assumptions and unary operators,
aggregate coercions, field mutation, inferred array/record construction and matches are not in
this initial profile. They refuse without a partial checked body, even after an
earlier return or in an otherwise unused function. Syntax errors remain original
parser diagnostics; semantic/profile failures retain full-source ranges.

These proofs are symbolic body evidence for this subset, **not executable IR**,
concrete specialization or a generic CLI feature. Concrete substitution/rechecking,
call plans, full body coverage, constructor/match IR and remaining G13 phases stay
open. Existing source/CLI checking still refuses unsupported generic programs.

See [checked signatures](generic-signatures-v2.md),
[type equivalence](generic-type-equivalence-v2.md) and the approved
[G13 contract](../design/G13_COMPLETION_CONTRACT.md).
