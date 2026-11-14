# Recursive match-arm bodies

Status: **Partial G13 E1 program-only match nesting.** check_program_source now
accepts a match directly inside another match arm, recursively. The ordinary core
statements before/after it remain in order. Both matches require already-bound
scrutinee identifiers, exhaustive unique nominal variants and correct immutable
payload bindings. There are no guards, wildcard/or patterns or match expressions.

RecursiveMatchSyntax retains an earlier MatchStatementSyntax **header** with empty
core arm bodies, plus separate complete versioned bodies aligned to those arms.
It is stored in EnumFunctionStatement::RecursiveMatch, never projected into a v1
Statement or presented as executable header-only data. Existing standalone match,
closed-body, single-function and check_entry_source APIs retain their strict grammar.
The separate [structured control extension](control-match-v2.md) now retains
these same program bodies under if/else-if/else/while.

Program parsing uses original admitted tokens; global structural depth is checked
before recursive parsing. Arm counts/header text are charged cumulatively across
all matches in a function before retaining headers/bodies. Existing per-function
scope-binding admission charges ancestor copies before forking arm scopes. Payload
shadowing removes any ancestor mutability; arm-local declarations do not escape.
Every arm/body is checked, including unreachable statements. Nested exhaustive
returns participate in conservative whole-function return coverage.

Coverage plans and scoped scrutinee/payload identities use lexical preorder across
all levels. Mapping indexes the same original headers/bodies under bounded helper
indexes, retains every real child statement and validates its expression/binding
facts. Function lowering emits nested snapshots, tag switches, payload copies,
scopes, releases and returns. Each match captures its scrutinee once. Outer
snapshots remain live while inner matches execute; changing an outer mutable
binding does not change a captured payload. Selected-arm scopes remain separate.
The [linked Rust runtime](program-execution-v2.md) executes these instructions
without a new interpreter or runtime representation.

Model-v2 costs stay unchanged: full match snapshots/payload copies are admitted
atomically; tag selection creates no hidden Bool; scope exits/releases do not
refund. A one-variant I::N(Int) inside O::Some(I), captured and matched twice then
printing its Int, costs exactly 14 nodes and 19 owned-text/display-materialization
bytes including root Unit completion. Tests verify exact and one-under budgets,
permuted arm order, three levels/siblings, 16-level execution, excessive-depth
refusal, immutable shadowing, snapshot mutation, invalid coverage/returns and
exact cumulative arm/header/scope quotas. The [independent fixture](../../examples/nested-matches/main.svr)
prints 7/0/0 through the checked-program API.

This does not enable extended CLI/formatter/LSP, JS/Worker/native or package/module
support. Condition/loop match bodies are now supported by that extension; broader language/backend/platform
qualification remain open under the [G13 contract](../design/G13_COMPLETION_CONTRACT.md).
