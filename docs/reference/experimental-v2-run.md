# Experimental root-program execution

```sh
svr run --experimental-v2 main.svr
svr run --experimental-v2 --allow-stdin main.svr
svr run --experimental-v2 main.svr -- first-argument --help
```

This explicitly selects the linked v2 Rust VM for a regular standalone `.svr`
source or [single-entry package](experimental-v2-packages.md). The bounded source
reader, root checker and all-function lowering run
before execution. A parameterless Unit `main` is required. Root enums, records,
aliases, linked functions and recursive match/if/while bodies use the existing
[checked-program semantics](program-execution-v2.md). Imports, dependency packages, registered modules remain
unsupported. Default `run`, `check`, `build` and v1 stage APIs are unchanged.

Put the source before `--`; everything after it is a program argument, including
`--help` and `--experimental-v2`. Dash-prefixed source paths can use `./`.
`std::arg_count`, `std::arg` and `std::set_exit_code` share one command context
across calls. A successful program returns its requested portable status (0–125),
initially zero. Source/read/lowering/runtime/output failures return 1; unsupported
options and invalid usage return 2. Failure status 1 can therefore also be an
explicit successful program status. There is no JSON execution mode.

No stdin or filesystem host is granted by default. The independent
[explicit host flags](experimental-v2-hosts.md) grant existing bounded process
services. Calls requiring an absent capability refuse
in preflight, including calls in unused functions. `print`/`println` capture raw
program text in memory. The CLI writes that output only after execution succeeds,
with one LF per captured line, without escaping program text. Runtime failure
publishes no captured output or requested status. Publication can partially write
before an OS output failure; write/flush failures return 1 without a panic.

## Fixed finite profile

| Allowance | Limit |
| --- | --- |
| VM instructions | 1,000,000 |
| Captured output UTF-8 bytes, including LF | 1,048,576 |
| Cumulative model-v2 value nodes | 1,000,000 |
| Cumulative model-v2 text bytes | 16,777,216 |
| Active call frames | 256 |
| Cooperative elapsed execution | 30 seconds |

The command uses `ProgramSourceLimits::DEFAULT`, `ProgramLoweringLimits::DEFAULT`
and the existing default IR-validation limits. These finite compiler allowances
are separate from runtime fuel. There are no CLI overrides or unlimited fallback.
Library `RuntimeVmPolicy::DEFAULT` is unchanged and still requires callers to
choose their own optional component limits.

Logical fuel measures the documented operations, not total CPU work, allocations
or resident memory. The elapsed clock begins after graph lowering; cooperative
checkpoints cannot interrupt compiler work, helper traversals or cleanup. This
profile is not a hard deadline or an OS sandbox. Output remains ordinary program
text, so consumers decide whether to render control characters.

G13 remains partial. Multi-source packages, confined filesystem/new host policies, IDE selection,
JavaScript/Worker/native v2 backends and installed cross-platform qualification
remain open. [Checking](experimental-v2-check.md) grants no execution authority.

For root test_ discovery without requiring main, use the separate
[experimental test mode](experimental-v2-test.md).

For supported package directories, optional `--locked` verifies the captured
inputs before compilation or execution. See [v2 locks](experimental-v2-locks.md).
