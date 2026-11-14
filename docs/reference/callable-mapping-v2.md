# Checked callable structural mapping

Status: **Partial E1 versioned lowering foundation.**
`CheckedScopedCallableFunction::map_for_lowering` creates a bounded structural
map of its immutable checked source and retained semantic facts. It does not
emit executable IR instructions, compile applications or execute enums.

`MappedCallable` borrows the checked owner; it does not clone AST strings, types
or payloads. Its preorder nodes include the function, parameters, ordinary
statements, matches, arms, payload declarations and core expressions. Each node
has a mapping-local identity and an earlier parent, except the function root.
Another mapping refuses its node IDs; clones preserve their identities. Syntax
references remain tied to the borrowed checked owner.

Expressions distinguish evaluated values, resolved call-name heads, direct write
targets and indexed write targets. Match scrutinees remain separate binding-read
nodes. No untyped call head, write or scrutinee is fabricated as a typed value.
Nested ordinary branches/loops, arrays, records, calls and indexing retain their
original structure. Match arms retain checked declaration-variant ordinals even
when source arm order differs.

Mapping validates one-to-one source coverage for retained expression/declaration
facts and read/write/scrutinee uses. Reads attach through checked expression IDs;
write permissions, declaration names/roles and source shapes are validated.
Call/result and actual-value/context links must cover their tables exactly once.
Calls validate their retained canonical standard/self/nominal constructor target;
context actual types must match the retained value type. The mapper consumes
existing facts rather than repeating type inference or lexical scope resolution.

Original spans are checked locators, not structural identities. Source nodes are
indexed by role and original location to attach older facts that do not retain a
structural path. Ambiguous locations within a role, missing facts, foreign IDs,
shape/type disagreements or incomplete coverage refuse the entire map. No
arbitrary match or legacy fallback is returned. Structural IDs are assigned
independently of offsets. A future source form that introduces ambiguous spans
needs an explicit checked structural link before this mapper can accept it.

`MappingLimits` admits every scheduled/retained node and cumulative semantic
attachment before adding it. Pending traversal entries and lookup maps are bounded
by those node/fact counts. Traversal is iterative. Parent edges are included in
node storage; no source/type text is retained by this mapping. Temporary call-name
validation strings and original checked-owner storage are outside these logical
counters. Zero allowances are real limits; they are not physical memory or
complete-work bounds. Failures return `MappingError` without a partial map.

This structural layer now feeds separate [expression IR emission](expression-ir-v2.md)
and validation. Function-level stores/control flow/returns and executable runtime/
backend/native integration,
nested matches, other user functions/complete programs, packages and tooling
remain open. Public v1 AST/IR/Value/native formats and CLI behavior remain unchanged.
See the [approved completion contract](../design/G13_COMPLETION_CONTRACT.md).
