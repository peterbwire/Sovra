# JSON check reports

`check --format json --symbols <source.svr>` opts into checked symbol/type evidence on
successful standalone checks. Default reports remain unchanged. Failed checks
omit evidence; absence does not mean the source has no declarations. The option
requires JSON, rejects directories and cannot combine with `--service-calls`.

The additive `symbols` object contains `declarations`, `references`, and a
`truncated` boolean. Declarations have qualified `name`, bounded source `signature`
excerpt, declaration byte `start`/`end` and identifier `name_start`/`name_end`.
References describe actual AST call targets with byte `start`/`end` and nullable
`definition` (an index into declarations). Standard-library calls instead have
null definition and a registry `signature`. They have no invented source file.
No evidence is inferred from strings/comments or unresolved/invalid source.

Additive `locals` and `local_references` arrays supply checked parameter/local
identities. Local declarations use the same range/signature fields as functions,
plus `kind` (`parameter` or `local`). Their references use byte start/end and
`definition` indices into `locals`, independently of function indices. Branch and
loop scopes restore outer identities; initializer references precede insertion of
a shadowing binding. Variable/function namespaces are distinct. Field receivers,
array targets/indices and assignment targets are visited; field names are omitted.
Explicit annotations retain their source spelling. Unannotated locals add
`inferred_type`: a bounded concrete type label or null when inference contains a
hole or the display budget is exceeded. Concrete signatures read, for example,
`let count: Int (inferred)` or `let values: Array<Array<Int>> (inferred)`.
Nominal labels use checked standalone qualification. A null label retains
`inferred type unavailable` in the signature; it does not invent Any/Unknown as
a concrete type. Annotated locals and parameters omit `inferred_type`.
Declarations and reference targets now project the immutable checked snapshot;
the IDE collector does not repeat name resolution or type inference.

This is partial evidence, not a complete semantic symbol graph: aliases, fields
and records are omitted. Raw annotation spellings
remain in source signatures, and module function names retain qualification.
Each projected declaration/reference table is capped at 8,192 entries and shares
a 512 KiB report allowance. Source signatures, type labels and local signatures
are at most 4 KiB, including any ellipsis. `truncated: true` marks omitted records,
unavailable type display or signature clipping. References to omitted declarations
are omitted, never redirected to same-spelled outer bindings; included indices
remain valid. Nested array labels use `Array<T>` and remain subject to the label
byte allowance, not an extra syntax-depth restriction.

The opt-in collection request uses the existing 250,000-node admission allowance
for structural/fact/declaration/use/coercion counts, 8,192 interned types and
512 KiB each for retained nominal names and declaration names. Source admission
still uses the ordinary frontend limit. If evidence collection exhausts a budget,
ordinary checking is retried without evidence: source errors still fail normally;
valid source returns success with empty symbol arrays and both `unavailable: true`
and `truncated: true`. No partial checked snapshot is projected. These are IDE
request/projection allowances, not new language acceptance limits or total
memory/CPU quotas. Default JSON reports and ordinary check/run/build remain
unchanged. The LSP strictly validates bounded tables, strings, UTF-8 byte ranges,
definition indices and inferred metadata before using it.

IDE consumer (2026-11-11): `toolchain/ide/language-server/` checks temporary
standalone snapshots through this existing schema. It maps snapshot diagnostics
back to the open document URI and converts UTF-8 byte offsets to LSP UTF-16
positions. This does not change report offsets or add project/package validation.

Default and explicit Rust frontend budget APIs return E1003 for source/token/lexical
diagnostic exhaustion and E2008 for parser diagnostic exhaustion. These are error
diagnostics using the existing schema. Pre-tokenization size rejection has no
token location; other limits retain the stopping position. Approved executable
source defaults are 16 MiB, 1,000,000 tokens and 256 frontend diagnostics. Package
errors retain the owning source file; scanner-only project validation has
different coverage. See [ADR 0035](../adr/0035-frontend-resource-budgets.md).
Executable package compilation additionally enforces a 64 MiB aggregate source
ceiling across canonical packages. Aggregate exhaustion reports E1003 at the
owning entry file, including unused dependencies; aliases count once. Package
language-test reports contain no executed tests after compilation exhaustion.
Manifest/graph exhaustion uses E4112: 1 MiB per manifest (including initial
classification), 8 MiB total canonical manifest bytes and 8,192 alias edges.
Byte overflow identifies the manifest; excess edges retain the declaration-header
range. Ordinary scanner-only checks also enforce the per-manifest read cap.

Malformed manifest inspection and executable semantic analysis return at most
256 diagnostics per failing stage, including a terminal E4018 (manifest) or
E3019 (semantic) when the capacity is reached. Original ranges and owning files
are retained; earlier diagnostic codes remain unchanged. Remaining errors may
be omitted and no partial checked program executes. This changes error-report
presentation for invalid inputs, not acceptance of valid source. Schema version
remains 1; diagnostic message sizes and total analysis/IR work are separate limits.

Messages from guarded compiler emitters are capped at 4,096 UTF-8 bytes including
`[message truncated]` when clipped. Codes, severity, source-file identity and full
ranges are retained; clients should use those structured fields rather than
parsing messages as complete identifiers/type descriptions. JSON escaping can
expand serialized length beyond the decoded message limit. Schema version stays
1 and public raw diagnostic/report construction does not automatically truncate.

Main project/manifest and executable package diagnostics stream formatted text
into the existing message allowance. Cycle path order, route/page error prefixes,
Unicode clipping, error codes and source provenance are preserved; the version-one
schema and decoded message ceiling are unchanged. This does not bound source-file
identity or JSON escaping.
CLI I/O failures also use bounded message production while preserving the full
requested path and null diagnostic location. Language-test configuration messages
retain their full declaration ranges and existing codes.
Human CLI error rendering now escapes terminal controls. JSON reports continue
to preserve original decoded message/target/file values and the version-one
schema; human escape sequences are not substituted into those structured fields.
The JSON encoder additionally escapes DEL/C1 (U+007F–009F) with standard JSON
`\uXXXX` sequences, so those controls are absent as raw scalars from serialized
string fields. Decoding recovers the original values exactly; only representation
changes, with schema version 1 retained. Existing C0 short escapes are preserved.

The caller-selected primary-project scan API can report E4150 for discovery/read
allowance exhaustion. Its message identifies the resource/path; filesystem
admission errors have a null location instead of invented source offsets. Limited
scan read failures retain E4009 and stop further reads. Primary project checks
now enforce the approved ADR 0040 defaults: 16 MiB/source, 64 MiB total, 4,096
source paths, 4,096 directories and 100,000 yielded entries. Optional body/service
inspection remains separate; schema version stays 1.

Explicit AST admission and IR emission APIs can return E3020 (syntax-node
allowance), E3021 (emitted instructions) and E3022 (owned IR string payload bytes).
These retain source context and use the existing diagnostic schema. Standalone
source checking now rejects more than 250,000 AST nodes with E3020. Standalone
run/build/test also limit retained IR to 1,000,000 instructions and 64 MiB of
string payload (E3021/E3022); see accepted ADR 0037. Package linking remains separate.

Caller-selected `packages::compile_with_compilation_limits` also uses E3020 for
cumulative canonical-source AST admission and E3021/E3022 for final linked IR.
E3023 reports cumulative interface metadata-node exhaustion, using source context
from the declaration or consuming import statement. Schema version remains 1;
package CLI compilation now enforces the approved ADR 0038 defaults: 1,000,000
AST nodes, 250,000 interface nodes, 4,000,000 linked instructions and 256 MiB
retained linked string payload. Partial application scanning remains separate.

`packages::compile_with_interface_string_limit` can report E3024 when cumulative
interface string materialization exceeds a caller-selected byte allowance.
Imported copies point to the consuming import; local metadata uses declaration
context. Schema version remains 1. Ordinary executable package compilation and
its check/run/build/test CLI paths now enforce the approved 128 MiB cumulative
allowance; the explicit API remains caller-selected (accepted ADR 0039).

Parser declaration recovery reports one E2000 for a contiguous run of unexpected
declaration tokens, then resumes at the next declaration (or module closing brace).
Later independent syntax errors still appear with their original source ranges.
Consumers must not rely on one diagnostic per invalid token or a fixed count of
cascading errors. Diagnostic codes and schema version remain unchanged; any parser
error still prevents compilation and execution.

Statement recovery preserves nested block boundaries and subsequent unambiguous
statement keywords. Independent malformed statements in both branches of an
`if` retain their original E2001 ranges rather than producing misleading outer
declaration errors. `run` and `build` still reject the complete source.

Malformed indexed statements no longer duplicate errors during assignment-target
probing. E2001 identifies an absent index expression; E2005 identifies a missing
closing delimiter. Subsequent independent statements retain their own locations.

Semantic target errors also preserve independent expression diagnostics. An
undefined assignment target or non-array indexed target does not hide errors in
its right-hand side. An unsupported computed call still checks its callee and
arguments. Existing E3001/E3003/E3007 codes and schema version 1 remain unchanged;
reports may contain additional errors at their original expression ranges.
Record literals likewise retain value errors for unknown records and duplicate
fields, alongside existing E3004/E3008 diagnostics. Missing-field errors are
ordered alphabetically by field name within each literal, independent of the
compiler's hash-map iteration order.

## Executable local packages

Opt-in application diagnostics include E4140 for an invalid direct local
reassignment. Its source range identifies the target name. Reassignments do
not add a new `local_bindings` entry; that array describes declarations.
E4141 reports an invalid indexed assignment with the indexed target's range.
E4142 reports an invalid indexed read at its expression range, and E4143
reports a known incompatible array literal at the literal range. Invalid
expressions can also leave a local initializer unresolved (E4136).

For directories declaring dependency sections, `check --format=json` now resolves,
checks and links package entry sources. The schema remains version 1 with
`kind: "project"`; errors carry their owning manifest/source file and original
byte ranges. These reports omit application-scanner fields (`service_operations`,
`service_coverage`, `member_calls`); absence is not an empty application scan.
E4110–E4113 describe graph/path failures, E4114 malformed executable imports,
and E4115 missing direct dependencies/modules.
E4116 reports private or unsupported types exposed by an exported function
signature or record field; its location identifies the library's annotation.
Explicitly exported records retain nominal package identity (ADR 0011).
Scalar aliases are resolved in the library before consumer checks. Errors retain
E30xx codes, including E3004 for inaccessible private calls. `--service-calls`
is rejected with exit 2 for executable dependency packages.

Directories without dependency sections still use partial application checking.
Standalone `.svr` checks retain their source-only behavior and do not load manifests.

## Experimental service-call checking

Use `svr check --service-calls --format json <project-directory>` to request
service operation/positional-arity and primitive signature validation after ordinary
project validation.
The flag is project-only; source-file use or repeated flags return usage exit 2.
Normal checks are unchanged. Once inspection runs, the version-one report adds:

```json
"service_coverage": {
  "complete": false,
  "files": [{"file": "app/jobs.svr", "inspected": false, "reason": "unsupported application syntax"}]
}
```

Each file has an inspected boolean and nullable reason. Complete means all files
fit the supported inspection syntax, not that application types or all ordinary
member calls were validated. E4093/E4094/E4095 report resolved service-call errors.
Unsupported files add E4096 with null diagnostic location; their paths/reasons
appear in coverage. Incomplete inspection returns exit 1 and success false even
when no service-call errors were found. Complete inspection with valid signatures
and no call errors exits 0. E4117 reports unresolved service parameter/return
annotations, including named types and generic forms not yet connected to this
checker. Supported types are Unit, Bool, Int, Float and String; omitted returns
mean Unit. E4118 reports a failure to canonicalize the signature's source owner.
Both use the original file and whole declaration range, not an annotation range.
E4119 reports literal argument type mismatches at the argument expression range.
Int may widen to Float. Primitive parameters and inferred literal/binding locals
also participate in E4119 checks with lexical shadowing. Primitive annotated
locals propagate their declared type only after a known compatible initializer;
E4120 identifies incompatible initializers at their expression ranges. Unknown
initializers produce E4136 at their expression ranges; unsupported local annotations
produce E4135 at annotation ranges instead. These checks include unused locals. Supported primitive
binary arithmetic/comparison expressions contribute argument and initializer types.
E4121 reports incompatible known operands at the expression range. Resolved service
and ordinary calls inside &&/|| are inspected on both sides; invalid logical
operand types also produce E4121. Logical conditions retain E4125 when their type
cannot be resolved. Resolved service
call results contribute types when receiver, arity and all argument types validate;
invalid or unknown arguments prevent result propagation. E4122 reports known
service return mismatches at the expression range (the keyword for bare returns).
E4123 reports a non-Unit service body that can fall through at the full function
range. Both if/else arms must return; while loops alone never guarantee return.
E4124 identifies known non-Bool if/while conditions at their expression ranges.
Unresolved condition types produce E4125 and unresolved service return types
produce E4126, at the expression range. Both fail the check even when syntax
coverage is complete; they report incomplete validation rather than a mismatch.
Same-file ordinary call results now resolve from primitive signatures. E4127
reports duplicate ordinary/task declarations (including function/task collisions),
E4128 wrong arity, E4129 incompatible
arguments, E4130 unresolved call signature/argument types, and E4131 ordinary
return-contract errors. Arity uses the whole call range; argument errors use
argument ranges. Return annotation/fallthrough errors use the declaration range;
explicit returns use expression/keyword ranges. Direct exported application
function imports resolve under ADR 0013. JSON schema 1 adds `ordinary_calls`, retaining the existing
member-call metadata and diagnostics. Records include calls resolved through
the Rust stdlib registry. Stdlib arity,
argument mismatch and unresolved-argument failures use E4128/E4129/E4130. E4132
identifies an ordinary declaration colliding with a builtin callable. User Any
annotations do not acquire builtin wildcard behavior. Typed signature
records are available through the Rust API, not additional JSON fields.
E4133 reports unresolved ordinary callees (including local/computed function values
and unsupported imports) at the whole call range. It is an incomplete-validation
error, not a claim that every unresolved name is absent from the project.
Known scalar member calls also produce E4133, with the whole call range and an
unresolved ordinary-call record explaining that the receiver has no callable
members. Existing member-call receiver metadata is retained.
Unresolved member receivers also emit E4133 and an unresolved ordinary-call record
at the whole call range. Local/unresolved member metadata remains visible; its
classification alone must not be interpreted as successful call validation.
E4134 identifies unresolved ordinary function or task parameter annotations at the
owning declaration range, including unused declarations. It emits one diagnostic
per parameter.
E4131 also reports unresolved explicit task return annotations at the owning
declaration range. Under ADR 0017, it also reports task return mismatch or
unresolved value at the return expression/keyword, and non-Unit fallthrough at
the task declaration. Omitted task results default to Unit in M12 checking.

Each `ordinary_calls` record contains `function`, `is_task`, nullable `callee`,
`kind` (`ordinary`, `builtin`, or `unresolved`), nullable `reason`, nullable
`declared_return_type`, an `arguments` array and `location`. A nullable
`declaration_file` is also present on each call: canonical owner for imported
functions, the current file for local ordinary functions, null for builtins and
unresolved calls. Each argument has a
nullable `type` and `location`. Types currently use Unit/Bool/Int/Float/String;
null means no supported primitive type evidence. The declared return type describes
the signature, even when arguments fail validation; it is not a validated call
result. A computed callee has null `callee`. Calls are sorted by source range
within each function, not evaluation order. Only fully inspected files contribute.
An empty array does not establish full project coverage. Plain source/project
reports omit this opt-in field. Existing schema-version-one fields retain their meaning.

Unresolved discarded non-call expressions produce E4137 with the complete
expression location and owning file in `diagnostics`. File-local scalar aliases
appear as their canonical scalar type in call and binding evidence (for example,
an annotation `Amount` targeting `Float` reports `Float`). E3008/E3017 alias
diagnostics retain their declaring file and declaration ranges.
These errors do not mark parsing coverage incomplete: inspected syntax can still
fail type validation.
Direct call statements retain their existing callable/argument diagnostics.

Opt-in reports also add `local_bindings`, with `function`, `is_task`, `name`,
nullable `declared_type`, `initializer_type` and `resolved_type`, and `location`,
nullable `annotation_location` and `initializer_location`. Ranges use the existing
file/byte/line/column shape. Declarations exclude the optional trailing semicolon.
Only supported primitive evidence is serialized; null is not proof of validity.
Nominal application record evidence also currently serializes as null in these
scalar fields. Scalar field reads, such as a Float `point.x`, supply canonical
scalar evidence. Invalid member expressions produce E4138 with original ranges.
Invalid record constructors produce E4139 at the full constructor range and do
not provide a resolved local/argument type. Valid nominal results retain the
existing null scalar representation; their scalar field reads provide type evidence.
A valid record contract can therefore have null type evidence;
use diagnostics and coverage rather than interpreting null as an error by itself.

Additive schema-one type descriptors distinguish valid nominal evidence from
unresolved evidence without changing the existing scalar fields:

- Locals add `declared_type_info`, `initializer_type_info`, `resolved_type_info`.
- Ordinary calls add `declared_return_type_info`; their arguments add `type_info`.
- Member calls add positional `argument_type_info`, preserving the numeric
  `arguments` field.

A descriptor is `{"kind":"scalar","name":"Float"}`, for example,
`{"kind":"record","identity":"..."}`, or
`{"kind":"array","element":{"kind":"scalar","name":"Int"}}`.
An empty array has a null `element` until compatible element evidence is
available. Unresolved/unsupported evidence is null.
Record identity is an opaque equality token derived from the declaring source
module and record; do not parse it or treat it as a portable package identifier.
Aliases to the same record share identity, while identical layouts from different
declarations do not. Type evidence does not replace diagnostics or establish
runtime validity. Plain reports still omit these opt-in inspection records.
An absent annotation has null annotation_location, while an unsupported annotation
retains its location and has null declared_type. Entries are lexical occurrences
in source order, not a flattened symbol table: nested bindings may reuse names.
Only fully inspected files contribute, and plain source/project reports omit it.
If initial manifest/import/wiring validation fails, inspection does not run and
the ordinary diagnostic report has no service_coverage field. Service-call reports
do not currently include service_operations; use ordinary project JSON for metadata.
Human output reports inspected/total files and prints errors to stderr.

Status: Implemented report format for the current source checker and partial
project checker. Use `svr check --format json <source.svr|project-directory>`.

For recognized service contracts, E4025 identifies the repeated operation's
full source line. E4026 identifies the service declaration when a required
opening or closing brace is missing, or its header has unsupported trailing
syntax (including nonempty inline bodies). Both retain the `.svr` file identity,
using the existing location schema. These checks remain a partial scan, not
service signature typing or execution validation.
`--format=json` also works; the option may appear before or after the path.
The default is `human`, and `--format human` selects it explicitly.
Use `--` before a path that starts with `-`; subsequent text is treated as a
path, including names such as `--help`.

After valid arguments, JSON mode writes one UTF-8 JSON document plus a newline
to stdout for a completed check or an input I/O failure. It does not also print
the human success summary or diagnostics. Exit codes are 0 for a successful
check and 1 for a validation/I/O failure. Invalid command arguments (including
unsupported/duplicate formats, wrong path count and non-`.svr` source paths)
retain exit 2 and a human error on stderr, with empty stdout. Help remains
human-readable and exits 0. Consumers must inspect the exit status before
assuming stdout contains a report.

```json
{
  "schema_version": 1,
  "target": "tests/fixtures/untyped-parameter.svr",
  "kind": "source",
  "success": false,
  "diagnostics": [
    {
      "severity": "error",
      "code": "E3014",
      "message": "parameter `value` requires an explicit type annotation; write `value: Type`",
      "location": {
        "file": "tests/fixtures/untyped-parameter.svr",
        "start": 10,
        "end": 15,
        "line": 0,
        "column": 10
      }
    }
  ]
}
```

| Field | Meaning |
| --- | --- |
| `schema_version` | Integer 1 for this wire contract. |
| `target` | Requested path as supplied; not necessarily absolute/canonical. |
| `kind` | `source`, `project`, or null if filesystem inspection could not classify the target. |
| `success` | Whether the report contains no error-severity diagnostics. |
| `diagnostics` | Ordered array; empty on successful checks today. |
| `severity` | `error` or `warning` (the current check pipeline emits errors). |
| `code` | Existing compiler/project code; E0001 identifies CLI input inspection/read failures. |
| `message` | Description, JSON-escaped without discarding Unicode or control characters. |
| `location` | Source location object or null where provenance is unavailable. |

Source locations use UTF-8 byte offsets `[start, end)` and zero-based line and
Unicode-scalar column indices. A supplementary character counts as one scalar,
a combining mark counts separately, and a tab counts as one rather than an
expanded terminal tab stop. Positions describe the original source spelling,
including escape sequences, rather than decoded string contents. CRLF starts
the next line once. Human diagnostics display one-based line/scalar columns;
LSP converts byte ranges to UTF-16 code-unit positions. These coordinate systems
must not be interchanged.
Source structural-depth rejection reports a single E2007 with the 128-level
limit and the offending delimiter/operator/call-token location. Excessive
application depth instead yields E4096 with incomplete per-file coverage when
`--service-calls` is requested. Neither report means a runtime execution budget
has been enforced.
Expression diagnostics identify the relevant variable, literal, binary
expression, callee or argument. Arity diagnostics cover the full call; grouping
parentheses are included in a grouped expression's range. Binding/return
contract and declaration diagnostics retain statement/declaration ranges.
All-zero fallback spans are rendered as null;
for example, an empty-input EOF position cannot be distinguished from an
unavailable position with the current span representation. Invalid-character
diagnostics do include the full UTF-8 byte range, even at the start of input.

Project checking still validates manifests and application wiring through a
line-based scanner. `kind: "project"` does not imply source parsing, function
type-checking or runnable application behavior. Its diagnostics retain
per-file identity for manifest parsing, source scanning, manifest-value and
wiring errors associated with declarations. These errors
now include the scanned file and a full-line byte range, including indentation
but excluding LF/CRLF line endings, with column zero. The file path is formed
from the requested project directory and need not be canonical or normalized.
Missing-target errors identify the referencing declaration. Duplicate errors
identify the repeated declaration. Manifest-value errors identify the assignment,
including empty required values and entry paths referring to absent files.
Entirely missing required keys, discovery and I/O errors still have
`location: null`; the formatter does not invent a source range for them.

Successful JSON checks return the same envelope with `success: true` and an
empty diagnostics array. Successful project checks additionally include
`service_operations`, an array (empty when no operations were found). This is
an additive version-one field; consumers should ignore unknown object members.
Failed project checks and source checks omit it; absence does not mean an empty
successful scan.

Missing service parameter annotations and missing annotations in recognized
top-level function/task/page/view signatures fail project checking with E4097 at
the signature line (ADR 0009). Malformed parameterized function/task/page/view
signatures report E4098. The structural metadata schema retains nullable
annotations for compatibility; successful project checks require them present.
Annotation names are still unresolved, not verified types.

Each operation contains `service`, `name`, ordered `parameters` (each with `name`
and nullable `annotation`), nullable `return_annotation`, boolean `has_body`,
and `location` with file/start/end/line/column fields. Annotations are unresolved
source text. Null means omitted, not inferred Unit or Any. `has_body` means a
body starts on the declaration line or the next nonblank, noncomment line,
not that it was checked or can execute. A semicolon ends a declaration-only
signature. Operation metadata locations cover all collected parameter-list
lines for multiline signatures. Scanner diagnostics identify the opening
signature line; parameter-token diagnostic ranges remain future work.
Operations follow sorted source-file discovery and declaration order. Empty
services have no entries. General statistics and resolved symbols remain planned.

Opt-in `check --service-calls --format json <project>` reports also expose
`member_calls`, an additive version-one array from fully inspected files only.
Service operation bodies use qualified enclosing names such as `mail.relay`
and `is_task: false` (ADR 0008). Declaration-only operations emit no body records.
These names are metadata, not new executable identifiers.

Each record contains `function` (enclosing declaration name), `is_task`,
`operation`, `arguments` (positional count), `location` (member-expression range),
and `receiver`. The receiver has a `kind` of `service`, `local`, `unresolved`, or
`ambiguous`, and a `candidates` array of `{module, name}` service identities.
A service receiver has one candidate, ambiguity retains all candidates, and
local/unresolved receivers have none. Module identities currently use canonical
local file paths; they are not portable package identifiers.

Member locations cover the complete receiver through the member name, excluding
the outer call's argument list. For `mail.create().send(value)`, the range is
`mail.create().send`; grouping parentheses are retained. This includes literal,
binary and chained receivers even when their classification is unresolved.

Calls follow file/function inspection order and expression traversal order
(nested argument calls precede their containing member call). Qualified namespace
calls are not member-call records, but member calls in their arguments are.
These records describe resolution, not successful type checking or execution.
Always consult `service_coverage`: missing call records from an unsupported file
do not imply that it contains no calls. Ordinary checks omit `member_calls`.

Rust diagnostics carry optional `source_file` provenance; stage function
signatures and the version-one JSON envelope remain unchanged. Serialization
lives in `src/compiler/check_report.rs`. Related-location notes and richer
diagnostic rendering remain separate work.

### Explicit checked-evidence exhaustion

Opt-in `EvidenceError::into_diagnostics` and
`CheckedPackageError::into_diagnostics` use E3025 for structural, retained-source
or semantic evidence allowance exhaustion. Existing parse, semantic, interface
and emission diagnostics pass through unchanged. E3025 carries severity `error`
and a null location: no source byte range or file is invented. The report target
still identifies the invocation. Inspect a structured package error before
conversion if its canonical failing entry is needed. Standalone run/build/native emission and language-test compilation now use
active bounded checked evidence and may return E3025. Plain semantic check and
the separate IDE evidence unavailability/fallback policy remain unchanged.
A successful plain check does not guarantee later compilation-budget admission. See [the evidence policy](checked-evidence-policy.md).

Ordinary package compilation (including package check/run/build/test and
caller-selected source/graph/interface/IR limits) now uses cumulative checked
metadata defaults and may report E3025 with null location. Explicit legacy package
lowering is available to compatibility/parity consumers and is not an automatic
fallback. Structured explicit checked compilation still retains the failing entry
before diagnostic conversion. Plain standalone semantic check remains separate.

The compiler-only package overlay API can return E4117 for invalid/duplicate
canonical targets and E4118 for overlay admission limits. These diagnostics have
null locations. Parse/semantic diagnostics retain canonical entry identity and
byte offsets into the supplied unsaved buffer. The CLI now supports `--overlays-stdin` with JSON-only package checking;
[the versioned wire contract](source-overlay-wire.md) defines request IDs, E4119
framing/admission and E4120 report-size failures. LSP package integration remains
pending; see [ADR 0042](../adr/0042-unsaved-package-sources.md).

## Package task qualification update

G09 existing-entry package tasks and real VS Code workspace acceptance are now
Linux-qualified. See [editor package tasks](editor-package-tasks.md) for save
policy, cancellation and platform limits. `check --executable-package` performs
full package entry compilation without execution; ordinary directory checks keep
the partial application scanner. This flag cannot combine with overlay, symbols
or package-context service modes.


The additive Rust [package source-inventory API](package-source-inventory.md)
uses E4160 for registration/path/identity errors or unavailable host identity
checks, with manifest-owned locations where available. This API has no new CLI
mode or JSON schema extension; current CLI compilation still rejects `[modules]`.

The source-unit Rust API additionally uses E4161 for capture/UTF-8/unavailable-host
errors and E4162 for registered module header/shape mismatches. Existing lexical,
parser and E3008 duplicate-module errors retain source-owned locations. These
library outcomes do not add a CLI mode or change the JSON schema.

The additive registered primitive-function compiler uses E4163 for unavailable
user record interfaces in its scoped mode. Core body/call and existing interface/
metadata/emission codes retain source ownership. This is a Rust compiler API,
not a new CLI JSON mode; older executable record behavior is unchanged.

E4164 identifies unavailable nonregistered field contexts in the separate
registered record Rust API; visible declared record-valued fields are implemented. E3008/E3017 field failures and E4116 private
interface/field leaks retain provider ownership; construction/call errors retain caller
locations. No new CLI JSON mode is implied.

Executable CLI checks now route registered graphs through source-owned compilation
using this same version-one report. Module source errors retain module filenames
and zero-based locations. Partial scanner success cannot mask registered source
errors. Context/overlay modes remain explicitly unavailable for registered graphs;
see the [CLI contract](registered-package-cli.md).
