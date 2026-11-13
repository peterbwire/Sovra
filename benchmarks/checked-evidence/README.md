# Checked-evidence qualification corpus, version 3

This opt-in probe compares legacy and checked compilation without changing
compiler defaults. It never executes input programs. From the repository root:

```sh
cargo build --locked --release --example checked_evidence_probe
python3 scripts/bench-checked-evidence.py --samples 5
```

The report defaults to `target/benchmarks/checked-evidence.json`; `--probe PATH`
and `--report PATH` select alternatives. `--profile generous|candidate` selects
probe-only allowances; the default remains `generous`. The candidate option uses
the now-active standalone/graph profiles in the evidence policy (the option
name is retained for reproducible historical commands). Samples accept 1–30. Python 3 and Rust
are required; no Node dependency is needed. The default probe path includes
`.exe` on Windows, but Windows execution has not been qualified here.

The corpus reuses all five [compiler/runtime version 1](../compiler-runtime/README.md)
inputs and adds generated 1,024/4,096 Float/array/context functions, the checked-in
local-package example, and a two-unit graph with 16 aliases to one provider with
128 exported functions. Aliases pressure attached consumer interfaces and import
metadata without recompiling the canonical provider. Version 2 adds 192 layers of
inferred arrays in shallow syntax, 512 nominal record identities, 256 long binding
names, 64 calls with 64 Float parameters and a ten-unit diamond graph with eight
branches sharing one canonical provider. Increment this corpus version
when changing generators, case sizes or measurement semantics.

Each case runs one warmup per mode, then five samples by default with alternating
mode order. Every warmup/sample must succeed with byte-identical debug IR across
both modes and deterministic checked-evidence counters. No IR parity failure is
accepted as a timing. The probe uses AST-only standalone snapshots so both paths
parse exactly once; this does not measure source-owning IDE snapshot copies.
The package path includes existing resolution and interface/link work. Retained
metadata counters are emitted separately from IR. Package counters show initial
and remaining quotas; subtraction yields consumption. Probe-only finite quotas
are deliberately generous and are **not proposed production defaults**.

Raw samples and medians include process startup, reading/parsing/checking,
lowering, counter formatting and debug IR serialization. They do not isolate
semantic/lowering stages. POSIX child peak RSS uses the existing Python helper;
process-launch/inherited RSS floors can mask small memory differences. Unsupported
hosts report null. Neither measurement accounts for allocations or establishes
hard memory/CPU bounds. IR serialization also contributes to peak RSS. The probe
keeps its snapshot through lowering but does not simulate long-lived IDE retention.
The 30-second child timeout comes from the helper. There is no timing CI gate.

Reports retain the probe binary, runner/helper/corpus and compiler-source hashes,
input hashes, revision/dirty status, Rust/Python/OS details and raw observations.
The build command above identifies the intended release profile; supplied binary
build provenance cannot be inferred from its hash. Filesystem caches are warm;
results are local observations, not cold-start, cross-platform, fuzz, Windows,
MSRV or security qualification. A failed run exits without publishing a new
successful report: inspect timestamps and rerun rather than using an old report.

G08 still requires a reviewed production-limit policy, normal caller migration,
and the remaining ADR qualification gates. Finite record counters are not byte
limits for all compiler memory. Do not turn observed corpus maxima into default
quotas without considering adversarial shapes, attached interfaces and the
existing source/AST/interface/IR admission budgets.

## Local observation: Linux, 2026-11-12

Release/LTO probe, Rust 1.99.1, five samples per mode, one warmup. All nine cases
preserved exact IR in every sample. Selected medians from the retained local
report (rounding here only):

| Case | Legacy time | Checked time | Legacy peak RSS | Checked peak RSS |
| --- | --- | --- | --- | --- |
| 1,024 context functions | 10.80 ms | 15.44 ms | 8.78 MB | 9.91 MB |
| 4,096 context functions | 39.48 ms | 64.89 ms | 22.89 MB | 35.27 MB |
| 16 aliases / 128 exports | 6.21 ms | 7.78 ms | 8.78 MB | 8.78 MB |

Decimal MB; small-case RSS values hit the process-launch floor. The 4,096-function
case retains 40,967 structural nodes, 32,771 facts, four interned types, 12,289
declarations, 8,194 uses, 4,097 coercions, 4,097 function results and 16,385
contexts, plus 31,662 declaration-name bytes. The alias graph compiles two units
but retains 2,048 import bindings and 68,928 spelling/target bytes. Import growth
therefore cannot be budgeted from unique provider count alone.

These observations show real overhead; they do not justify default promotion or
unbounded collection. Next qualify additional adversarial shapes/large graphs,
review finite production policies for every independent table, then migrate
normal callers with unchanged diagnostics and emission budgets. Stage-level
profiling may help distinguish indexing/recording from IR serialization before
optimizing. Raw reports live under `target/benchmarks/` and are local artifacts;
rerun on each target rather than treating these numbers as portable thresholds.

## Version 2 shape qualification: Linux, 2026-11-12

All 14 cases passed exact IR equality for one warmup and five samples per mode.
The report is `target/benchmarks/checked-evidence-v2.json`. Selected medians:

| Added case | Legacy time | Checked time | Key retained pressure |
| --- | --- | --- | --- |
| Inferred array depth 192 | 3.82 ms | 6.56 ms | 194 types from 579 nodes |
| 512 nominal identities | 4.15 ms | 5.62 ms | 515 types; 512 coercions |
| 256 long names | 4.48 ms | 5.77 ms | 258,454 declaration-name bytes |
| 64 wide calls | 4.41 ms | 6.68 ms | 4,096 coercions; 4,097 contexts |
| Eight-branch diamond | 4.52 ms | 4.96 ms | Ten canonical units; 264 imports |

All added cases hit the RSS launch floor, so equal reported memory is inconclusive.
The [production policy proposal](../../docs/reference/checked-evidence-policy.md)
records concrete candidate profiles and the remaining saturation/migration gates.
Version 1 observations above remain historical and use the previous runner hash.

## Version 3 combined tables and lowering: Linux, 2026-11-12

Version 3 adds 16,384 nominal records with typed factory functions and a five-unit
package graph whose four providers each contain 8,192 Float/array functions.
The former combines type identities, parameters, local declarations, uses and
contexts. The latter keeps linked IR and existing prepared interface/AST caches
alongside per-unit checked evidence. Canonical provider metadata is dropped after
its unit is emitted; the graph result retains IR, not checked snapshots.

```sh
cargo build --locked --release --example checked_evidence_probe
python3 scripts/bench-checked-evidence.py --profile candidate --samples 3 --report target/benchmarks/checked-evidence-v3-candidate.json
```

All 16 cases preserve exact IR across one warmup and three measured samples per
mode using candidate profiles. Selected medians (decimal MB):

| Case | Legacy time / peak RSS | Checked time / peak RSS |
| --- | --- | --- |
| Combined nominal/context tables | 264.53 ms / 111.80 MB | 419.71 ms / 136.48 MB |
| Four large providers | 491.26 ms / 188.20 MB | 714.41 ms / 205.89 MB |

The combined case retains 147,464 nodes, 81,924 facts, 16,387 types and 49,153
contexts. The graph charges 327,729 nodes, 262,164 facts, 98,317 declarations,
65,548 uses and 131,080 contexts across five units. It does not saturate the
1,250,000-node graph candidate or all tables simultaneously.

The manual saturation example also accepts `--lower` to include IR emission:

```sh
cargo build --locked --release --example checked_evidence_saturation
python3 scripts/benchmark-process.py target/release/examples/checked_evidence_saturation nodes exact --lower
python3 scripts/benchmark-process.py target/release/examples/checked_evidence_saturation types exact --lower
```

Single local samples peak at about 131.95 MB for the exact 250,000-node shape
(249,998 emitted instructions) and 109.09 MB for the exact 65,536-type shape
(only parameter declarations, zero instructions). The latter is not a heavy IR
workload. These include source generation/parsing/checking/lowering and snapshot
lifetime, but exclude debug IR serialization. The local artifact is
`target/benchmarks/checked-evidence-lowering-peaks.json`. Do not subtract separate
process peaks to estimate stage allocations or extrapolate graph maxima linearly.
No runtime programs are executed and no hard memory bound is established.

## Aggregate graph/IR boundary corpus, version 2

A separate manual probe constructs four providers with 249,999 AST nodes each
and a four-node root, reaching the existing 1,000,000-node aggregate source ceiling.
No provider exports are attached, so this isolates aggregate source/IR admission
rather than saturating interface copies or all semantic tables.

```sh
cargo build --locked --release --example checked_package_boundary
python3 scripts/bench-checked-package-boundary.py
```

Options: `--probe PATH`, `--report PATH`. Python 3 and Rust are required; default
probe paths include `.exe` on Windows, but this corpus is locally qualified on
Linux only. Each child uses the existing helper's 30-second timeout; inputs are
compiled, never executed. The default artifact is
`target/benchmarks/checked-package-boundary.json`. Inspect its success/timestamp;
a failed run does not replace an older successful report.

The runner verifies exact debug IR parity under existing default compilation
limits, then at the measured linked IR allowance (999,994 instructions, 500,078
string bytes). Reducing either IR allowance by one must preserve identical
located E3021/E3022 diagnostics. Adding one empty root function must produce
identical E3020 aggregate-AST rejection. A checked-only run with a cumulative
999,999-node evidence allowance must reject the original graph as E3025; no
individual unit exceeds that allowance, so resetting it per unit would fail this
check. The probe asserts every expected result before returning success.

Initial single-process observations on Linux place successful checked graph
compilation around 408 MB peak RSS versus 366 MB legacy, including resolution,
parsing, retained AST/interface caches, checking, lowering and debug IR output.
These are individual samples, without warmups or statistical timing claims.
Captured output is hashed and removed from the report; raw time/RSS, input,
runner/probe/source fingerprints and diagnostic/IR digests are retained. Diagnostic
digests compare within the fixture, not across randomly named temporary roots.

The exact IR cases saturate caller-selected allowances matching this graph's
output, **not** the default four-million-instruction/256-MiB payload ceilings.
The graph has little nominal/context/import pressure. Passing these cases is
not simultaneous table saturation, allocator accounting, a hard memory bound,
a reason to raise candidate limits or platform/security certification.

Version 2 of the aggregate corpus exercises ordinary checked package entry points
against explicit `compile_with_legacy_lowering`, rather than only the opt-in
checked API. It also compiles four providers each containing 65,536 nominal
parameter types: legacy acceptance and active checked E3025 rejection demonstrate
the new cumulative type-admission boundary. The runner now verifies 13 isolated
processes. Earlier local observations remain historical; inspect the current
report's hashes and raw samples for the activated compiler. No timing threshold
or worst-case memory bound is introduced.

## G08 final qualification: Linux, 2026-11-12

Fresh release builds pass all 16 checked-evidence cases with three samples per
mode and exact IR parity. `target/benchmarks/g08-final-checked-evidence.json`
records raw samples and hashes. The separate existing five-case compiler/runtime
corpus passes 21 observations with three samples per operation at
`target/benchmarks/g08-final-compiler-runtime.json`. Active profile decisions and
compatibility changes are in the policy. Earlier measurements above remain
historical; final G08 verification hashes are recorded locally under
`target/checked-evidence/g08-final-verification.json`. No timing threshold, hard
memory bound, cross-platform release qualification or security certification is
implied by completion of this contributor task.
