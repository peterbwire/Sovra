# JSON check reports

## Executable local packages

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
initializers and unsupported annotations remain unresolved. Supported primitive
binary arithmetic/comparison expressions contribute argument and initializer types.
E4121 reports incompatible known operands at the expression range. Resolved service
call results contribute types when receiver, arity and all argument types validate;
invalid or unknown arguments prevent result propagation. E4122 reports known
service return mismatches at the expression range (the keyword for bare returns).
E4123 reports a non-Unit service body that can fall through at the full function
range. Both if/else arms must return; while loops alone never guarantee return.
E4124 identifies known non-Bool if/while conditions at their expression ranges.
Unknown condition/return types and ordinary call results remain
unchecked. Typed signature
records are available through the Rust API, not additional JSON fields.
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

Source locations use byte offsets `[start, end)` and zero-based line and
Unicode-character column indices. Columns are not UTF-16/LSP offsets.
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
