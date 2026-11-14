# Experimental v2 source and package formatting

```sh
svr fmt --experimental-v2 main.svr
svr fmt --experimental-v2 --check main.svr
svr fmt --experimental-v2 --write main.svr
svr fmt --experimental-v2 --package --check package-directory
svr fmt --experimental-v2 --package --write package-directory
```

This opts into root enum/alias/record/function syntax with recursive match and
if/else-if/else/while bodies. Default formatting and its existing import/module
support are unchanged. V2 syntax still refuses imports and nested modules. Inline type modules and
[syntax-only module function bodies](type-modules-v2.md) are supported by formatting;
the program checker now supports the [qualified module-function subset](module-functions-v2.md).
`--locked` is not a formatter option. Use `--` before a
filename starting with a dash. Duplicate/conflicting options remain usage errors.

Formatting requires syntactic validity, not semantic success. Declaration-only
and empty sources, unresolved type names, duplicate declarations/variants, invalid
main signatures and other type errors can still be formatted when their syntax
parses. It never runs code, grants runtime services or requires Node at runtime.
Single-file mode does not read manifests or resolve dependencies. Package mode
resolves local manifests solely to select source files; neither mode reads locks. The same
source file can remain invalid when later checked by the compiler.

The shared syntax-only API `v2::parse_program_syntax(source, limits, max_functions)`
returns immutable `ParsedProgramSyntax`: root declaration ranges, enum headers,
versioned function bodies and core alias/record headers. Core headers are not an
old-AST projection of executable enum bodies. All coordinates refer to the
original supplied source. No functions/main are required for this parse stage;
checked source/program APIs retain their earlier semantic and missing-function
rules. Syntax parsing does not publish checked types, call links or executable IR.

## Style and safety gates

V2 formatting uses the existing four-space/LF layout engine with declaration and
statement boundaries supplied by v2 syntax. Variant and arm order, parentheses,
semicolons, literal spelling, namespace separators and token order are preserved.
Comments retain their content and token attachment. There is no style setting,
rewriting, insertion of missing syntax, symbol renaming or semantic simplification.

Before publication/replacement, the formatter verifies:

- identical token kinds and original token spelling/order;
- unchanged comment content and attachment to neighboring tokens;
- successful output parsing and structurally equal span-free v2 syntax;
- idempotent output from the same layout engine.

A failed gate returns no partial source and performs no replacement. Input remains
limited to one MiB and generated output to four MiB, with existing frontend,
structural-depth, enum and statement caps. The syntax profile admits at most 128
functions. Unlocated quota failures retain their code without a fabricated line/column.
These logical caps do not establish a physical-memory or hard-time bound.

Default mode writes formatted text to stdout and leaves the input unchanged.
`--check` writes no source and returns 1 when formatting is needed. `--write` uses
the existing guarded snapshot/atomic replacement path on qualified Windows/Linux
hosts; unchanged files preserve metadata. Links/reparse points, read-only targets,
source conflicts, unsafe replacements and unsupported write hosts retain their
existing refusals. Syntax/read/gate/write/publication failures return 1; usage
errors return 2. Stdout write failures are handled without a panic, but can leave
a partial OS write. Linux execution is verified here; exact other-host execution
and installed distribution acceptance remain separate gates.

Changing bytes with `--write` makes an existing source lock stale. Formatting does
not update or verify that lock. Explicitly regenerate it after reviewing edits
when using [locked v2 package commands](experimental-v2-locks.md).

Qualification includes the legacy real-program/import/package formatter corpus,
the new v2 parser/CLI cases and a relocated executable smoke test. That relocation
proves the formatter needs no checkout/runtime-source payload, not complete
installer acceptance. Opt-in [editor integration](experimental-v2-ide.md) is also
available. G13 remains partial; module/dependency compilation, executable backends
and broader platform qualification remain open.

## Package selection and guarded writes

Package mode requires explicit `--package` and `--check` or `--write`; it does not
concatenate source on stdout. The existing package formatter selects canonical
entries across the local dependency graph and deduplicates aliases. Registered
source inventories are selected by the existing inventory path. Only selected
files are formatted: unrelated files are not recursively discovered. Each selected
file must independently parse as supported v2 root/type-module syntax. Registered
files must pass the existing selection/inventory constraints and v2 syntax gates;
imports still refuse. Formatting module functions does not compile/link them.

Formatting a dependency graph does not link it or establish that v2 check/run
accepts that graph. V2 compilation still refuses dependencies and registered
modules. Declaration-only files and semantic errors remain formattable. This is a
syntax tool, not a second package compiler.

The shared package engine snapshots manifests, revalidates selection, checks all
sources and formats all output before publishing notices or replacements. It
stages writes, then rechecks source/manifest identities, bytes and stage validity.
A preflight failure leaves selected source files unchanged. Before each commit it
rechecks manifests. A later publication failure retains earlier replacements and
reports progress; this is not an all-or-nothing graph transaction or OS sandbox.
Temporary stages are cleaned up by the existing guarded writer.

Existing package limits apply: 128 selected sources, 16 MiB cumulative input
(including manifest snapshots) and 64 MiB cumulative output, with checked
arithmetic. Per-source v2 limits and token/comment/AST/fixed-point gates still
apply. Manifests and locks are never rewritten. A source edit can stale a lock;
review the formatting before explicitly regenerating it. Exact Linux execution
is qualified here; Windows package v2 execution remains a separate host gate.
