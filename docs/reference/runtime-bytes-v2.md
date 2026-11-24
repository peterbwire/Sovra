# Checked Byte/Bytes runtime foundation (D1)

The explicit Rust `compiler::v2` API now provides `RuntimeByteV2`,
`RuntimeBytesV2`, `RuntimeBytesLimits`, `RuntimeDataWork` and structured
`RuntimeDataError`/`RuntimeDataErrorKind` refusals. This implements a data-runtime
foundation. Source types/builtins, Byte keys, JSON and explicit CLI/native
admission are now integrated separately in the [data profile](data-host-v3.md).
Consolidated G13 qualification remains open.

`RuntimeByteV2` is distinct from Int. `from_int` accepts only 0..255 and returns
an error otherwise; `to_int` is explicit. Storage is private, with no unchecked
constructor or Clone/Copy implementation. A scalar construction, snapshot or
owned Int conversion admits one logical node. Borrowed Rust scalar inspection
is metadata rather than another owned language value.

`RuntimeBytesV2` owns immutable packed bytes. Construction from raw Rust bytes,
valid text or a checked Byte array, snapshots, half-open slices, concatenation,
index access and array conversion perform explicit admission. Invalid indices or
slice boundaries return structured errors, never a default Byte. Owned buffers
and returned arrays are independent copies. Borrowed read-only Rust slices,
length/emptiness and equality results do not materialize language wrappers.

## Strict codecs

- Hex output is lowercase; decoding accepts ASCII upper/lowercase, rejects
  nonhex bytes, whitespace and odd trailing nibbles.
- Base64 uses the RFC 4648 standard alphabet and required padding. Decoding
  rejects whitespace, URL alphabet substitutions, data after padding, malformed
  lengths/padding and nonzero discarded bits. It never silently repairs input.
- UTF-8 decoding uses strict validation and reports the original invalid-byte
  offset. It inserts no replacement characters and preserves BOM, NUL and exact
  Unicode contents without normalization. String-to-Bytes copies valid UTF-8.

Codec errors retain zero-based original input byte offsets. Incomplete base64
reports the end-of-input position; an unmatched hex nibble reports its own index.
Invalid UTF-8 reports `valid_up_to`, including incomplete sequences. Bounds errors
identify the rejected buffer boundary. Capacity/work/materialization/control
errors have no fabricated input offset. The Rust error/result metadata is
not a materialized Sovra std::Result/std::Error value; source integration must
admit its actual wrappers and labels separately.

## Versioned ownership and work

| Owned result | Nodes | Owned text bytes |
| --- | --- | --- |
| Byte or explicit Int conversion | 1 | 0 |
| Bytes/Byte-array with n elements | n + 1 | 0 |
| Encoded/decoded String of t UTF-8 bytes | 1 | t |

Binary bytes do not consume owned UTF-8 text units. Every copied byte contributes
one Byte node even though physical storage is packed. Whole output nodes/text are
preflighted together before output allocation/copy; transferred descendants are
not admitted twice. Earlier work/ownership is not refunded, including on failure.
Malformed codec inputs produce no successful partial output or materialization.

`RuntimeDataWork` is finite and cumulative, with optional cancellation/elapsed
controls. Result preparation charges `2*n + 1` work units, where n is output
buffer/array elements or output String bytes. Hex/base64 validation additionally
charges one unit per input byte examined. UTF-8 validation additionally precharges
its input byte count; equality precharges both buffer lengths plus one. Access
charges one unit. Copy loops and codec emission check controls while progressing;
raw copies checkpoint per 64 KiB chunk. Scalar Byte operations have constant work
and retain selected fuel-control checks. These are cooperative controls, not hard
CPU deadlines or physical process isolation.

Initial caps are 16 MiB binary and 64 MiB codec text. Caller limits must be finite
u32-sized ceilings; zero permits empty results. Checked arithmetic, per-result
limits and platform allocation-index ceilings precede growth. Limits do not
raise the independently selected node/text/work ceilings. Existing VM/CLI policy
and value-model-v1 events are unchanged.

Tests include primary RFC 4648 vectors, standard alphabet/padding and discarded-bit
refusals, overlong/surrogate/out-of-range/incomplete UTF-8, original Unicode byte
offsets, mutation isolation, exact/refused admission/work/control limits and all
256 one-byte plus 65,536 two-byte codec round trips.

## Source integration and qualification

The explicit data profile now supplies checked layouts, IR/VM operations, complete
owned Result/error wrappers, Byte keys and independent CLI/native consumers.
Strict JSON and host adapters are implemented. See [current qualification](../G13_QUALIFICATION.md)
for the remaining consolidated evidence gate.

The independent codec vectors and canonical-padding rules follow
[RFC 4648](https://www.rfc-editor.org/rfc/rfc4648), especially sections 3.5, 4, 8
and 10. Strict UTF-8 validity follows
[RFC 3629 section 4](https://www.rfc-editor.org/rfc/rfc3629#section-4).

The [explicit compiler annotation API](generic-data-types-v2.md) now retains
sealed Byte/Bytes identities and Byte Key proofs. Older CLI/native profiles keep
their strict refusal; only the explicit data profile admits executable operations.
