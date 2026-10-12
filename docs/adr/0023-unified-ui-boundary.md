# ADR 0023: Unified UI boundary and renderer contract

Status: **Proposed; approval required before a public UI API or protocol is
implemented.** Private Task Manager prototyping with existing language
features may proceed without changing compiler semantics.

## Context

The Node-hosted Task Manager runs real Sovra business operations but defines
its HTML, labels and actions in JavaScript. Sovra has no browser runtime,
cross-platform widget ABI, callbacks or asynchronous event loop. A reusable UI
framework needs one ownership boundary so web, desktop and mobile adapters do
not each redefine application behavior. See the detailed
[architecture plan](../architecture/SOVRA_UI.md).

## Proposed decision

Application components are Sovra functions that receive explicit state and
produce an immutable, versioned platform-neutral view. Named events return
explicit state/view outcomes. The renderer owns platform widgets and escaping;
the host owns process lifecycle, transport, scheduling and capability grants.
Application domain rules and persistence remain in Sovra. Stable component
keys and action identifiers connect events to views. Accessibility metadata is
part of the view contract, not optional renderer decoration.

The first renderer is a Node-hosted browser renderer using the existing
generated-JavaScript process bridge. A private, bounded view/event protocol
proves the idea in the Task Manager before choosing a public encoding, package
API or browser runtime. Desktop, Android and iOS adapters are future work and
must meet shared conformance tests before claiming support.

Public contract decisions still to settle: tree node and property set; key and
action identity; serialization and schema versioning; event ordering and
replay; state/session lifetime; navigation/history; effect/capability grants;
resource limits; error taxonomy; accessibility mapping; and compatibility
policy across renderer versions. This ADR proposes ownership but does not
silently select those details.

## Alternatives

- Put UI templates and application actions in each platform host. This makes
  adapters easy to start but duplicates application behavior and prevents
  Sovra-owned interfaces.
- Add DOM/window/native APIs to the compiler first. This requires language and
  runtime decisions before the existing process bridge has proven a reusable
  component contract.
- Compile directly to browser JavaScript now. The present backend assumes Node
  facilities, so this is a separate runtime target and compatibility decision.

## Consequences and acceptance

The renderer may vary while the application view/event behavior stays testable
in Sovra's interpreter and generated JavaScript. A process bridge adds startup
cost and limits interactive responsiveness; performance targets and a persistent
session protocol need later measurement. The private prototype is accepted only
when an unrelated component and the Task Manager use the same renderer, a real
browser event changes persisted state through Sovra, invalid input is rejected,
and both execution engines agree. Public API approval follows that evidence.

Any new syntax, type/memory semantics, callback or async model, compatibility
rule, browser target or native ABI requires its own approved design under
`AGENTS.md`. This ADR does not authorize those changes.
