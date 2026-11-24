# Checked String utility calls (V2)

`compile_generic_string_source` extends the host-free builtin profile with three
exact qualified calls. Experimental generic CLI commands select this profile.

| Call | Argument | Result and existing behavior |
| --- | --- | --- |
| `std::len` | String | Int UTF-8 byte length, including controls and zero bytes |
| `std::lines_unique` | String | Bool exact LF-delimited line uniqueness |
| `std::escape_terminal` | String | String display escaping of C0, DEL and C1 controls |

Length counts bytes, not Unicode scalar values or graphemes. Line comparison does
not normalize case, Unicode or carriage returns. A terminal LF does not add a final
empty line; empty input is unique. Terminal escaping reuses the core formatter:
controls become uppercase `\u{XXXX}` text, while Unicode and existing backslashes
remain unchanged. It is display escaping, not a general serialization codec.
Source literal syntax is unchanged; this does not add Unicode escape syntax.

All calls require one String and no type arguments. Every symbolic body, including
unused functions, is checked under shared call/argument/work admission. Concrete
rechecking validates arguments/results; IR independently validates primitive result
layouts and typed stack consumption. Primitive result calls do not invent nominal
records or specialized function instances.

The VM shares instruction, work, materialization and cooperative control policy
with every other operation. Byte length uses checked Int conversion. Line checking
charges a scan before splitting, then conservative comparison work before each
set lookup/insertion. The ordered set borrows the input's lines; each retained
entry consumes a temporary logical node. Temporary storage is never refunded.
This is deterministic logical admission, not a process-memory/time sandbox.

Escaping charges input scanning, computes exact output bytes with checked
arithmetic, then admits output work and one String node/text allowance before
formatting. Checkpoints surround scans and copying. Exhaustion returns no partial
outcome, with original call provenance. Tests cover exact budget boundaries,
primitive result/return corruption and old-profile refusal.

Native feature word **127** repeats this profile's bounded checking at startup.
Words 1/3/7/15/31/63 retain previous environments; only those seven exact words
are accepted. Downgrading a String utility consumer refuses. The updated
[consumer example](../examples/generic-result-adapters.svr) exercises these calls in
a real locked Linux native build run without project inputs or tool PATH.
The earlier `compile_generic_builtin_source` API stays limited to parsing/splitting.

Polymorphic `std::to_string`, array builtin bridging, host I/O and remaining E2/G13
requirements remain open. Installed-payload and other-host execution are separate
qualification gates. Task Manager is unchanged.

Value utility update: [display and array length](generic-value-builtins-v2.md)
now extend experimental commands and native feature 255. Earlier APIs/image
environments retain their previous scope; host I/O remains gated.
