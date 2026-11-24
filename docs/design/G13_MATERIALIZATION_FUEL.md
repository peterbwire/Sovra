# G13: logical materialization fuel

Status: **Implemented opt-in Rust/JavaScript model v1; option A and the complete table approved by the user.**
Read-only [value footprint measurement](../reference/value-footprint.md) is
implemented independently. Existing instruction/output APIs, default execution,
CLI and native-image policies remain unchanged.

## Approved decision

Approved **A: cumulative logical materialization fuel**. An opt-in run receives
independent u64 allowances for newly materialized value nodes and UTF-8 owned text.
Successful creation/copy consumes fuel permanently; dropping or overwriting a value
does not refund it. This bounds logical growth/copy traffic, not actual heap/RSS,
GC, allocator capacity, scratch buffers or elapsed time. It adds no references,
new ownership model or source syntax. Existing unlimited/default behavior remains.

Alternative B is live logical heap accounting with release credits and a separate
logical lifetime contract. Alternative C is to defer enforcement while qualifying
host-specific process memory isolation. Those do not have the same accounting or
failure behavior as A and must not be silently substituted.

## Units for model version 1

One node per newly created scalar, Unit, array or record. Arrays/records include
no extra node for a field label or container slot. One text unit per UTF-8 byte in
new String payloads and record type/field labels. Copying a complete value charges
its complete logical footprint, even if an engine physically shares an immutable
string. Labels are included because they are part of the record representation.

Formatted print text also consumes text fuel, but is not a VM String value and
therefore adds no String node. Its trailing LF belongs to the existing output
allowance, not this text pool. Std::to_string returns a String value and charges
one node plus the formatted UTF-8 length. Captured line retention reuses the same
logical line; host copies are outside the policy.

Logical field/function environment keys, IR metadata, stack slots/capacity,
conversion temporaries, encoder buffers, work lists and host-owned data before
VM admission are excluded. Physical GC/sharing does not change these units.
Counters and aggregate sizes use checked u64 arithmetic; overflow rejects admission.

## Complete operation table

| Event | Implemented charge before VM materialization |
| --- | --- |
| LoadLiteral | One node; String literals also charge their UTF-8 payload. |
| LoadName, LoadField, Index | Complete footprint of the selected snapshot, after lookup/index validation. |
| User-function parameters | Complete footprint per copied argument, left to right, after arity/depth validation; argument evaluation is charged separately. |
| std::array_append / std::array_concat | One new container plus all output-child node/text footprints, after type/container and checked length validation. Converted second arguments are copied; no input-container/double child charge. |
| std::array_slice | One result-array node plus complete selected-child node/text footprints; validate types/bounds and admit the whole result before iterative copying. No second child charge. |
| MakeArray | One container node; existing stack children transfer without being charged twice. |
| MakeStruct | One container node plus UTF-8 type/field labels; already materialized field values transfer. |
| StoreName, StoreIndex, Pop, explicit Return with a value | Ownership transfer/drop only; zero fuel and no release credit. |
| Empty-stack Return or implicit fallthrough | One new Unit node, including a discarded root result. |
| WidenFloat/WidenFloatArray | Zero; logical in-place conversion of an already paid value. Backend-only rebuilt numeric arrays are representation overhead, not logical snapshots. |
| Numeric/comparison/equality binary result | One scalar node after operation/type/overflow/division validation. |
| String concatenation | One String node plus the checked sum of input UTF-8 lengths, admitted before constructing the result. |
| print/println | One result Unit node plus formatted line payload text, admitted before host output; output allowance independently includes LF. |
| std::set_exit_code | One Unit node, admitted after argument validation but before changing exit status. |
| std::arg_count, std::len, std::array_len | One Int node. |
| std::arg | One String node plus selected argument UTF-8 bytes, before VM copying. |
| std::to_string | One String node plus exact display bytes measured without constructing compound display text. |
| std::escape_terminal | One String node plus exact escaped UTF-8 size, measured before construction. Existing control escaping stays unchanged. |
| std::split_once | Four nodes: result record, Bool, two Strings; labels plus the exact before/after payload lengths. Find/validate the split before admission. |
| std::lines_unique | One Bool node; helper set/split work and scratch are excluded. |
| std::parse_int | Three nodes: result record, Bool, Int; record/field labels. Existing decimal grammar/overflow behavior stays unchanged. |
| std::read_line | Three nodes: InputLine record, Bool, String; labels plus admitted text bytes, including the EOF/empty String case. |
| std::read_text | Four nodes: TextRead record, Bool, two Strings; labels plus text/error payloads. |
| std::write_text/std::write_text_if | Three nodes: TextWrite record, Bool, error String; labels plus the returned error payload. |

Array conversion deserves explicit review: Rust currently converts owned arrays
in place while JS rebuilds arrays. Both represent the same logical operation;
charging physical allocations would produce different fuel failures. This model
therefore cannot claim to bound JS conversion scratch or real memory consumption.

## Admission order and effects

Existing instruction admission remains first. Validate operation inputs before
materialization admission. Test both materialization dimensions before decrementing
either; a refused creation/copy publishes no partial value. Node exhaustion takes
precedence over text exhaustion when both are insufficient. Successful earlier
copies/operations stay charged if a later operation fails; cleanup remains iterative.

Printing first checks output admissibility without consuming it, then checks Unit/
line materialization fuel. Commit both allowances only when all checks pass, before
rendering/publishing. A refused line consumes no output or materialization fuel;
the dispatched instruction was still charged. Earlier output is not rolled back.

For host I/O, preflight the known minimum result shape before calling the host,
without yet consuming materialization fuel. After the host returns, calculate and
admit the complete result before building/publishing VM values. Host allocations
are outside this model, and input may have been consumed or a file written before
unknown returned payload size causes exhaustion. No rollback is promised. Host
errors that produce no VM result consume no materialization fuel. Blocking host
calls cannot be interrupted by this policy.

## Concrete expected accounting

An owned Packet with fields `items: ["λ", "😀"]` and `ok: true` has five value
nodes, 19 owned UTF-8 text bytes (labels included), and 38 display bytes. Copying
that complete Packet would consume another five nodes and 19 text units. The
38 display bytes are independent of owned-text bytes and do not include a print LF.

For this standalone source, under the operation table above:

```svr
fn main() {
    let source = ["λ", "😀"];
    let copy = source;
    print(copy);
}
```

Creation costs three nodes/six text units. The two LoadName snapshots each cost
three nodes/six text units. Print costs one Unit node/ten display-text units, and
empty-stack root fallthrough costs one Unit node. Total: **11 nodes, 28 text units**,
with **11 separate output bytes**. A ten-node allowance would fail at root
fallthrough after the already accepted line; prior effects remain. These are
implemented and regression-tested logical costs, not measured resident memory.

## API and implementation gates after approval

Use separate MaterializationLimits and a composed policy API rather than adding
fields to the existing public ExecutionLimits struct. Preserve earlier signatures,
CLI defaults and native image version. Unlimited mode must retain behavior; JS
must use exact BigInt counters and the same event order. Version the accounting
model so future optimization changes cannot silently redefine it.

Test every table row in Rust and Node, exact/exceeded/zero/unlimited dimensions,
Unicode labels/text, shared callee fuel, repeated overwrite without credits,
partial host effects, conversion parity, checked arithmetic and deep iterative
cleanup. Unknown operations must not fall back to unmetered materialization.
Live heap/RSS, helper work, interruption, authority, default profiles and bounded
native serialization remain separate designs.

## Approval boundary

[AGENTS.md](../../AGENTS.md) says to "review a concrete accounting contract before
implementing those broader policies" and "Document and pause before fundamental
syntax, memory-model, type-semantics or compatibility decisions."
The user approved A and this cost/effect table before enforcement implementation.
Continue that implementation without requesting the same decision again. B/C,
physical heap/RSS, default/native policy and interruption remain separate designs.
Read-only measurements alone do not enforce the approved limits.
