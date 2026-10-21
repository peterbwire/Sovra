# Source formatter

Implemented under accepted [ADR 0032](../adr/0032-source-formatter.md):

```text
svr fmt [--check | --write] [--] SOURCE.svr
```

Default mode prints formatted source without modifying the input. `--check`
prints no source and exits 1 if formatting is needed, otherwise 0. `--write`
replaces changed files and reports paths on stderr; unchanged timestamps remain.
Write mode is qualified and enabled on Windows only. Other hosts must use stdout
or check mode until replacement behavior is qualified there. Usage errors exit 2;
source, safety and filesystem failures exit 1. Help alone exits 0. Use `--` before
a filename beginning with a dash.

Accepts one existing UTF-8 `.svr` regular file in executable standalone syntax,
including inline modules. Imported `use` entries, directories, manifests and
application syntax are unsupported. No execution, dependency resolution or Node/
Rust installation is needed. Syntactically valid source with type/name errors can
be formatted. Input is limited to 1 MiB and output to 4 MiB; compiler nesting limits
also apply. Failures produce no partial formatted source.

The fixed style uses four spaces, LF endings, a final newline and spaced operators/
type colons. Braces use indented lines, including record fields; calls and arrays
remain compact. Comments retain their text, order and attachment; literal spelling,
optional semicolons, parentheses and other tokens remain. Blank separator runs
reduce to one. No wrapping, configurable style or syntax repair is provided. Token
spelling, comment attachment, AST equivalence ignoring ranges, and a second format
pass must agree before output is used.

All modes reject file symlinks/reparse points. Windows writes reject read-only or
special-attribute files. Replacements are staged beside the file, flushed and
closed; the original snapshot and parent are rechecked before atomic rename.
Failure preserves the original and cleans owned staging files. The conflict check
is best effort, not a cross-process lock: another writer can race final replacement.
After successful publication, staging cleanup is disarmed so a reused staging
pathname is not deleted by the completed operation.
Exact ACLs, alternate streams, extended attributes and hard-link identity are not
guaranteed. This is a source formatter, not a backup tool.

CI formats temporary copies of real applications and compares interpreter and
generated JavaScript behavior. Repository sources are not automatically rewritten.
Package formatting and repository-wide enforcement remain future work.

The repository depth probe also exercises formatting at and above supported
nesting limits. Successful results must pass `fmt --check`; rejected inputs must
retain their bytes and emit no partial source. CI retains results with the tested
compiler hash. This does not substitute for clean-machine/platform qualification.
