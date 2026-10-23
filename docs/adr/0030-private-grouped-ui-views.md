# ADR 0030: Private grouped desktop UI views

Application follow-up (2026-10-23): ordinary Sovra logic now paginates Task
Manager's grouped view in 25-row pages and bounds displayed titles/queries.
Navigation is private application metadata, not a new host node or public UI
navigation API. Existing renderer limits and flat interfaces remain unchanged.

Status: **Private prototype design (2026-10-20)** within the user-authorized
desktop work in ADR 0029. This does not approve a public UI protocol or change
Sovra syntax, types, runtime semantics or compiler targets.

Keep `ui-view STATE` and `SVR-UI-1` unchanged for existing browser/desktop
consumers. Opt-in desktop profiles may request `ui-layout-view STATE`, producing
`SVR-UI-2`. Events remain `ui-event STATE ACTION [VALUE]`. No automatic probing
or fallback calls are made against a program that has not opted in.

The private v2 stream adds `group|KEY|vertical|card` (or `horizontal`, `plain`)
and matching `end|KEY` around existing heading/text/input/button nodes. Keys are
unique lowercase action-style identifiers. Nesting is limited to eight groups,
256 leaf nodes, 256 groups and 64 KiB UTF-8 output. Closing markers must match;
unknown properties, unbalanced groups and duplicate action IDs are errors.
The first node remains a heading. No arbitrary CSS, XAML or native properties
are accepted. The 256-leaf limit preserves v1's primitive-control capacity.

Sovra decides grouping and order. The generic renderer maps vertical groups to
native stacks, horizontal groups to wrapping rows and cards to bordered panels.
It must not recognize Task Manager titles, IDs, commands or row formats. Stable
action controls may move between groups without losing their drafts/focus.
Removed or kind-changed fields retain the existing discard rules.

Task Manager emits an input card and a task-list card with one horizontal group
per task. An unrelated component must use the same renderer. Old v1 rendering,
event behavior, persistence and browser tests must remain passing. Test malformed
streams, bounds, cross-group movement, async worker transport and exact native
packages before calling the slice implemented. Public tree/package contracts,
layout APIs and compatibility policy remain proposals under ADR 0023.
