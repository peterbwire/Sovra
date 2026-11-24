# Standalone generic commands

The explicit `--experimental-generics` mode connects the bounded
[generic source pipeline](generic-source-v2.md) to the compiler executable:

```sh
svr check --experimental-generics docs/examples/generic-source.svr
svr check --experimental-generics --format json docs/examples/generic-source.svr
svr run --experimental-generics docs/examples/generic-source.svr
```

Run these from a repository checkout with a current `svr`, or prefix each command
with `cargo run --` instead of `svr`. The [example](../examples/generic-source.svr)
uses generic records, enums and functions with an ordinary helper and exhaustive
matching. It exits successfully without output after verifying its result. A
subprocess regression includes this exact source.

`check` compiles the entire admitted standalone file, including unused bodies,
and builds checked generic IR without executing it. It requires a parameterless,
nongeneric `main`; its return type may be any supported type. `run` additionally
requires a Unit return type before execution and returns process status zero
only after execution succeeds. Generic calls require explicit arguments.

Execution uses the host-free Rust VM with its finite default profile:
1,000,000 instructions, 1,000,000 logical materialized nodes, 16 MiB logical text,
256 active frames, 65,536 active slots and operands, 16,000,000 work units and
zero output bytes. The CLI additionally selects a 30-second cooperative elapsed
limit. These are logical/cooperative controls, not process isolation or a hard
CPU/heap deadline. The CLI reads at most 4 MiB of UTF-8 source, under cumulative
source/declaration and semantic-stage limits. No process stdin, output, files,
network, arguments or permission grants are provided to Sovra code in this mode.

Both commands accept one regular standalone `.svr` input or a
[guarded single-entry root package](generic-packages-v2.md). Package-only
`--locked` verifies the existing source lock before compilation or effects. `check` accepts one
optional `--format human|json`. Successful standalone JSON checks may request
[checked declaration evidence](generic-symbols-v2.md) with `--symbols`. `--` terminates option parsing, after which one
input pathname may be supplied; `run` has no program-argument transport.
Duplicate flags, incompatible engine modes, permissions,
extra inputs and unsupported flags refuse with status 2. Unsupported package
shapes refuse under existing package diagnostics.
[Native executable builds](generic-native-v2.md) now support
`build --experimental-generics --emit exe --output PATH SOURCE.svr`.
Test/dependency-linking/JavaScript integration remains unavailable for this mode. Default and existing `--experimental-v2` command paths remain separate.

Status 1 means input, compilation, entry or runtime failure; status 0 means the
requested operation succeeded. Human check success goes to stdout; diagnostics
go to stderr. JSON check uses the existing version-one report on stdout, with
no human diagnostic on stderr. I/O errors use E0001 or existing source-read
resource codes. E3123 identifies generic compilation or entry refusal; E3125
identifies generic execution refusal. These umbrella codes retain bounded stage
detail in their message. Primary spans use original-file bytes and zero-based
line/character columns, when known; global refusals have null JSON locations.
Runtime human locations identify the original file and operation. No partial
successful result or fallback is published after failure.

Tests in `tests/generic_cli_v2.rs` cover real subprocess checks/execution, the
example, mapped diagnostics, JSON, finite-loop refusal, entry contracts, UTF-8
and byte limits, unavailable hosts, strict flags, delimiters, input kinds and
unchanged pipeline selection. This is a Linux qualification slice; G13 E2 still
requires dependency/module linking, JavaScript integration, broader editor evidence and consolidated qualification.

Current propagation integration: [experimental command/native support](generic-propagation-cli-v2.md)
now enables the explicit source profile under `--experimental-generics` and native
feature word 15. Historical profiles 1/3/7 and ordinary source APIs remain strict.

Adapter environment update: [explicit standard result adapters](generic-result-adapters-v2.md)
now extend experimental commands and select native feature word 31. Historical
words 1/3/7/15 retain their previous environments. Host builtin calls remain gated.

Host-free builtin update: [checked parsing and splitting](generic-builtin-calls-v2.md)
now use the explicit builtin source profile and experimental CLI/native feature 63.
Schema-only profile 31 stays strict; other host/builtin operations remain gated.

String utility update: [checked String calls](generic-string-builtins-v2.md)
now extend experimental commands and native feature 127. Earlier source APIs and
image words retain their previous environments; host I/O remains gated.

Value utility update: [display and array length](generic-value-builtins-v2.md)
now extend experimental commands and native feature 255. Earlier APIs/image
environments retain their previous scope; host I/O remains gated.

Array operation update: [growth and slicing](generic-array-builtins-v2.md)
now extend experimental commands and native feature 511. Earlier APIs/image
environments retain their previous scope; host I/O remains gated.

[Generic editor mode](generic-ide-v2.md) now provides opt-in standalone buffer
diagnostics and Check/Run tasks; generic navigation/formatting and package targeting
remain open.
