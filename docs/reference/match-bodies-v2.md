# Closed-scope match body checking

Status: **Partial E1 semantic foundation.**
`CheckedEnumTypes::check_match_bodies` parses one match statement and checks its
ordinary arm bodies against an explicitly supplied outer scope and expected
return annotation. It requires exhaustive nominal coverage and a bound scrutinee
of the selected enum type. Aliases resolve in the retained enum environment;
closed module type names must use their canonical qualified spelling.

`MatchOuterBinding` supplies trusted name/type/mutability metadata from an
embedding checker. This API does not infer or validate declarations in surrounding
source. Invalid/duplicate outer names and unresolved annotations are refused.
Each arm gets a fresh copy of that scope. Its payload binding has the variant's
resolved type and is immutable, including indexed assignment to an array payload.
The binding may shadow an outer local; ordinary local shadowing inside the arm
keeps core language semantics. Names introduced in one arm never enter another.

Bodies reuse core checking for local inference, annotation/assignment/return
compatibility, numeric widening, arrays, records and standard calls. Real record
fields are retained in the checked environment and charged to its metadata/text
allowances. Enum type probes are excluded from record-field tables: neither
record literals nor field projections may masquerade as nominal enum operations.
Standard result records retain existing core behavior.

`CheckedMatchBodies` retains immutable syntax and nominal coverage, exposes
per-arm return coverage, and reports `guarantees_return()` only if every arm
conservatively returns. An `if` requires returning then/else branches; a loop
alone does not prove return. A nonreturning arm is valid because a surrounding
function could continue after the match. The containing function's required
return coverage is a later integration step. Invalid return types still fail.

`MatchBodyLimits` bounds parsing, outer binding count, total context text and
cumulative initial bindings copied into all arm scopes. Context text includes the
source label, expected return annotation and each outer name/type. The scope
counter includes every outer binding per arm plus each payload binding, even
when it shadows another local. Zero allowances are real limits. Existing fixed
depth and semantic diagnostic ceilings apply; syntax frontend diagnostic settings
do not replace the core semantic ceiling. These are logical admission bounds,
not physical heap or complete semantic-work accounting. Temporary record tables
and core scope/checker scratch remain outside retained environment accounting.

Failures distinguish source syntax, nominal coverage, body diagnostics and
closed-context limits. `E3108` reports invalid supplied context or scrutinee;
body diagnostics preserve core codes and original match-source spans. Body
`Diagnostics` belong to the single supplied match source and have no inferred
filesystem identity. No partial checked body result is returned.

A separate [source-function checker](enum-function-v2.md) now derives surrounding
locals from one function and checks direct-match return coverage. This closed-scope
API still accepts embedding metadata. Only standard calls are available; user
function call-target integration, nested matches and module/package visibility
proof remain open. The result does not retain lowering
coercion/call evidence and is not executable IR. Existing v1 stages, CLI and
runtime formats remain unchanged. See the
[approved completion contract](../design/G13_COMPLETION_CONTRACT.md).
