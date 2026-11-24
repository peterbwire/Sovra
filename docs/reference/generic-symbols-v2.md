# Checked generic declaration symbols (V2)

Successful standalone generic JSON checks can opt into compiler-owned declarations:

```sh
svr check --experimental-generics --format json --symbols main.svr
```

`--symbols` requires standalone JSON check, appears at most once, and is unavailable
for package directories, Run or Build. Plain checks retain their previous reports.
Failed compilation emits diagnostics without symbol facts. This does not execute
source or change the native feature contract.

`generic_source_symbols` projects a sealed `CompiledGenericSource` into the existing
bounded symbol envelope. It reports original source declarations only: functions,
records, enums and aliases. Injected compiler-owned definitions are omitted.
Kinds, whole declaration ranges, exact name ranges and checked source headers come
from retained syntax owners after complete semantic checking. Function signatures
exclude bodies; record/enum headers end at their parameter lists. Alias headers
retain their checked target text. No token/name search infers bindings. [Checked direct call references](generic-call-navigation-v2.md)
now link retained callee tokens to bound function declarations.

`GenericSymbolLimits` independently admits declaration count, names/headers,
JSON bytes and work. Hard ceilings are 8192 declarations, 4096 bytes per string
and 512 KiB of evidence. Escaped JSON bytes are admitted before emission, including
control characters. Quota or provenance refusal returns only a fixed empty
`truncated:true, unavailable:true` envelope. Compilation still succeeds; no partial
facts or truncated Unicode strings are published.

Generic editor mode requests this evidence and enables declaration outlines and
hover at declaration names, with UTF-16 mapping through original Unicode/CRLF.
Direct user-function call references now support Go to Definition and declared
signature hover. Parameters and locals now support checked definition/type hover; user function-header type references are available. Invalid edits and unavailable optional evidence
clear previous facts rather than retaining stale navigation. Formatting, package
overlays and IR/JS editor tasks retain their existing gates.

Tests cover compiler projection limits, source ownership, control escaping,
optional failure preserving execution, CLI opt-in/flag refusal, existing strict
client decoding, unsaved LSP generation replacement and actual VS Code declaration
outlines/hover. Other-host editor execution and packaged VSIX installation remain
separate gates. Broader semantic navigation, modules/dependencies/JavaScript and
remaining E2/G13 requirements remain open. Task Manager is unchanged.

See [checked generic local navigation](generic-local-navigation-v2.md) for parameter/local/payload definitions and checked type hover.

See [checked function-header type navigation](generic-type-navigation-v2.md) for record/enum/alias annotation definitions and hover; other type contexts remain open.
