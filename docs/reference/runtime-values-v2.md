# Versioned runtime values and enum fuel

Status: **Partial E1 runtime-value foundation.** `RuntimeValueV2` executes value
construction, snapshot/extraction, inspection, formatting and equality operations.
The separate [opt-in Rust VM](runtime-vm-v2.md) now executes function IR;
CLI/JavaScript/native integration remains open.

Values own flat acyclic node arenas with child indexes, rather than recursively
owned compound children. Ordinary drop, snapshots, equality and display traversal
remain iterative at deep nesting. Nodes/labels cannot be mutated externally;
copying requires an explicit fuel-admitted `snapshot`. Constructors transfer
owned children, so they do not double charge descendants. Model marker
`VALUE_MODEL_V2` is 2; the old Value/StructValue and model-v1 marker stay unchanged.

`RuntimeValueFuel` reuses the existing atomic v1 materialization admission:
node refusal precedes text refusal, both dimensions are checked before either
changes, and drops never refund. The current complete **enum/value-layer** table is:

| Event | New logical nodes | Newly owned UTF-8 bytes |
| --- | --- | --- |
| Scalar/Unit | 1 | String payload bytes, otherwise 0 |
| Legacy snapshot import | Full imported subtree | All String/record labels |
| Array from transferred values | 1 | 0 |
| Record from transferred fields | 1 | Type and field labels |
| Enum from transferred optional payload | 1 | Canonical enum type and variant labels |
| Full snapshot | Full subtree | Full owned text |
| Payload/array-item/record-field extraction | Full selected subtree | Its owned text |
| Control tag/scalar/length inspection | 0 | 0 |
| Structural equality returning a Rust control bool | 0 | 0 |
| Materialized display String | 1 | Complete formatted UTF-8 bytes |
| Existing numeric widening | 0 | 0 |

A future VM comparison must materialize its language Bool separately. Display
String materialization is not print: output preflight/commit, LF allowance and a
print Unit result still need runtime integration. Representation/traversal/index
scratch is outside these logical counters; they do not measure physical heap,
complete CPU work, output/instruction policy or interruption.

Nominal enum constructors validate environment identity, variant and exact
nullary/single-payload arity before fuel admission. Complete payload checks cover
primitives, arrays, records and enum identities. Int-to-Float widening is allowed
at scalar/array payload boundaries and happens in place after admission. Record
fields must already match their declared schema; a forged record is not silently
repaired by numeric conversion. Record schemas/labels retain existing local
canonical-name behavior, not a new package visibility/authority proof. Duplicate
field names are refused before snapshot import/record wrapper admission.

`enum_tag` performs control-only selection. `payload_snapshot` requires the exact
nominal enum and variant, then admits the complete selected payload before copying.
Nullary and Unit payloads remain distinct. Enum equality requires the same nominal
identity and variant, then structural payload equality. Arrays preserve order;
record associations ignore field order and scalar numeric equality keeps legacy
rules. Formatting is `Type::Variant(payload)` or `Type::Variant()`; Unit payloads
display empty contents but retain their distinct stored shape. UTF-8 is exact.

`from_legacy` preserves valid legacy value footprints. Independent Rust getters
borrow scalar contents/array length without allocating a language value. Transferred
operands are consumed by Rust ownership even when an operation refuses; existing
charges are not rolled back. Guard/type/shape validation precedes admission; copy
and render admission precede owned payload construction. The optional display-byte
cap is checked before materialization and does not publish anything.

Function-IR execution, arithmetic/bounds enforcement, shared per-invocation
instruction/output/control/host policies, recursion/return/frame cleanup and
Rust/JavaScript/native parity remain open. No existing runtime defaults or public
v1 formats change. See the [approved completion contract](../design/G13_COMPLETION_CONTRACT.md).
