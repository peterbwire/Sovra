# Compiler/runtime corpus, version 1

This is a small, repeatable process-level baseline for the currently executable
language subset. It is independent of Task Manager. It does not measure language
completeness, compare languages or establish a performance/security guarantee.

Install stable Rust, Node 22+ and Python 3. From the repository root:

```sh
cargo build --release --locked
node scripts/bench-compiler-runtime.mjs
```

The default report is `target/benchmarks/compiler-runtime.json`. Options accept
values: `--compiler PATH`, `--samples 1..30`, `--report PATH`, `--python PATH`.
For Windows use a Python executable name/path available in that shell. The
default compiler is `target/release/svr` (`svr.exe` on Windows). Use `--compiler`
for debug or another candidate and record the build command/flags separately;
the runner hashes the executable but cannot infer how it was built. Available
Rust/Cargo versions describe the measurement environment, not proven provenance
of an arbitrary supplied compiler. The runner does not build the compiler.

## Inputs and correctness

`corpus.json` fixes case identities, generator sizes and expected stdout.
Increment its version when changing inputs, generation rules or expected output.
Commit fixtures and runner changes together. Each measurement records the exact
generated/source input SHA-256 and UTF-8 size, not just its friendly case name.
Source is copied into a unique temporary directory, removed after the run.

| Case | Purpose |
| --- | --- |
| startup | Empty entry point; compiler/Node runtime startup dominates. |
| integer-loop | 5,000 checked integer iterations; prints the exact sum. |
| unicode-growth | 300 immutable string concatenations; `std::len` measures 1,800 UTF-8 bytes, not 600 Unicode scalars. |
| declarations-32 / declarations-256 | Deterministically generated independent typed functions and one entry point; modest check/build scaling observations. |

For every case, measure JSON checking, JavaScript generation, interpreter run
and execution of the pre-generated JavaScript in Node. Also measure compiler
`--version` as a separate startup observation. Every warmup and sample verifies
exit status, empty stderr and exact expected output. JSON checks must succeed
with no diagnostics; repeated builds must emit identical JavaScript within the
run. Failed output is never recorded as a successful timing. A failure during
measurement writes an unsuccessful report with completed observations; failures
before report initialization (for example a missing compiler) exit immediately.

## Measurement contract

- One warmup per operation, then five samples by default. Retain raw samples and
  min/median/max wall time. Warmup results are checked but excluded from summaries.
- A fresh Python helper times one direct child, including process creation and
  termination. Python interpreter startup is outside that interval. Every run
  launches a fresh compiler/Node process; filesystem and OS caches are not reset.
  These are warm-environment process timings, not cold-boot latency.
- Linux/macOS record `RUSAGE_CHILDREN.ru_maxrss` from a fresh helper, converted
  to bytes using that OS's units. This is the maximum resident set for its child,
  including process-launch overhead; it is not allocator accounting or summed
  process-tree memory. Other hosts report memory as null, never zero. Container
  scheduling, inherited limits and other machine load affect observations.
- Hardware/OS, Node/Python and available Rust/Cargo versions, compiler hash/version,
  Git revision/dirty status, corpus/runner/helper hashes and generated JS bytes/hash
  are retained. No user environment variables or credentials are collected.
- Checking and JS generation include frontend, validation, output serialization
  and CLI overhead. They do not isolate parser/semantic/lowering time.
  Interpreter execution includes source compilation. Node execution excludes JS
  generation but includes Node startup. Do not compare these as equivalent pure
  execution costs. Run cases in a fixed order; no randomized statistical study
  is implied.
- Each child has a 30-second timeout; the outer helper has a 40-second timeout
  and bounded captured output. The trusted fixtures are deliberately small.

For comparisons, use the same corpus/input hashes, build profile, machine,
sample count and Node/Python versions; retain both complete reports and exact
build commands. Avoid background workloads and inspect raw spread before
optimizing. There is no pass/fail speed or memory threshold. No CI timing gate
is introduced. Native backend, large package graphs, invalid inputs, browser,
mobile, sustained workloads and isolated compiler-stage profiling remain future
corpus extensions. Generated JS size is recorded; installed/native package size
is not measured here.
