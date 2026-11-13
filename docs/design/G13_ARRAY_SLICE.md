# G13: snapshot array slicing

Status: **User directed implementation after review; implemented.**
See the [API reference](../reference/array-slice.md). This is the next collection
slice under the expanded [18-task checklist](../18_TASK_COMPLETION_CHECKLIST.md).
Typed arrays, array_len and resource controls are already implemented. The user
separately directed continuation of the reviewed slicing contract.

## Approved API and semantics

Add the compiler-known builtin `std::array_slice(values, start, end)` with contract
`([T], Int, Int) -> [T]`. T describes the input's retained element type, not new
source generic syntax or an Any return. No range expression, method syntax,
mutating resize, append, borrowing or shared mutable storage is introduced.

Use half-open element bounds: copy values[start..end]. Require
`0 <= start <= end <= array_len(values)`. start == end returns a typed empty array;
0..length copies the full array; length..length is valid. Negative, reversed and
out-of-range bounds fail without clamping, Python-style offsets or partial results.
Indices count elements, not UTF-8 bytes. Int-to-host-size conversion is checked;
JS compares BigInt bounds before converting admitted indices to Number.

The result is an independent snapshot, including nested arrays and record values.
Changing a returned element does not change the input. Immutable physical string
sharing is permitted only under the existing logical snapshot semantics.
All arguments retain existing left-to-right evaluation and normal copy costs.
Input argument evaluation can already copy the complete source array; this API
makes no zero-copy or asymptotic source-evaluation promise.

## Typing and compiler integration

Preserve the canonical input element type, including nested arrays, aliases and
nominal package/record identity. Check start/end as Int, never Float or coercible
strings. Ordinary arity/type diagnostics keep their established codes/spans;
unsupported argument types must fail semantic checking before executable output.
Unresolved input/error types must not become fabricated checked result facts.

Typed empty inputs such as `let values:[Int]=[]` work. Preserve the existing empty
literal inference rules; do not add backward generic inference or erase an element
type merely to accept this builtin. Existing Int-to-Float array assignment/return
widening still applies to the returned value, with existing conversion accounting.

Update the canonical stdlib registry, semantic/checked result facts, legacy and
checked lowering, package interfaces/registered compilation and both runtimes.
The return type depends on the checked first argument, so a registry placeholder
must not leak into public types or IDE evidence. Preserve public stage APIs and
ordinary unknown/error diagnostic behavior. No independent generic type system
is selected by this operation.

## Runtime errors and ordering

Existing call arity validation remains first within the operation. Runtime type
validation for malformed IR uses
`std::array_slice expects Array, Int, Int arguments`. Invalid bounds use
`array slice bounds out of range`. Validate types and bounds before result fuel
admission or copying. Existing dispatch control/instruction admission precedes
operation handling. The operation performs no host I/O.

After validation, premeasure and admit the entire result atomically, then construct
it using iterative child snapshots. A failure publishes no prefix/result and
uses existing iterative Rust ownership guards for partial construction/cleanup.
Allocation failure is not converted into a new recoverable language guarantee.
Cooperative control remains at approved instruction boundaries; slicing helpers
are not newly preemptible midway and no fixed latency is promised.

## Model-v1 logical materialization charge

Charge one new array node plus the complete node/owned-text footprint of every
selected child. Record type/field labels and nested String payloads count in those
children. Empty slices cost one node and zero text. Do not charge copied children
twice, charge unselected descendants, refund dropped input values or reinterpret
scratch/capacity as logical text. Compute aggregate sizes with checked u64 sums
before allocation. Node/text admission remains atomic with existing error priority.

This extends the table with a specified new event; it does not change earlier
model-v1 charges. The new operation is unavailable in earlier compilers, so locks,
public support claims and examples must reflect the compiler feature requirement.

For this complete main:

```svr
fn main() {
    let original = [1, 2, 3];
    let selected = std::array_slice(original, 1, 3);
    print(selected);
}
```

Verified logical cost is 18 nodes and 6 text bytes, including normal input/print
snapshots, print Unit and root implicit Unit. Printed output `[2, 3]` independently
costs 7 output bytes including LF. Actual checked-IR execution tests confirm these exact costs. A 17-node allowance would refuse the
root Unit after output; a 5-text allowance would refuse the print before output.

## Consumer and qualification gates

Create an independent registered library exporting, for example,
`tail(values:[Int]) -> [Int]`, and a separate consumer. Define tail's empty-input
behavior in library code; do not make builtin slice silently repair invalid bounds.
Retain reusable behavior rather than an app-specific compiler primitive.

- Verify primitive/nested/record/alias arrays, typed empty/full/end-empty slices,
  rejected nonarrays/non-Int/arity, private nominal interfaces and type mismatch.
- Prove snapshot independence after mutating an output element, including nested
  values supported by existing syntax. Preserve original source diagnostics.
- Test all bounds failures, i64 extremes and zero-length inputs in Rust, ordinary
  Node and controlled Node workers. Invalid bounds precede new materialization
  exhaustion within the operation; dispatch controls retain their earlier priority.
- Test exact/one-under node/text limits, Unicode/record-label costs, no double
  charge, deep/wide snapshots and no fallback. Update the complete fuel table.
- Compile/check/run/test the independent locked package consumer, inspect retained
  types/coercions and verify formatter/IDE compatibility. Run actual Linux native
  output, retain full regressions and qualify other hosts separately.

Map/Set, general generics, sum types/Option/Result, binary/JSON, filesystem/time/
process APIs and their authority/lifetime contracts remain separate decisions.

## Approval boundary

[AGENTS.md](../../AGENTS.md) says: "Document and pause before fundamental syntax,
memory-model, type-semantics or compatibility decisions." The approved
[collection foundation](G13_COLLECTIONS_AND_DATA.md) also explicitly requires
review of the next collection/error/binary/JSON decision before implementation.
The user directed implementation after reviewing this half-open refusal policy,
dependent array result type, snapshot behavior and complete logical fuel charge.
Do not ask again for this operation; future collection/type/resource designs
remain separate decisions.
