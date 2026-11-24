# Explicit checked Byte execution

The Rust library APIs `compile_generic_data_package` and
`execute_generic_data_package` admit scalar Byte execution in captured local
packages. This is an explicit experimental profile, including for data-free
programs. Default and collection-only execution refuse its IR; both existing
native-image encoders refuse it. No CLI flag or native data format is enabled.

Source operations are:

```svr
fn main() -> std::Result<Int, std::Error> {
    let byte = std::byte_from_int(255)?;
    return std::Result::<Int, std::Error>::Ok(std::byte_to_int(byte));
}
```

`std::byte_from_int(Int)` returns `std::Result<std::Byte,std::Error>`.
All values from 0 through 255 succeed. Other integers return an owned error with
code `byte_range`, message `byte value must be in 0..255`, and absent offset.
`std::byte_to_int(std::Byte)` is the explicit conversion back to Int. No implicit
conversion or integer-backed Byte forging is admitted. Generic forwarding,
arrays, snapshots, equality, display and map/set key comparisons retain Byte
identity. Borrowed results expose `as_byte()` separately from `as_int()`.

Before constructing either conversion result, execution preflights its complete
owned node/text cost, including labels. Success owns two nodes and 13 text bytes;
range failure owns five nodes and 94 text bytes. Input evaluation and later
snapshots consume additional cumulative budget. Shared work, instruction and
cooperative execution controls still apply. Byte display uses decimal digits;
it is not a binary serialization format.

The semantic checker, concrete recheck and IR admission validate operands and
sealed Result/Byte/Error schemas independently. The same explicit profile now
admits [immutable buffer construction and length](generic-bytes-execution-v2.md).
Packed buffer codecs currently remain Rust helper APIs. Further source buffer
operations, CLI/native data admission, JSON and remaining G13 phases remain open. The separate Rust collection-storage descriptor API still refuses
Byte domains; Byte keys here execute through the checked flat VM.

Focused source tests cover all 256 Byte values, owned range errors, explicit
conversion and propagation, generic key forwarding, arrays, map/set behavior,
display, exact materialization boundaries and strict older-profile refusals.
