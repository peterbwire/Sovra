# Matches in conditional and loop bodies

Status: **Partial G13 E1 checked-program structured bodies.** Program checking now
retains recursive versioned bodies for if, else/else-if and while. Matches can
appear inside those bodies, and controls can appear inside match arms. Conditions
reuse the existing core expression grammar, including its parenthesis behavior.
The earlier standalone/single-entry parsing APIs retain their original scope.

RecursiveControlSyntax contains an original If/While condition/span header with
empty legacy body vectors, plus separately retained complete versioned bodies.
No body is dropped or projected as a successful executable v1 Statement. Original
header/condition/body coordinates survive checking, mapping and IR emission.

The parser checks global token nesting first and explicitly limits recursive
statement depth. This second guard matters for long unbraced else-if chains,
whose braces do not reflect their statement depth. Excess returns E2007 before
recursive checking/lowering. Match-arm/header quotas remain cumulative within a
function across all branches and loops. Existing scoped evidence/frame and
match-binding-copy allowances remain in force; control forks preserve the existing
per-branch lexical scope semantics rather than introducing new value/fuel rules.

Conditions must be Bool under existing alias/type rules. Every branch/body is
checked, including unreachable code. Branch/loop locals and payloads do not escape;
assignments to inherited mutable bindings keep their declaration identities.
Payload shadowing remains immutable through deeper controls. If/else proves a
return only when both bodies guarantee it. While remains conservative and does
not by itself prove a non-Unit function return, even with a true condition.

Mapping indexes the real recursive bodies behind their original control headers,
visits conditions before source-order branches and preserves preorder match plans.
Lowering emits existing evaluate/branch/scope/jump and match snapshot/tag/release
instructions. Loop conditions re-evaluate before every iteration and the final
false test. Completed iteration scopes/snapshots are released before the backedge;
outer snapshots remain live when a loop is inside an arm. The existing linked
engine executes these streams, sharing its invocation limits/services as before.
There is no new VM opcode, value representation or accounting model.

Tests cover linked returns across then/else-if/else matches, iterative local
rebinding, controls inside arms, snapshot cleanup, invalid conditions/coverage,
immutable/non-escaping bindings, conservative return rejection, exact legacy
output/materialization parity, scoped frame admission, bounded infinite loops,
once-per-test call conditions and excessive else-if depth. The
[independent control-matches fixture](../../examples/control-matches/main.svr)
prints 9 through the checked-program API.

This closes the previously explicit if/while match-body restriction in program
checking. It does not integrate the extended grammar into default CLI/formatter/
LSP, JS/Worker/native or package/module handling, or complete G13. Those and broader
host/platform qualification remain open under the [contract](../design/G13_COMPLETION_CONTRACT.md).
