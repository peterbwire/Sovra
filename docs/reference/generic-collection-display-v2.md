# Collection display (explicit v2 profile)

`std::to_string` now formats intrinsic maps and sets, including collections nested
in records, enums, arrays and other collections, through the explicit collection
package compiler/VM APIs.

| Value | Display |
| --- | --- |
| Empty map | `std::Map { }` |
| Map with two associations | `std::Map { 1: one, 2: two }` |
| Empty set | `std::Set { }` |
| Set with two members | `std::Set { λ, 😀 }` |
| Public entry record | `std::MapEntry { key: 1, value: one }` |

Display preserves insertion order. Replacement retains its position;
remove/reinsert appends. Equal maps/sets can have different display text when
insertion orders differ. Type arguments are omitted, matching existing nominal
display. Public entry records retain ordinary declaration-order record formatting;
the private entry nodes inside maps contribute only `key: value`, never record
names or internal identities.

Existing scalar conventions remain unchanged: strings are raw text without
quotes/escaping, Unit is empty text, and numeric/Bool spelling follows the current
Rust-owned runtime. This is display text, not an unambiguous serialization format.
Use explicit terminal escaping where needed. Formatting itself performs no output
or host I/O.

## Admission and traversal

The formatter retains the existing two-pass iterative flat-value algorithm.
The first pass validates collection layouts, entry owners and key/value child
layouts, checks every set member's key layout and calculates exact UTF-8 output
bytes with checked arithmetic. It visits every node under the shared work/control
policy; collection child validation adds one work unit per child. No recursive
Rust value traversal is introduced.

Before allocating output text or emission tasks, it atomically admits one output
String node, exact output bytes and the existing conservative temporary allowance
of `4*N + 1` task nodes for `N` input value nodes. `N` includes the real private map
entry nodes. This allowance is cumulative and not refunded. Ordinary operand
snapshots keep their previous costs. The second pass emits iteratively with
bounded pending tasks and checks exact preflight/output length agreement. Scheduling
each collection child adds one work unit; output-byte, task, scalar-numeric and
other shared display work charges retain their existing definitions.

For size calculation, a map/set container contributes 12 bytes when empty or
13 bytes plus `2*(length-1)` separator bytes when nonempty. Each private map entry
adds two bytes for `: `; descendant scalar/array/record/enum sizes use the existing
display rules. Materialization charges the resulting output text, not another
owned collection snapshot. All nested values and labels contribute their rendered
bytes. Cancellation, elapsed-time, instruction, work and node/text limits remain
active. Failure publishes no partial String and has no placeholder fallback.

The [source example](../examples/intrinsic-collection-display.svr) exercises
polymorphic map formatting and exact Unicode set text. Tests cover nested values,
public/private entry distinctions, empty and scalar conventions, deep recursive
map/record values, exact output/task budgets and 64 ordered update histories.

## Remaining gates

Default compiler/CLI execution and native image encoding still refuse the
collection profile. CLI/native admission and consolidated backend/host/resource
qualification remain open; C1/G13 are incomplete. Existing noncollection display
and native feature words remain unchanged.

The separate [word-1023 collection image API](generic-collection-native-images-v2.md)
now supports bounded source-backed encoding/rechecking/execution. References above
to native encoding refusals describe the ordinary encoder and installed CLI/native
launch path; those retain their gates. C1/G13 remain open.
