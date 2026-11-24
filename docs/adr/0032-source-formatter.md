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

## 2026-11-12: G10 Linux write qualification

`fmt --write` now enables the existing guarded replacement path on Linux as well
as Windows. Tests cover permission preservation, unchanged inode/timestamp,
read-only/link/special-mode refusal, staging cleanup and failure injection for
source identity, mode and parent changes. A test fixture now sets its initial
mode explicitly so restrictive process umasks cannot hide the injected change.
Existing token/comment/AST/fixed-point and interpreter/JavaScript preservation
checks remain active. Other hosts stay disabled. Imports and package-directory
formatting remain deferred; this is a partial G10 checkpoint. No Task Manager
features changed. Replacement remains a best-effort conflict check, without an
atomic cross-process lock or ACL/xattr preservation guarantee.

## 2026-11-12: G10 import-aware single-file formatting

The package compiler and formatter now share the existing token-level import
parser. No new grammar or resolver is introduced. Formatter output validates
ordered import alias/module identities alongside token spelling, comments,
normalized AST and fixed points. Unresolved imports require no manifest or
dependency reads; nested/malformed imports still fail. Regression coverage uses
comments inside import paths, duplicates, CRLF/Unicode, malformed syntax and
formatting both entries in a copied real package graph with interpreter/JS
behavior comparison. Guarded Windows/Linux writes remain available. Directory
formatting and macOS write qualification remain G10 work. No Task Manager
features changed. This supersedes the first-slice import rejection above.

## 2026-11-12: G10 read-only package graph formatting checks

Explicit `fmt --package --check DIR` now follows the existing compiler resolver
and deduplicates canonical entry files. It includes local dependencies outside
the root, without recursive scanning, execution or writes. A 128-package cap,
16 MiB cumulative manifest/source input cap and 64 MiB cumulative output cap
bound preparation. Manifest snapshots precede a second graph resolution; source
and manifest snapshots are rechecked before reporting any formatting notices.
Errors retain source ownership and emit no partial style report. This remains
best-effort disk conflict detection. Package stdout/write modes are usage errors.
Regression coverage includes shared aliases, nested entries, unrelated sources,
clean/dirty states, malformed provider syntax, missing dependencies, link policy,
no-write timestamps and graph/byte ceilings. CI runs the package formatter suite.
Next is multi-file write qualification; macOS writes remain deferred. No Task
Manager features changed.

## 2026-11-12: G10 staged package writes

Explicit `fmt --package --write DIR` now preflights and stages all canonical graph
entry outputs before any original replacement. Local dependencies outside the
root are included; manifests and unrelated sources stay unchanged. All snapshots
and staging identities/metadata/bytes are validated before publication; manifests
and the current source/staging are checked again per replacement. Staging reads
use retained handles. Linux metadata comparisons now include link counts and
Windows comparisons include file attributes. Existing single-file writes reuse
the same staging primitive.

Publication is atomic per file, not across the package: late failures stop,
report the changed-file completion count, retain earlier replacements and clean
unpublished staging files. No rollback or filesystem locking is claimed. The
existing 128-entry, cumulative input/output and per-file limits remain active.
Regression tests inject manifest/source/staging-content/staging-identity/link
changes before publication and external edits after the first replacement.
CLI tests cover valid writes, late-invalid-source preflight refusal, read-only
and hard-link policies, unchanged inode/timestamps and unchanged manifests.
Windows/Linux enable the command; new execution evidence is Linux-only. macOS
writes and cross-platform qualification remain G10 gates. Task Manager remains
on hold. This supersedes the earlier package-write deferral.

## 2026-11-12: G10 exact-host qualification reports

`scripts/qualify-formatter.mjs` now retains platform/architecture, compiler and
selected checkout hashes, version logs, formatter-related Rust regressions and
both real CLI formatter suites. It checks hashes again after execution and
records command statuses, timeouts/signals and log hashes. Missing/wrong compiler
regressions require failed reports and forbid qualification. Package tests accept
an explicit compiler path; unsupported hosts now explicitly verify standalone
write refusal as well as package write refusal.

Three-OS CI invokes the report runner and uploads reports/logs with `always()`.
Windows/Linux success means writes verified on that host. macOS success means
its disabled-write policy was verified, not that writes were enabled or qualified.
Only actual retained host reports can close execution gates; this change does
not claim remote Windows/macOS runs. Task Manager remains unchanged.

## 2026-11-12: formatter open-race hardening

Review before G11 found that pathname regular-file checks preceded a plain open:
a replacement FIFO could block before bounded reads began. Snapshot and conflict
reads now share an opened-handle check that rejects unexpected metadata before
reading. Linux x86/x86_64/aarch64 add nonblocking/no-follow flags; Windows opens
reparse points directly and rejects their handles. No dependency was added;
Linux flag values are explicitly limited to the stated architectures. Other
architectures retain regular-handle/metadata checks without those flags.

A deterministic Linux test replaces the already-statted source with a no-writer
FIFO, a link to the original and a different regular file. All are rejected and
original/external bytes remain intact. Existing best-effort parent/final-rename
race limitations remain. Windows execution and macOS write gates stay open.
Next: G11 structured application work under accepted ADR 0009. Task Manager
features remain unchanged.
