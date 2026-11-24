# Polymorphic display and array length (V2)

`compile_generic_value_source` extends the checked host-free builtin profile.
Experimental generic commands now select this environment.

- `std::array_len(array)` returns checked Int length for any admitted `[T]`,
  including nested arrays, nominal elements and generic element parameters.
- `std::to_string(value)` returns display text for any checked value, including
  an unconstrained generic parameter after concrete rechecking.

Both calls take one argument and no explicit type arguments. Symbolic, concrete
and IR stages independently validate arguments/results. Array length requires an
actual array type; an unconstrained `T` is insufficient. Display accepts all types
in the current checked value model. Array literals retain their existing explicit
context requirement; pass a typed local rather than relying on new inference.
Neither operation invents callee instances or host access.

Display follows existing primitive/array and versioned nominal conventions:

| Value | Example display |
| --- | --- |
| Unit | empty String |
| Bool, Int, Float | existing Rust-owned runtime spelling |
| String | original text without quotes or escaping |
| Array | `[1, 2]` |
| Record | `Pair { left: 1, right: two }` |
| Enum | `std::Option::Some(7)` or `std::Option::None()` |
| Map/Set (explicit collection profile) | `std::Map { 1: one }`, `std::Set { true }` |

Records use the checked layout's declaration order, independent of initializer
evaluation order. Nominal type arguments are not included in displayed names.
This is display text, not serialization. Use explicit terminal escaping where
needed; formatting itself does not publish output.

Formatting traverses flat owned values iteratively. A first pass computes exact
output bytes with checked arithmetic and shared work admission; scalar numeric
formatting uses a fixed conservative work charge. Before allocating output or
emission tasks, the VM admits one output node, exact output bytes and a conservative
`4 * input nodes + 1` temporary task-node allowance. The second pass emits under
shared work/control checkpoints and verifies the preflight size. Temporary
allowances are cumulative and not refunded. Deep runtime values do not consume
recursive formatting stack frames. Array length uses checked Int conversion.

Native feature word **255** retains this environment at bounded startup checking.
Words 1/3/7/15/31/63/127 retain their previous environments; unknown/partial words
and consumer downgrades refuse. The updated [consumer](../examples/generic-result-adapters.svr)
executes array length and display in a real locked Linux native build without
project inputs or tools on PATH. Earlier source APIs remain strict for these calls.

Append, concat and slice bridging, host operations, remaining E2/G13 requirements,
installed-payload execution and other-host qualification remain open. Task Manager
is unchanged.

Array operation update: [growth and slicing](generic-array-builtins-v2.md)
now extend experimental commands and native feature 511. Earlier APIs/image
environments retain their previous scope; host I/O remains gated.

[Intrinsic collection display](generic-collection-display-v2.md) is now available
through the explicit collection package profile, with unchanged ordinary
compiler/native feature gates and display costs.
