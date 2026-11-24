# Opt-in collection annotation semantics

`check_generic_collection_package_types(table, limits)` is an explicit Rust API
that adds compiler-owned `std::Map<K:std::Key,V>` and `std::Set<K:std::Key>` headers
to the existing guarded package semantic checker. It validates real captured
Sovra annotations, aliases, record fields, enum payloads and function signatures.
It returns checked type/signature evidence, **not** an executable program or a
proof that function bodies are valid.

The existing `check_generic_package_types`, complete package compiler, CLI and
native decoder keep their previous profiles. Intrinsic collection annotations
still refuse there. This does not introduce a CLI flag, native feature word or
source-level collection operations. The ordinary Sovra reference package remains
the currently executable collection library.

## Intrinsic identity and opacity

`standard_generic_collection_types()` creates only the two immutable standard
headers, with fresh parameter owners. `GenericCollectionSyntax` has private
fields; callers cannot supply arbitrary names, parameters or constructor kinds.
Each registered header has `GenericTypeDeclarationKind::Collection` and a sealed
`CheckedGenericDefinition::Collection` after definition ownership checks. These
are distinct from aliases, records and enums: there are no fake fields, variants
or zero-field record constructors.

`CheckedGenericCollection` retains its registry-owned ID, original synthetic
parameter evidence and prepared alias/type environment. Fresh standard headers
cannot replay parameter owners from another registration. Same-spelled user
Map/Set records remain ordinary records with their own declaration IDs.

Exact type identity uses the existing alias-expanded declared-type representation.
Map/Set constructor identities and all arguments remain significant. Key checking
accepts Int, Bool and String, or symbolic parameters already constrained by
`std::Key`. Unbounded parameters, Float, Unit and compound/collection keys refuse.
Aliases normalize before checking bounds. Wrong arity refuses, including bare
Map/Set names and extra arguments.

Collection value arguments can contain nested collections and other admitted
nominal/array types at this semantic stage. All argument names resolve using the
annotation's original source context. Direct dependency imports, private/export
visibility, definition-source aliases and canonical nominal ownership are unchanged;
wrapping a private type in Map cannot make it visible.

## Limits and diagnostics

The opt-in checker injects exactly two headers and three parameter records, with
separate immutable owners and standard/global contexts. They consume the same
cumulative scope/context, registry, name/source, definition metadata and alias/
annotation checking allowances as existing standard types. No per-file reset or
unbounded semantic fallback is added. Default profiles do not allocate or charge
these new headers. Errors retain the original importing annotation/source location.

The subsequent existing body API can check ordinary forwarding of opaque typed
values. Collection record literals and field projections refuse: opacity does not
supply a record/enum constructor or field schema. Concrete rechecking validates
closed collection arguments and key bounds using the same bounded specialization
proofs. The original executable layout planner still refuses with
`intrinsic collection execution is not enabled`. The separate
[opt-in layout graph planner](generic-collection-layout-graphs-v2.md) now publishes
opaque shapes under existing budgets, then ordinary construction/IR refuses
`CollectionOperationsUnavailable`. A separate
[initial operation profile](generic-collection-operations-v2.md) now executes typed
empty creation and length under explicit admission. No partially checked collection value reaches execution or serialization.

## Relationship to storage and layout plans

[Opaque layout plans](generic-collection-layouts-v2.md),
[generic snapshots](generic-collection-values-v2.md) and
[typed map storage](generic-collection-maps-v2.md) are existing explicit Rust APIs.
This semantic registration supplies actual source-owned intrinsic type evidence;
it does not silently turn those storage objects into VM values. The executable
layout table and operation/backend admission remain separate qualification steps.

## Qualification and next work

Tests cover source annotations in aliases/fields/enum payloads/signatures, nested
collections, symbolic key constraints, primitive keys, wrong arities/keys,
dependency visibility, original-file errors, opaque construction/projection refusal,
cumulative header admission, foreign parameter owners and successful concrete
forwarding followed by explicit layout refusal. Default source/native compilation
continues to reject the same new annotations.

Bounded [opaque layout graphs](generic-collection-layout-graphs-v2.md) are now
available through a separate explicit API. Next: add actual collection builtin
IR/VM operations with shared controls and full language result admission, then
qualify versioned native/backend support. C1 and G13 remain incomplete.
