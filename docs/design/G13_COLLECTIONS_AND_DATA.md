# G13: collection and data API foundation

Status: **First contract approved by the user and implemented (2026-11-13); G13 remains partial.**
Structural `[T]` annotations and `std::array_len` now work in executable programs
and package interfaces. G12 remains partial and continues independently. General
collections, error sum types, JSON and binary APIs are still planned.

## Starting point at approval

The executable compiler already has homogeneous array literals/indexing,
`Type::Array`, recursive numeric widening and snapshot value semantics under
[ADR 0012](../adr/0012-compound-value-semantics.md). Source annotations accept
primitive or qualified named types, not array type expressions. Consequently
arrays cannot be reliably declared in public function/record interfaces. G12's
registered interface normalizer explicitly rejects arrays. Existing standard
records provide specialized input/text/decimal results; they do not implement
sum types, generics or arbitrary JSON values. JSON used internally by compiler
reports/locks is trusted Rust tooling, not a Sovra JSON runtime API.

## Recommended first decision: structural array annotations

Implemented: recursive `[T]` type expressions wherever an existing type annotation is
accepted: parameters, returns, aliases, locals and record fields. Preserve `[a,b]`
as expression syntax in expression positions. For example:

```svr
mod numbers {
    export struct Samples { values: [Float] }
    export fn total(values: [Int]) -> Int {
        let mut index = 0;
        let mut result = 0;
        while (index < std::array_len(values)) {
            result = result + values[index];
            index = index + 1;
        }
        return result;
    }
}
```

`[Int]` and `[[Int]]` describe structural arrays; element record identities remain
nominal and package-owned. No general type parameter syntax, implicit Any,
heterogeneous array, new ownership rule or subtyping is introduced. Existing
compatible recursive Int-to-Float conversion remains the only numeric widening;
public array element types obey existing private-record leak restrictions.
Aliases resolve to array structures with cycle checks and existing resolution
budgets. Empty literals gain their element type from an explicit expected type;
without one, existing conservative inference/refusal remains. Type expression
parsing, resolution, retained evidence, linking and backend conversion all require
bounded depth/node/string accounting before publication of checked facts.

Add `std::array_len(values: [T]) -> Int` as a compiler-known array operation whose
result does not depend on the element type. This notation describes its contract,
not source-level generic declarations. Reject nonarray arguments semantically;
keep `std::len(String)` unchanged. No resizing, append, slicing, iterators or
shared mutation is part of this first decision. Length is element count, distinct
from UTF-8 byte length. Preserve checked IR and interpreter/JS/native-host parity.

Alternative: use `Array<T>` spelling now. That commits the parser/type system to
angle-bracket generic syntax before user-defined generic substitution, constraints
and alias/cycle rules exist. Recommendation: approve `[T]` only and keep general
generics a separate reviewed design.

## Subsequent decisions, still planned

1. **Collections:** resizing versus immutable returned arrays; deterministic
   Map/Set iteration, equality/hash keys and duplicate handling. Preserve value
   snapshots. Do not infer ownership from host implementation containers.
2. **Errors:** payload-bearing sum types, exhaustive matching and generic
   Option/Result; decide propagation explicitly. Existing specialized result
   records remain compatible until a separate migration is approved.
3. **Binary:** byte values/range policy, immutable buffers versus `[Int]`, codec
   result errors and byte offsets. String remains valid UTF-8; arbitrary bytes
   must never silently decode with replacement or lossy conversion.
4. **JSON:** recursive explicit value representation, duplicate-key policy,
   numeric precision/range, error byte offsets, depth/size/member/string budgets
   and canonicalization scope. No `eval`, prototype pollution, implicit record
   deserialization, filesystem authority or partial-success parsing.

Start each implemented API with a real independent package consumer and valid,
invalid, exact/exceeded-budget cases in interpreter and generated JS. Native
qualification must execute the claimed output on the actual host. Windows,
macOS, mobile/browser host APIs, MSRV and release qualification remain separate.

## Implementation order after approval

1. Retain bounded structural type expressions and update specification/course;
   keep old primitive/named stage behavior compatible.
2. Resolve aliases/records/functions and check empty literals, nested arrays,
   privacy, cycles and original source diagnostics.
3. Carry array type evidence through checked lowering, imports, registered module
   interfaces, interpreter and JS with coercion/value-snapshot regressions.
4. Add `std::array_len` and an independent local library/consumer. Update formatter,
   JSON symbol evidence and IDE type display for the new annotation syntax.
5. Review the next collection/error/binary/JSON decision before implementing it.

## Approval boundary

[AGENTS.md](../../AGENTS.md) requires: "Document and pause before fundamental
syntax, memory-model, type-semantics or compatibility decisions."
The user approved the `[T]` annotation and compiler-known array operation before
implementation. Continue their verification without requesting that decision again.
Existing G12 work remains authorized. No approval of this document implies approval of the later
Map/Set, Option/Result, binary or JSON designs.


## Implemented scope and retained limits

AST type spellings stay strings for stage compatibility; bounded parsing retains
nested brackets and complete annotation spans. Existing semantic `Type::Array`
represents checked structures. At most 127 array wrappers plus one leaf fit the
128-level type bound, including expansion across alias chains; cycles/excess fail.
AST admission counts every written wrapper, and package interface admission counts
normalized array wrappers for copied signatures/record shapes. Checked type/context
and interface string budgets remain active without fallback. Empty arrays use
explicit expected types without rewriting their intrinsic unknown type facts.

Checked lowering now widens numeric arrays at parameter, initialization, return
and record-field boundaries, preserving value snapshots. The independent
[package consumer](../../examples/typed-array-packages/README.md) executes in
interpreter/Node and tested Linux native output, including locked compilation.
Entry-only package and legacy-lowering parity also has regression coverage.
Formatter parsing/equivalence accepts annotations; compiler-owned local symbol
projection and the strict IDE decoder retain explicit nested annotations. Existing
inferred-type JSON labels remain `Array<Int>` for compatibility; those labels do
not define source generic syntax. Builtin registry `Array` metadata is not a source
type or a declaration of general generics.

Older partial application alias/record inspection explicitly refuses array type
declarations until its own field identity/privacy contract is integrated. This
slice does not complete M12/G11/G12, general G13 libraries or platform qualification.

Runtime resource qualification remains open: recursive record/array shapes can
construct values deeper than a source annotation. [Runtime traversal hardening](../reference/runtime-values.md)
now uses iterative Rust cloning/runtime-owned cleanup and both engines' display,
with explicit caller-owned Rust cleanup preserving public enum compatibility.
[Optional instruction/output allowances](../reference/execution-limits.md) now
have explicit per-invocation Rust/JS contracts. Approved cumulative logical
[node/text fuel](../reference/materialization-limits.md) is also implemented.
Physical heap, full-work/interruption and default/native policy still need contracts before
claiming hostile-program resource isolation. This does not establish a process
sandbox or a general ownership model.


## Next concrete collection review

The separately reviewed [array_slice contract](G13_ARRAY_SLICE.md) now implements
a compiler-known typed snapshot result, half-open bounds/refusal, checked logical
charges and an independent registered-library consumer. See its
[reference](../reference/array-slice.md).
The [expanded 18-task checklist](../18_TASK_COMPLETION_CHECKLIST.md) also makes
filesystem, dates/time and processes explicit G13 work, with G14 resource/authority
contracts and G12 distributed-package consumption tracked separately.


The separately approved [array growth contract](G13_ARRAY_GROWTH.md) now
implements snapshot append and concatenation with first-directed typing,
second-argument coercions, explicit length refusal and complete fuel costs.
See the [reference](../reference/array-growth.md); slice/resource contracts stay valid.


## Next error-handling foundation

The proposed [payload-enum syntax slice](G13_SUM_TYPE_SYNTAX.md) adds bounded
single-declaration variant/payload parsing beside unchanged older stages. It is
not an implemented API, typed enum value, exhaustive match or Option/Result.
Later constructor/handling/runtime contracts remain explicit review gates.
