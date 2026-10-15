# `svr test --format json`

Accepted ADR 0028 introduces a separate test report schema, without changing
`svr check --format json`. Valid invocations emit one JSON object and a newline
on stdout. Test print output is captured inside the report; runtime failures
do not add a second stdout document. Usage errors return 2 on stderr.

| Field | Meaning |
| --- | --- |
| `schema_version` | Integer 1 |
| `target` | Input as supplied |
| `kind` | `source` or `package`; package tests use full executable linking |
| `success` | A nonempty passing suite without error diagnostics |
| `passed`, `failed`, `total` | Executed test counts; configuration errors execute zero tests |
| `diagnostics` | Compilation/configuration diagnostics preserving code, severity, message, source identity and span |
| `tests` | Execution outcomes in root source order |

Each test has `name`, `passed`, `output` (array of captured lines), `exit_code`
and `error`. An ordinary return has a numeric exit code and null error; a runtime
failure has null exit code and an error string. Output before a failure is retained.
Nonzero ordinary status fails without inventing a runtime error.

Each diagnostic has `severity`, `code`, `message`, `source_file` and `span`.
Unknown file identity is null. Spans contain zero-based `start`, `end`, `line`
and `column`; end is exclusive. Unknown locations are null, not fabricated.
Runtime errors currently have no source stack traces; do not infer their location.

`E5001` means a discovered function has parameters or a non-Unit return type.
`E5002` means no root-level `test_` function was found. Existing parser/semantic/
package codes retain their meaning. Exit 1 accompanies compilation/configuration
failures or any failed test. No discovered tests is a failure, even though the
executed passed/failed counters are both zero. See [the lesson](../course/testing.md)
for execution/effect limits and conventions.
