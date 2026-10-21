# ADR 0032: First Sovra source formatter

Status: **Accepted (2026-10-21).** The user approved the command, first-slice scope,
style, resource limits, safety gates and guarded write behavior. Implementation
and validation evidence are tracked in the development log.

## Inspection and scope

Before this change, the CLI labeled fmt planned. The compiler lexer discards whitespace and
`//` comments; AST nodes do not retain all punctuation or original literal spelling.
Printing only the AST would lose comments and can alter parentheses or optional
semicolons. A formatter must preserve source trivia and original non-whitespace
tokens, rather than introduce a new language grammar.

First slice: one existing UTF-8 `.svr` file in the executable standalone grammar,
including inline modules, typed functions, aliases, records, arrays and control
flow. No directory traversal, manifest formatting, stdin, imported `use` entries
or proposed application syntax yet. Package imports currently have a separate
preprocessing path; reject them explicitly until that syntax can be preserved and
validated through a shared import-aware parse path. Do not use partial project
scanner success as evidence that a file can be safely formatted.

## Approved public command

```text
svr fmt [--check | --write] [--] SOURCE.svr
```

- Default: emit formatted source to stdout without modifying the input.
- `--check`: no source output or writes; exit 0 for already formatted input and
  1 when changes are needed. Report the affected path with a short stderr notice.
- `--write`: replace that file only after all validation succeeds. Emit a short
  human status to stderr; do not mix status with source stdout. Unchanged files
  are not rewritten, preserving timestamps.
- Exactly one input; reject conflicting/repeated options, unknown flags, missing
  inputs, directory inputs and non-.svr paths with exit 2. `--` terminates options
  and allows filenames starting with a dash. Help alone exits 0; help combined
  with other arguments is a usage error. No JSON reporting contract in this slice.
- Source read/UTF-8/size errors, unsupported syntax, parse failures, failed safety
  checks and write failures exit 1 without formatted stdout or input modification.
  Existing compiler diagnostic codes/ranges are retained for lexer/parser errors.
  New formatter-specific failures use clear human messages, not invented E-codes.
- Accept syntactically valid source even if it has unresolved names or type errors.
  Do not execute source, resolve dependencies, load manifests or require Node/Rust.
- Bound source input to 1 MiB, also bounding generated output to 4 MiB. These are
  formatter-only limits, not new compiler or language limits. Existing nesting
  limits still apply. Reject a failed allocation/output budget without partial output.

## Approved fixed style

- Four spaces per indentation level; no tabs in generated indentation.
- LF line endings outside string literals, final newline, no trailing whitespace
  outside literal/comment content. Normalize comment line terminators to LF while
  preserving comment text. Preserve literal bytes accepted by the existing lexer;
  physical newlines inside string literals remain invalid, rather than adding syntax.
- Keep declaration order, token spelling, parentheses, semicolons, literal escapes,
  identifiers and comments. Do not sort, rename, simplify, insert missing syntax,
  remove imports, or repair invalid programs.
- Put function/module/control blocks and record field declarations on indented
  lines, with closing braces aligned to their opener. Keep `} else {` together
  when no intervening comment requires its own line. Use a space before a block.
- Space binary/assignment operators; space after commas and type colons. Do not
  space namespace separators, field-access dots or the inside of ordinary paired
  call/index/group delimiters. Preserve lexical separation when joining tokens
  would form another token, numeric literal or comment.
- Preserve standalone comments on their own lines and trailing comments beside
  their preceding construct. Never move code into a line comment. Reduce repeated
  blank separator lines to one without reflowing comment text or literal content.
- No configurable style, hard line-width limit or literal/comment reflow initially.
  Long expressions may remain long. Any future canonical-style revision needs a
  documented compatibility decision; do not silently reformat installed projects.

Example:

```svr
fn main(){let title="Keep // text";print(title);}
```

becomes:

```svr
fn main() {
    let title = "Keep // text";
    print(title);
}
```

This style is an approved tool convention, not a language syntax restriction.

## Architecture and safety gates

Implement a private Rust formatter module called by the thin CLI. Keep public
lexer/parser/compiler APIs unchanged. A lossless source view combines lexer byte
spans with retained gaps/comments; original token slices preserve literal spelling.
Classify brace/statement contexts from validated executable syntax rather than
assuming every brace or semicolon has the same layout role. In particular, handle
record construction separately from blocks and retain condition parentheses.

Before publishing output, require:

1. Successful input lexing/parsing in the supported grammar.
2. Successful output lexing/parsing with identical ordered token kinds and raw
   non-whitespace token spellings, excluding EOF spans. Exact string-literal bytes
   are retained; comments stay in their original order and attachment category.
3. Equivalent AST structure ignoring source ranges, plus retained comment content.
4. Idempotence: formatting the output again produces identical bytes.

Perform all gates in every mode, including stdout and check. If a construct cannot
be represented safely, reject the file instead of claiming it is already formatted.
Use ordinary token/parse rules; no Task Manager-specific recognition or patches.

## Approved write behavior

Refuse symlink/reparse-point inputs and non-regular files in all modes, with an
explicit error. Read one bounded snapshot. For write mode, stage a uniquely owned
file beside the destination, preserve ordinary file permissions, and flush/close
before replacement. Recheck source identity and bytes immediately before replacing;
if changed, leave the current file untouched and report concurrent modification.

Use an atomic replacement mechanism that preserves the old file on failure; never
delete the original first. Do not reuse a helper blindly: the existing text-file
API has its own reviewed contracts and must remain unchanged. Qualify replacement
and permissions on Windows and Unix before enabling write there. Refuse read-only
files rather than clearing their attributes. Preserve unrelated files and clean
up only owned staging files. No backups, recursive writes or configuration changes.

The recheck is a best-effort conflict guard, not a filesystem compare-and-swap:
an external writer can still race the final atomic replacement. Document that
limit; do not claim a cross-process lock or exact preservation of ACLs, alternate
streams, extended attributes or hard-link identity. Unsupported preservation cases
must fail clearly rather than fall back to truncating writes.

## Implementation milestones after approval

1. Retained-trivia source model and pure formatting engine; test strings containing
   comment markers, Unicode, escapes, CRLF and embedded newlines. Cover trailing
   and standalone comments, optional semicolons, nested records, arrays and branches.
2. Token/AST equivalence and idempotence gates; test invalid, unsupported and bounded
   inputs. Test that valid-but-type-invalid source formats without being executed.
3. CLI stdout/check modes with exact output/exit tests and `--` filenames; format
   Task Manager and unrelated executable examples in temporary copies, then compare
   interpreter/JavaScript behavior. Do not rewrite repository applications wholesale.
4. Safe write helper and platform tests: unchanged timestamp, permissions/read-only,
   links/reparse points, concurrent-edit detection, failed replacement, staging
   cleanup and original-byte preservation on every failed validation.
5. Installed Windows acceptance without Node/Rust on PATH, documentation and
   deterministic CI check fixtures. Keep existing compiler/dogfood suites intact;
   do not enable repository-wide formatting enforcement before a reviewed migration.

## Approval and implementation boundary

The user approved this contract on 2026-10-21. No language syntax, type semantics,
memory model or runtime API changed. Stdout/check modes and Windows writes are
implemented; non-Windows writes remain disabled pending execution qualification.
The retained Unix helper tests are not evidence that Unix replacement was tested
locally. Package imports/application syntax remain deferred. See the development
log for actual validation evidence and the reference for current limitations.
