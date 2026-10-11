# ADR 0022: Terminal-safe display of untrusted CLI text

Status: Proposed; compatibility decision awaiting approval.

## Problem

The Task Manager accepts a title containing U+001B (ESC), stores it, and prints
it unchanged in `list`. A title such as `Safe` + ESC + `[2J` + `Spoofed` emits a
terminal clear-screen sequence. The same raw-output path exists for missing-title
errors and unknown interactive commands. This is reproducible with the current
interpreter and is relevant to generated JavaScript as well.

Line-break rejection does not cover these controls. A persisted file may contain
such a title even if new input is later restricted, so input checks alone cannot
make all output safe. This ADR does not change the text-file format or Unicode
identity of stored titles.

## Recommended decision

Add a general `std::escape_terminal(text: String) -> String` function. It replaces
Unicode scalars U+0000 through U+001F and U+007F through U+009F with printable
ASCII `\u{XXXX}` notation, using uppercase hexadecimal digits and at least four
digits. All other scalars, including ordinary Unicode text and backslashes, pass
through unchanged. This is a display transformation, not a reversible encoding.
For example, ESC becomes the printable text `\u{001B}`.

Use that function for every Task Manager field sourced from a title or command
before printing it. Reject newly added titles containing one of those controls
with a static error message and nonzero status. Continue to decode existing
version-one files containing control characters; show their titles escaped and
leave their stored bytes intact until a normal edit. Exact title lookup remains
based on the original title, not its display form. Existing LF/CR title errors
retain their current message.

This changes Task Manager acceptance and display for control-bearing text. It
does not change Sovra string values, string equality, Unicode normalization,
`print`, or the file-I/O APIs. The standard function is useful to other CLI
programs and must have identical interpreter and generated-JavaScript behavior.

## Alternatives

- Reject control characters while loading a task file. This would make existing
  files unreadable instead of allowing users to inspect or repair their tasks.
- Escape only when stdout is a terminal. That requires a new terminal-detection
  contract and leaves redirected output unsafe for terminal viewers downstream.
- Continue raw output and document the risk. This is unsuitable for a
  production-quality Task Manager CLI.

## Acceptance

- A pre-fix regression demonstrates that ESC in a title reaches `list` output.
- Valid and invalid standard-call tests cover argument count and type; execution
  tests cover C0, DEL, C1, ordinary Unicode, and backslashes in both engines.
- Task Manager tests cover direct and interactive new-title rejection, safe
  listing of an older control-bearing file, missing-title and unknown-command
  errors, and preservation of stored bytes on read-only commands.
- Existing Task Manager transcripts, persistence migration, and package tests
  remain passing. Document the display rule and its compatibility effect.

## Approval boundary

AGENTS.md requires documenting and pausing before fundamental compatibility
decisions. Rejecting previously accepted title input and changing output for
stored titles are observable application contracts. This proposal needs user
approval before implementation.
