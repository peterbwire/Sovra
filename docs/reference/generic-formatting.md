# Generic and data source formatting

Select the generic syntax profile explicitly:

```sh
svr fmt --experimental-generics source.svr
svr fmt --experimental-generics --check source.svr
svr fmt --experimental-generics --write source.svr
svr fmt --experimental-generics --package --write ./app
svr fmt --experimental-generics --package --check ./app
```

This profile supports admitted generic aliases/records/enums/functions, standard
Option/Result/Error and data type annotations, explicit generic calls, match
bodies and postfix `?`. It also formats leading `use dependency::module;` imports
and registered module wrappers. It reuses the compiler's generic declaration and
propagation parsers. Legacy formatting and `--experimental-v2` keep their existing
grammars. Duplicate or conflicting profile flags are usage errors; `--` terminates
option parsing. No additional `--experimental-data` flag is needed for formatting.

Formatting checks syntax only. It does not resolve source names, type-check
expressions, execute source, acquire host grants or require `main`. Single-file
formatting does not load surrounding manifests. Package formatting follows the
existing local graph/source inventory, preflights every selected source and uses
guarded staged publication. A malformed later source prevents earlier source
publication. Successful earlier replacements can remain if a later filesystem
commit fails; package publication is not a multi-file atomic transaction.

Before publishing, the formatter verifies exact token kind/spelling/order and
comment text/attachment, reparses output, compares bounded normalized body ASTs
and token-index overlays for generic calls/matches/postfix sites, and requires
another formatting pass to produce identical output. Declarations are parsed on
both passes; their exact tokens are preserved. Trees are depth-checked before
recursive comparison and discarded iteratively on refusal.

The existing [formatter](formatter.md) limits apply: 1 MiB input, 4 MiB output,
structural depth 128, regular files and guarded writes on Windows/Linux. Generic
declaration/body/parser quotas additionally apply, including 128 KiB per
declaration and 16,384 body tokens. Package limits remain 128 selected sources,
16 MiB cumulative input and 64 MiB cumulative output. Comments, literal spelling
and source tokens are preserved; this tool does not repair syntax or provide a
hard memory/CPU sandbox.

In VS Code, enable `sovra.experimentalGenerics` and restart the Sovra language
server. Document formatting uses unsaved buffer text through the real compiler
and returns edits. It does not apply edits or write workspace files itself.
Existing workspace trust, version checks, cancellation, timeout and temporary-file
cleanup still apply. Stale requests cannot publish edits for a newer buffer.
The CLI can format data syntax even where editor data semantic checking remains
unsupported; formatting does not enable data execution or host authority in LSP.

Format before generating a source lock:

```sh
svr fmt --experimental-generics --package --write ./app
svr lock --experimental-generics ./app
svr check --experimental-generics --experimental-data --locked ./app
svr run --experimental-generics --experimental-data --locked ./app
```

Formatting never updates `sovra.lock`. A changed source requires an explicitly
regenerated lock; formatting an already formatted package preserves the lock.
Integration tests exercise locked data and dependency-module consumers, identical
execution before/after formatting, malformed/deep input refusal, no partial
stdout, all-source preflight, symlink refusal and LSP stale-request retirement.
The native data consumer is formatted before locked build and still launches
after its source directory is removed with Node/PATH unavailable.

Actual new VS Code Electron host acceptance and Windows/ARM runtime qualification
require their own host runs; passing Linux CLI/LSP tests is not that evidence.
