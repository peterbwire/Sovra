# M12 model fields: first typed slice

Status: **Option A approved by the user; implemented as an additive Rust source API.**

`application::check_model_primitive_fields` implements this bounded slice.
Default project/CLI checking and aggregate header coverage remain unchanged.

This proposal makes the next G11 step concrete after integrated partial header
coverage. Current model bodies remain explicit gaps. Accepted
[ADR 0009](../adr/0009-m12-checker-contract.md) already selects nominal model
identity, explicit annotations, canonical primitive names and library-owned domain
capabilities. It does not settle a complete model-field grammar.

## Accepted decision

**Accepted option A:** support explicit fields separated by newlines or commas,
with a single optional trailing comma. Start semantic checks with canonical
primitive types and file-local aliases that resolve to those primitives.

This preserves the newline style of the proposed Fielddesk models while accepting
ordinary comma-separated field spelling. This approval covers the bounded source
parser/checker slice below; it does not approve defaults, generic/optional syntax,
model value construction, persistence, field layout, ownership or runtime APIs.

**Alternative B:** require comma-separated fields only, matching executable record
separators. Existing proposed newline-only model examples would need migration
before their model fields could validate. All other first-slice limits below can
remain the same.

## Source grammar for the additive checker

The following are accepted field-checker fixtures, not executable model examples:

```svr
model Item {
    id: Int
    title: String
    active: Bool
}
```

```svr
model Item { id: Int, title: String, active: Bool, }
```

Mixing comma and newline separators is allowed:

```svr
type Label = String;
model Item {
    id: Int,
    title: Label
    active: Bool,
}
```

Rules:

- A field requires an identifier, colon and explicit simple annotation name.
- Annotation names resolve only to Unit, Bool, Int, Float, String or a file-local
  alias resolving to one of those primitives. There is no field type inference.
- Ordinary whitespace/comments may occur within a field. A newline is a separator
  only after its annotation is complete. Blank lines/comments are not empty fields.
- Between two complete fields, require a comma or at least one physical newline.
  LF and CRLF have the same meaning. A closing brace ends the final field.
- One trailing comma is permitted. Empty interior comma items are errors.
  Semicolon separators are outside this first slice.
- Source declaration and field order are retained. Duplicate field names are
  errors within their declaring model.
- An empty model body is admitted as an empty field table only. It establishes
  no storage or construction semantics.

These inputs fail field parsing:

```svr
model Bad { id: Int title: String }
model Bad { id: Int,, title: String }
model Bad { id }
model Bad { id: }
```

These have semantic errors:

```svr
model Bad {
    id: Int
    id: String
}
model Bad { amount: Money }
```

`Money` must be explicitly supplied by a later supported declared/imported type
context. Capitalization grants no special status. An explicit file-local
`type Text = String;` may be used under normal primitive-alias rules; undeclared
`Text` is not implicitly String.

## Identity and semantic scope

Models retain model declaration identity and source ownership. Implementations
must not translate models into executable StructDeclaration nodes or register
implicit record constructors to obtain checking accidentally. This slice validates
field declarations/primitive annotations; it does not make model values usable in
functions or change ordinary executable record behavior.

Reuse core primitive/alias resolution and bounded diagnostics. Diagnostics
reuse E3008 for duplicate declarations/fields and E3017 for unknown annotations,
with model-appropriate messages and exact original field-name/type token ranges.
Existing alias diagnostics remain intact. No new diagnostic code is reserved here.

Aliases to records, models, enums, arrays or unresolved/imported contexts are
explicitly outside this first primitive-only checker. Declared unsupported types
must not be reported as primitive field facts or silently erased. Imports and
qualified annotations require a later contextual checker. Unknown undeclared
simple annotations produce errors; unsupported syntax/context makes the scoped
request unavailable without a partial valid-field table.

Default expressions, field mutation, relationships, recursive models, enum variant
semantics, `Id<Model>`, `Set<T>`, `T?`, queries, generated `Model.find`/`insert`,
serialization and runtime allocation/layout are not selected by this proposal.
Their source stays unparsed/unavailable in the relevant partial report. No
unsupported suffix may be discarded after a valid annotation prefix.

## Integration and resource limits

Implement an additive source checker first, retaining original source spans and
separately reporting field/type diagnostics. Preserve existing header-only and
project-checker APIs, including E4096. Typed field metadata must never convert
incomplete application checking into success or authorize a policy referencing a
model name. Broader project integration remains an explicit later step.

Admission limits reuse current scales:

- 1 MiB source; 65,536 non-EOF tokens; structural depth 128.
- 4096 model headers; 8192 cumulative fields per source.
- 1024 diagnostics and 1 MiB cumulative diagnostic-message bytes.
- No source writes, filesystem import resolution, runtime execution or new
  dependency. No Task Manager feature changes.

Exhaustion makes the optional field projection unavailable, never a truncated
valid-field table. Existing semantic diagnostics/default checking stay independent.

## Implemented sequence

1. Retain source-owned model field nodes with name/type/declaration ranges and
   implement the approved separator grammar.
2. Validate duplicate names and primitive/file-local primitive-alias annotations
   through shared compiler rules, keeping model identity separate from records.
3. Test valid and invalid cases below, diagnostic/source parity, exact/exceeded
   quotas and unchanged older APIs. Update documentation and the maturity audit.
4. Report actual supported-host evidence; keep full M12/platform/release gates open.

## Acceptance matrix

| Case | Expected first-slice result |
| --- | --- |
| Newline, comma and mixed separators | Accepted with ordered fields under option A |
| LF/CRLF, Unicode comments, blank lines | Equivalent fields; exact original byte/scalar spans |
| Explicit canonical primitives and primitive alias chains | Field type facts use core canonical types |
| Duplicate field or invalid/unknown annotation | Bounded diagnostic at owning source token |
| Missing separator/name/colon/type or repeated comma | Whole scoped field request rejected |
| Default/generic/optional/model-valued field | Explicit unsupported context/syntax; no partial valid field table |
| Invalid/cyclic alias definitions | Existing core alias errors retained |
| Same fields in different model declarations | Distinct declaration ownership; no structural type unification |
| Empty model | Zero fields; no runtime/type-construction claim |
| Exact/exceeded field and diagnostic quotas | Exact boundary accepted; excess unavailable without partial success |
| Existing aggregate/checker APIs | Behavior and incomplete-check diagnostics unchanged |
| Policy model reference points to checked fields | Name identity only; no permission or condition validity inferred |

## Approval boundary

[AGENTS.md](../../AGENTS.md) requires documenting and pausing before fundamental
syntax/type-semantics/compatibility decisions. The user approved option A and this
scoped first slice before implementation. Existing
[structured coverage](../reference/application-type-syntax.md) remains available;
this slice does not complete nominal model typing or G11/M12. See the
[API reference](../reference/application-type-syntax.md#primitive-model-field-checking).


## Subsequent project integration

The additive `check_service_calls_with_model_fields` now exposes these local
field results from the same source strings as service inspection, with owning
file paths and cumulative admission limits. It preserves every original checker
diagnostic and does not resolve imported field types. See the [project API](../reference/application-type-syntax.md#optional-project-model-field-checks).
