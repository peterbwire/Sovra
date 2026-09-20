# JSON check reports

Status: Implemented report format for the current source checker and partial
project checker. Use `svr check --format json <source.svr|project-directory>`.
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
empty diagnostics array. Project statistics and symbols are not in version 1.
Rust diagnostics carry optional `source_file` provenance; stage function
signatures and the version-one JSON envelope remain unchanged. Serialization
lives in `src/compiler/check_report.rs`. Related-location notes and richer
diagnostic rendering remain separate work.
