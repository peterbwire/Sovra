# Checked generic function-header type navigation (V2)

Optional standalone generic `--symbols` evidence now links user-defined type
names in function parameter and return annotations to record, enum and alias
source declarations. Nested array and nominal type arguments are included.
Go to Definition selects the original declaration name; hover shows its checked
declaration header. A written alias links to the alias, while binding type hover
continues to display the transparently expanded type.

Projection visits every original checked function header once, including unused
functions. It resolves retained annotation syntax under the sealed signature
registry and parameter scope, with remaining optional work and finite node/edge
limits. Bounds are already established by signature checking, including alias
expansion. Registry-owned IDs map through source type order, independently of
function identities. No editor text search supplies bindings. Original name
ranges are checked against source bytes before publication.

Type parameters shadowing root types are not root-declaration references.
Primitives and compiler-injected types have no user-source destination and are
omitted. This slice does not link type parameters, record fields, enum payload
annotations, alias targets, local annotations, body type arguments or constructor
heads. Those remain separate work. It does not enable formatting or package
overlays.

Calls and header type references share `GenericSymbolLimits.max_references`
(hard ceiling 8192), existing JSON-byte limits and cumulative projection work.
Any exhaustion or provenance refusal discards all optional facts while preserving
compilation success. Invalid edits and unavailable snapshots retire navigation.
Unicode/CRLF offsets use the existing byte-to-UTF-16 mapping.

Tests cover nested annotations, unused generic headers, written aliases,
type/function name collisions, type-parameter shadowing, injected-type omission,
shared reference exhaustion, compiler JSON, unsaved LSP buffers and VS Code.
Runtime and native feature contracts are unchanged. E2/G13 remain open; Task
Manager is unchanged.
