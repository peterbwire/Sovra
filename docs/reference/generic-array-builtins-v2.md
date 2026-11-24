# Checked array growth and slicing (V2)

`compile_generic_array_source` extends the checked value profile with three exact
qualified calls. Experimental generic commands now select this environment.

| Call | Arguments | Result |
| --- | --- | --- |
| `std::array_append` | `[T]`, compatible element | owned `[T]` snapshot |
| `std::array_concat` | `[T]`, compatible array | owned `[T]` snapshot |
| `std::array_slice` | `[T]`, Int start, Int end | owned half-open `[start,end)` snapshot |

The first array determines the result type. Generic and nominal elements retain
their exact identities. Existing contextual array-literal typing remains active;
an unknown first element type is not inferred from the second argument. Calls take
no explicit type arguments, and all symbolic bodies are checked, including unused
functions. Appending/concatenating permits existing Int-to-Float widening through
structural array layers. Float-to-Int and nominal generic-argument conversions
refuse. Slicing requires Int bounds and preserves the first array's exact type.

IR stores an explicit right conversion layout for append/concat. Symbolic checking,
concrete rechecking and typed IR validation independently check element compatibility,
conversion targets and the exact first-array result. The VM copies and widens the
right operand only when layouts differ, using a bounded layout-pair plan and flat
snapshot traversal. It does not reinterpret layouts without converting values.

Every operation returns owned snapshots. Mutating source arrays afterward does
not change appended, concatenated or sliced results. Append/concat reuse the
existing maximum array growth length. Slices allow empty ranges at zero and the
array end; negative, reversed or out-of-range bounds fail at the original call.

The VM scans selected child footprints under shared work/control policy, preflights
all copied nodes/text plus temporary child slots before allocating result storage,
and then copies through the existing snapshot path. Recursive widening has its own
shared snapshot/work/temporary-plan admission. Temporary allowances and discarded
intermediates are not refunded. Exhaustion returns no partial success or fallback.

Native feature word **511** repeats the complete bounded profile at startup.
Words 1/3/7/15/31/63/127/255 retain previous environments; unknown/partial words
and consumer downgrades refuse. Earlier source APIs remain strict for these calls.
The updated [consumer](../examples/generic-result-adapters.svr) exercises all three
operations and numeric widening in a real locked Linux native build without project
inputs or tools on PATH.

Host operations, remaining E2/G13 requirements, installed-payload execution and
other-host qualification remain open. Task Manager is unchanged.
