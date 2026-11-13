# Source formatter

Implemented under accepted [ADR 0032](../adr/0032-source-formatter.md):

```text
svr fmt [--check | --write] [--] SOURCE.svr
svr fmt --package [--check | --write] [--] DIRECTORY
```

Default mode prints formatted source without modifying the input. `--check`
prints no source and exits 1 if formatting is needed, otherwise 0. `--write`
replaces changed files and reports paths on stderr; unchanged timestamps remain.
Write mode is qualified and enabled on Windows and Linux. Other hosts must use
stdout or check mode until replacement behavior is qualified there. Usage errors exit 2;
source, safety and filesystem failures exit 1. Help alone exits 0. Use `--` before
a filename beginning with a dash.

Accepts one existing UTF-8 `.svr` regular file in executable syntax,
including inline modules and top-level `use dependency::module;` imports.
Single-file mode rejects directories, manifests and application syntax. It does
not resolve dependencies or require Node/Rust. Package mode resolves local graph
metadata only; neither mode executes source. Syntactically valid source with type/name errors can
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
Repository-wide enforcement remains future work.

The repository depth probe also exercises formatting at and above supported
nesting limits. Successful results must pass `fmt --check`; rejected inputs must
retain their bytes and emit no partial source. CI retains results with the tested
compiler hash. This does not substitute for clean-machine/platform qualification.

Linux write qualification covers mode preservation, unchanged inode/timestamp,
read-only/hard-link/symlink/special-mode refusal, staging cleanup, failed rename,
and source identity/mode/parent changes before publication. It does not preserve
ACLs or extended attributes, guarantee power-loss durability of directory entries,
or lock out a writer racing the final rename. macOS write qualification remains G10 work.

## Import-aware single-file formatting

Imports use the same token parser as executable package compilation. Formatting
compares import alias/module order in addition to normalized executable ASTs,
exact token spelling/order and comment attachment. Import spans retain original
source coordinates for diagnostics; comparison ignores only their positions.
Malformed imports retain compiler diagnostics, including E4114. Nested imports
remain invalid. No import sorting, deduplication, resolution or dependency writes
occur. Unresolved imports and semantically invalid programs can still format.
Each import ends a line unless comments require additional lines. Existing
Windows/Linux write safeguards and source/output quotas apply unchanged.

## Package graph checks and writes

`svr fmt --package --check DIRECTORY` resolves the compiler-owned local dependency
graph and checks each distinct canonical source file, in canonical path order.
Graphs with `[modules]` use the compiler inventory, including all registered module
files in every declared dependency. Entry-only graphs retain their earlier resolver.
This includes dependencies outside the requested root. It does not recursively
scan directories or format manifests. Missing dependencies, cycles and malformed
manifests retain resolver diagnostics. Formatting is a syntax/style check, not
source type checking or proof that a package runs.

The formatter admits at most 128 selected sources, 16 MiB cumulative UTF-8 input bytes
(including manifest snapshots) and 64 MiB cumulative formatted bytes, in addition
to the existing 1 MiB source/manifest and 4 MiB per-output limits. Manifest
locators must be regular non-link files; source entries are the canonical paths
selected by the resolver. Graph selection is resolved again after manifest
snapshots, then all manifest/source snapshots are rechecked before any formatting
notice is printed. This is best-effort conflict detection, not an atomic disk
snapshot or a cross-process lock.

Exit 0 means all entries are formatted; exit 1 means formatting is needed or a
resolver/read/parse/safety/budget/conflict check failed. Stdout stays empty. Only
valid complete checks print `Formatting needed:` notices; failures identify their
source where available. Check mode never modifies input bytes or timestamps. Package stdout mode is
rejected with usage exit 2. Explicit package writes are supported as below.

### Guarded package writes

`svr fmt --package --write DIRECTORY` formats the same canonical entries,
**including local dependencies outside the root**. Windows/Linux enable writes;
other hosts reject them. The formatter never writes manifests or unrelated files.
All selected entries, including unchanged ones, must satisfy write eligibility.

Every manifest/source is read and all formatting, quotas and permission checks
finish before originals change. Changed outputs are staged beside each source,
flushed and synced. Staging uses at most one handle per changed entry (128 maximum).
All input snapshots and staged identities/metadata/bytes are checked before the
first publication. Manifests are rechecked before each replacement, and each
source and its staging are checked again just before its atomic rename. Staging
content checks use retained handles and compare the exact validated output.
Linux metadata checks include link counts; Windows checks include file attributes.

**The package operation is not atomic across files.** Preparation failures leave
all original sources unchanged and clean owned staging files. A failure during
publication stops further writes, reports the number of changed files already
published, preserves those replacements and cleans unpublished staging files.
It does not roll back earlier replacements or overwrite a detected external edit.
Successful per-file statuses go to stderr; stdout stays empty. Unchanged sources
retain their inode and modification time. All earlier conflict-race, ACL/xattr,
alternate-stream and crash-durability limitations still apply.

## Reproducible host evidence

After building the compiler from the checkout, run:

```sh
cargo build --locked
node scripts/qualify-formatter.mjs
```

The runner performs formatter-related Rust regressions, the standalone/import
execution-preservation suite and package check/write tests. It records host/CPU,
OS release, Node, compiler and Rust version logs, compiler SHA-256, selected
source/script hashes and each command's status/log hash. Source and compiler
hashes are rechecked afterward. Optional `--compiler PATH` selects a compiler;
`--output REPORT.json` selects an evidence path. The default report is
`target/formatter/<platform>-<architecture>-verification.json`, with sibling
`.logs` directory. Build the selected compiler beforehand; the harness does not
rebuild it. Selected checkout hashes describe the source context of the run.

A missing compiler, failed command, signal, timeout or changed input yields a
failed report and exit 1. There is no skipped-execution success path. Regression
tests deliberately select missing and unrelated binaries to verify this rule.
Successful Windows/Linux runs record `verified_on_this_host`; other hosts record
`disabled_policy_verified`, explicitly confirming write refusal rather than
qualifying writes. Package checks and syntax/output preservation still run there.
Invalid runner options exit 2.

CI uses this runner on Linux, Windows and macOS and uploads `target/formatter/`
with `always()` after the qualification step. A configured job is not execution
evidence: retain and inspect its actual report before closing a platform gate.
The report qualifies the stated formatter behavior for its exact host/compiler;
it does not qualify a release, MSRV, desktop host or language security claim.

## Opened-handle validation

Source/manifest snapshot reads and replacement conflict reads validate the opened
handle against the expected regular-file metadata before reading any bytes. On
Linux x86, x86_64 and aarch64, reads add `O_NONBLOCK | O_NOFOLLOW`: swapping a
regular file for a no-writer FIFO cannot block open, and final-component symbolic
links are refused. Windows opens with `FILE_FLAG_OPEN_REPARSE_POINT` and rejects
reparse handles. Other architectures retain metadata checks without the Linux
flags. This does not lock parent directories, establish atomic filesystem
snapshots or guarantee bounded latency for every filesystem/device operation.
A deterministic Linux regression substitutes a FIFO, symlink and different
regular file after the original metadata check, and verifies rejection while
preserving original/external bytes.

Registered graph selection retains inventory metadata through the second selection
check. Existing manifest/source/staging conflict checks and per-file sequential
publication apply to every selected source. This remains syntax formatting, without
semantic linking or lock enforcement; `sovra.lock` is never rewritten. Changed bytes
make a previously generated lock stale. Registered formatting has Linux regression
evidence; other-host execution/qualification remains separate, and unsupported
inventory identity checks fail explicitly.
