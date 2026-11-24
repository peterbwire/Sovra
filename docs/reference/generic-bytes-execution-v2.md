# Checked immutable Bytes execution

The explicit Rust library profile `compile_generic_data_package` /
`execute_generic_data_package` now executes immutable `std::Bytes` values.
Default and collection-only compilation/execution do not gain data operations.
Existing native-image encoders refuse this profile; no CLI data flag is enabled.

| Source operation | Result | Meaning |
| --- | --- | --- |
| `std::bytes_from_text(String)` | `std::Result<std::Bytes,std::Error>` | Copy the original UTF-8 encoding, without normalization or a terminator |
| `std::bytes_from_array([std::Byte])` | `std::Result<std::Bytes,std::Error>` | Copy checked bytes in array order |
| `std::bytes_to_array(std::Bytes)` | `std::Result<[std::Byte],std::Error>` | Copy raw bytes into an independent mutable Byte array |
| `std::bytes_to_hex(std::Bytes)` | `std::Result<String,std::Error>` | Encode lowercase hexadecimal, two digits per byte |
| `std::bytes_from_hex(String)` | `std::Result<std::Bytes,std::Error>` | Decode strict ASCII hexadecimal, accepting either letter case |
| `std::bytes_to_base64(std::Bytes)` | `std::Result<String,std::Error>` | Encode padded standard RFC 4648 Base64 |
| `std::bytes_from_base64(String)` | `std::Result<std::Bytes,std::Error>` | Strictly decode the standard alphabet with canonical padding and discarded bits |
| `std::bytes_to_utf8(std::Bytes)` | `std::Result<String,std::Error>` | Strict UTF-8 decoding with original invalid-byte offsets and no replacement |
| `std::bytes_len(std::Bytes)` | `Int` | Number of bytes, including any embedded zero bytes |
| `std::bytes_get(std::Bytes, Int)` | `std::Result<std::Byte,std::Error>` | Copy the Byte at a zero-based byte offset; negative and out-of-range indices return structured bounds errors |
| `std::bytes_concat(std::Bytes, std::Bytes)` | `std::Result<std::Bytes,std::Error>` | Copy both buffers in left-to-right order; capacity failures return a structured error |
| `std::bytes_slice(std::Bytes, Int, Int)` | `std::Result<std::Bytes,std::Error>` | Copy the half-open byte range `[start,end)`; invalid bounds return a structured error |

```svr
fn main() -> std::Result<Int, std::Error> {
    let bytes = std::bytes_from_text("λ")?;
    return std::Result::<Int, std::Error>::Ok(std::bytes_len(bytes));
}
```

Buffers have distinct checked identities from String and `[std::Byte]`. Copies,
arrays, nominal payloads and imported generic forwarding retain that identity.
Equality compares bytes in order. Display uses `std::Bytes [206, 187]` for the
example above; display is not a codec or serialization format. Buffers are not
Key values. Byte keys continue to work in the explicit data VM.

The flat VM stores each buffer as one root plus one Byte node per byte. This is
not the packed `RuntimeBytesV2` helper representation. The flat representation
preserves existing subtree copy, selection and cumulative resource accounting.
An N-byte buffer payload costs N+1 logical nodes and zero owned text bytes.
Constructor success wraps it in Result: N+2 nodes and 13 label text bytes in
whole. Original String evaluation still costs its UTF-8 text bytes. Array conversion creates an
independent snapshot, including for empty buffers. Snapshot and equality APIs
retain the original checked layout proof; `GenericCollectionSelector::Item`
selects a buffer Byte. Borrowed inspection uses `bytes_len()` and `byte_item()`;
`array_len()` continues to distinguish arrays.

Whole output node/text admission happens before allocation. Construction has a
16 MiB default byte cap, shared with `RuntimeBytesLimits::DEFAULT`. The explicit
VM policy `max_byte_buffer_bytes` can lower it, including to zero; values above
the default ceiling refuse policy admission. The same limit governs text/array
constructors and concatenation. UTF-8 length counts bytes, not characters.
VM materialization, work, instruction and cooperative cancellation/elapsed limits
also apply.
Failure returns no partial success value. Buffers can appear in otherwise
ordinary checked nominal/array/Option graphs in this explicit profile.

Safe access returns `Ok(Byte)` with two owned nodes and 13 label text bytes.
Bounds failures return `Err(std::Error)` with code `bounds`, message
`byte index out of bounds`, and offset `Some(attempted_index)`, including negative
indices. This owns six nodes and 86 text bytes, including Result/Error/Option
labels. The complete result is preflighted before payload/wrapper allocation.
Concatenation success preflights its entire N+2-node Result/buffer output and
13 Result label text bytes, preserves both inputs and shares the 16 MiB buffer
cap. Constructor and concatenation capacity failures return code `capacity`,
message `byte buffer capacity exceeded` and absent offset. This owned Err costs
five nodes and 93 text bytes, including every label and field String. Capacity
is checked before output-buffer allocation. VM resource exhaustion remains a VM
refusal. Semantic checking, concrete rechecking and IR admission validate exact
sealed Byte/Bytes/Result/Error identities separately.
Input evaluation and snapshots also consume cumulative limits.

Slicing copies raw bytes in order, preserving the input. The range is half-open:
`start == end` is a valid empty slice when the index is within `0..=length`.
UTF-8 character boundaries do not restrict byte-buffer ranges; a slice may contain
an incomplete encoding and remains Bytes. It never fabricates a String.

Bounds validation reports a negative start first, then a negative end, then a
reversed range at the start index, then an end beyond the buffer. The error has
code `bounds`, message `byte slice out of bounds`, and offset `Some(index)` with
the original signed index. It owns six nodes and 86 text bytes. Success owns
N+2 nodes and 13 Result label bytes for N copied bytes. Both complete results
are preflighted before slice/payload allocation; shared work and cooperative
controls also apply throughout copying. Generic forwarding and checked buffer
identity remain intact.

Conversion back to an array preserves Byte order and exact sealed element
identity, without interpreting UTF-8 or coercing Byte to Int. The array is an
independent snapshot: changing its elements leaves the buffer unchanged. An
N-byte conversion result owns N+2 nodes and 13 Result label text bytes, including
for empty arrays. The complete result is preflighted before copying or relabeling
the owned snapshot. Source checking, concrete rechecking and typed IR admission
independently require Bytes input and a sealed Result<[Byte],Error> output.
Imported generic forwarding retains the original array/element ownership.

Hex output is lowercase ASCII without a prefix, terminator or whitespace. Empty
input encodes/decodes successfully. Hex decoding accepts uppercase and lowercase
digits; it rejects nonhexadecimal bytes, whitespace, prefixes and unmatched final
digits. It does not normalize text or replace invalid input. Error offsets refer
to the original UTF-8 input bytes, rather than Unicode character positions.

The VM shares its hex-digit validator with `RuntimeBytesV2`. Following that
helper's order, decoding checks the 64 MiB codec-text ceiling and decoded buffer
capacity before scanning for the first invalid byte, then checks odd length.
Format errors have code `hex`, message `invalid hexadecimal input` and offset
`Some(original_byte_index)`. This complete Err owns six nodes and 84 text bytes.
Codec capacity errors use code `capacity`, message `codec capacity exceeded` and
absent offset, owning five nodes and 87 text bytes.

Hex encoding success owns two nodes and `13 + 2*N` text bytes for N input bytes;
decoding success owns N+2 nodes and 13 label text bytes for N decoded bytes.
The complete Result and payload costs are preflighted before output allocation.
Validation/copying shares the VM work, instruction and cooperative controls.
Input evaluation and propagation snapshots remain additional cumulative costs.

Base64 encoding uses the RFC 4648 standard alphabet (`+` and `/`) and required
`=` padding, without line breaks or whitespace. Empty buffers encode to empty
text. Decoding rejects URL-safe characters, whitespace, incomplete quartets,
misplaced/excess padding and nonzero discarded low bits. It does not normalize
or repair input. The VM and packed Rust helper use the same strict validator,
including original input byte offsets and format-before-decoded-capacity order.
A truncated quartet reports the original input length as its unexpected-end
offset. Control/work failures retain the caller's own error type.

Base64 format errors use code `base64`, message `invalid base64 input` and
`Some(original_byte_offset)`, owning six nodes and 82 text bytes. Codec capacity
errors retain the five-node/87-text-byte error described above. Encoding success
owns two nodes and `13 + 4*ceil(N/3)` text bytes for N input bytes. Decoding
success owns N+2 nodes and 13 Result label bytes for N decoded bytes. Complete
output ownership is admitted before allocation; validation and copying check
shared work and cooperative controls. Both operations retain strict sealed
String/Bytes Result types and explicit data-profile admission.

UTF-8 decoding follows RFC 3629 through standard-library validation of stack-only
segments of at most four bytes. It rejects stray continuation bytes, overlong
forms, surrogate encodings, code points above U+10FFFF, bad continuations and
truncated sequences. Invalid input never becomes a String. The error reports
the start of the invalid sequence, matching `std::str::from_utf8().valid_up_to()`,
with code `utf8`, message `invalid UTF-8 input` and `Some(original_byte_offset)`.
This complete Err owns six nodes and 79 text bytes.

Valid input preserves every byte, including BOM, embedded NUL, combining
characters and an explicitly encoded U+FFFD. There is no normalization, BOM
stripping, terminator insertion or replacement. Decoding success owns two nodes
and N+13 text bytes for N input bytes. The VM validates the complete input before
output allocation, preflights the whole Result, then copies valid segments into
an exactly sized String. Both passes check cooperative controls. No temporary
owned byte buffer or unchecked UTF-8 conversion is used.

All listed byte codecs now execute in the explicit library profile. CLI/native
data qualification, JSON and remaining G13 phases are still open.
