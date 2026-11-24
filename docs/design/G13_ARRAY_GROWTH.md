# G13: snapshot array append and concatenation

Status: **Approved by the user and implemented.**
See the [API reference](../reference/array-growth.md) for verified scope. This is the next collection
contract in the [18-task checklist](../18_TASK_COMPLETION_CHECKLIST.md), following
implemented [snapshot slicing](../reference/array-slice.md). Review both growth
operations together; neither introduces mutating resize, references or new syntax.

## Approved operations

| Builtin | Contract | Result order |
| --- | --- | --- |
| std::array_append(values, item) | ([T], T) -> [T] | All original elements, then item. |
| std::array_concat(left, right) | ([T], [T]) -> [T] | All left elements, then all right elements. |

T describes compiler-retained element facts, not a source generic declaration.
Both operations return independent snapshots and leave inputs unchanged. Nested
arrays, record fields and owned text retain existing value semantics. Evaluation
is left to right and normal argument snapshot costs still apply. Repeated append
may copy repeatedly; do not promise amortized append or zero-copy concatenation.
No implicit mutation occurs when a result is unused.

## First-argument-directed typing

The first argument must establish a fully known canonical array element type.
Aliases and nominal package records remain preserved. The result has that same
array type. For append, check item against T; for concat, check right against [T].
Use existing assignment/parameter compatibility and recorded numeric conversions:
Int may widen to Float, recursively through arrays, but Float never narrows to Int.
Do not infer a new common type from both arguments or introduce heterogeneous Any.

| Example | Required result |
| --- | --- |
| array_append([1], 2) | [Int] |
| array_append([1.0], 2) | [Float], with existing Int-to-Float argument conversion. |
| array_append([1], 2.5) | Semantic error; first input fixes Int. |
| array_concat([1.0], [2]) | [Float], widening right before the operation. |
| array_concat([1], [2.5]) | Semantic error; no backward widening of left. |
| array_concat([1], []) | [Int]; existing empty-literal compatibility applies on right. |
| array_append([[1]], []) | [[Int]]; existing empty-array compatibility applies to item. |
| array_append([], 1) | Semantic error; annotate an empty first input explicitly. |

For empty growth, write `let values:[Int]=[]` then append/concat it. Reject any
Unknown leaf in the first argument's retained type without fabricating a checked
result. An expected return/local annotation does not introduce new backward
inference for this first slice. Known Unit elements remain legitimate values.
Existing empty/slice behavior is unchanged. Ordinary arity/type diagnostics keep
existing codes and precise source spans; rejected source exposes no successful
checked/lowered result.

Registry descriptors must not leak into result facts or IDE/public interface
types. Extend the existing dependent-result semantic/lowering path rather than
adding a generic substitution system. Record coercions for the second argument
so checked and legacy lowering agree, including aliased/nested Float arrays.
The actual converted argument is the one copied into the result. Widening remains
zero logical fuel under the approved model, with representation scratch excluded.

## Length and malformed-IR policy

Validate arity first, then runtime container kinds. Append requires an array first
argument; concat requires both arguments to be arrays. Runtime errors are
`std::array_append expects an array argument` and
`std::array_concat expects array arguments`. The second append argument is any
already validated VM value. Raw IR lacks static T metadata; do not pretend these
container checks dynamically reconstruct or prove homogeneous element types.
Source type safety remains the checked compiler's responsibility, as for MakeArray.

Require each newly returned array length to be at most **4,294,967,295 elements**,
the common JavaScript array representation ceiling. This bound applies
to these new operations, not retroactively to existing array construction/slicing.
Check length addition without allocation or integer rounding. Reject overflow or
an excessive result with `array result length exceeds limit`, before fuel/copying.
No truncation, clamping, prefix result or fallback is permitted. Test the length
helper at/above its boundary without allocating billions of elements.

This logical representation limit is not a memory safety budget: actual allocation
can fail far earlier. No new recoverable allocator/OOM guarantee is introduced.
Existing control/instruction admission precedes operation validation. These pure
helpers use no host authority and are not preempted midway; cooperative control
continues at approved checkpoints.

## Complete model-v1 charge

After type/container/length validation, premeasure and admit the complete result:

- Append: one new array node plus complete node/text footprints of all original
  children and the converted appended item.
- Concat: one new array node plus complete node/text footprints of all children
  from both converted input arrays. Input container nodes are not copied as children.
- Count nested String payloads and record type/field labels. Empty concat still
  creates one array node; nested arrays contribute their own nodes normally.
- Use checked u64 aggregation and atomic node/text admission. Construct iterative
  snapshots only after admission; never charge child clones a second time.
  Preserve iterative Rust guards for partial construction and error cleanup.
- Evaluation/parameter/LoadName copies remain separately charged. Dropping or
  overwriting inputs gives no refund. Shared physical Strings do not reduce cost.

These new events extend the complete cost table without changing existing
model-v1 events or defaults. The optional counters measure logical traffic, not
allocator capacity, scratch, peak live memory, GC or RSS.

Exact costs verified against actual checked IR:

```svr
fn main() {
    let result = std::array_append([1], 2);
    print(result);
}
```

Expected completion: 11 nodes, 6 text bytes, independently 7 output bytes. For the
same main using `std::array_concat([1], [2])`, expect 12 nodes and the same text/
output costs. Each includes print's Unit and the root implicit Unit. One-under
nodes fails at root completion after output; one-under text refuses print.
These are implemented logical costs, not measured heap limits.

## Independent consumer and qualification gates

Extend a reusable registered collection library (separate from its consumer) with
an operation such as keep_positive(values:[Int])->[Int], using typed-empty output,
a loop and returned append snapshots. Add an independently consumed concatenate
operation and a nominal record-array result. Library signatures remain ordinary
Sovra, not business-specific compiler primitives.

1. Check primitive/Unit/nested/record/alias inputs, typed-empty growth, second-side
   empty inference, rejected unknown first-side types, wrong arity/nonarrays and
   incompatible nominal/primitive types. Keep original diagnostic file/span ownership.
2. Prove first-directed numeric behavior and recorded second-argument coercions
   in checked/legacy lowering, standalone/package interfaces and IDE results.
3. Compare Rust, ordinary Node and controlled workers for success/errors/output,
   snapshot independence and exact/one-under fuel, Unicode/record labels, empty
   concat and length-limit ordering. No unmetered exhaustion fallback.
4. Exercise deep selected copies and refusal/partial cleanup at 16,000 levels,
   with reduced-stack workers and isolated Rust children; include wide inputs.
5. Run a locked independent package consumer through formatter/check/test/run,
   Node and actual Linux native output. Retain full regressions and exact-host
   artifacts. Other OS/MSRV/release/browser/mobile qualification remains separate.

Array append/concat do not complete G13. Map/Set/equality/iteration, general
generics, errors/sum types, binary/JSON and filesystem/time/process APIs remain
separate reviewed work. No new CLI/native defaults or Task Manager features.

## Approval boundary

[AGENTS.md](../../AGENTS.md) says: "Document and pause before fundamental syntax,
memory-model, type-semantics or compatibility decisions." The approved
[collection foundation](G13_COLLECTIONS_AND_DATA.md) requires reviewing the next
collection decision before implementation. The user approved both snapshot growth APIs, first-directed coercion/inference,
the explicit result-length ceiling and complete fuel/error order before
implementation. Do not request the same approval again. Future collection/error/
resource decisions remain separate; earlier slicing/resource contracts stay valid.
