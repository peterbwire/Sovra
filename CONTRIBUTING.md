# Contributing

Sovra welcomes focused experimental contributions. Start with the
[current maturity audit and contributor task board](docs/LANGUAGE_MATURITY_AUDIT.md),
[platform goal](docs/PLATFORM_VISION.md) and `AGENTS.md`. Mobile, desktop, web and
servers are the goal; current capabilities and release qualification are narrower.
Task Manager enhancements are paused while the compiler/UI/Windows foundations
are qualified. Existing application regression tests remain useful.

## First successful checkout

Install stable Rust and Node.js 22+ (used by backend/tooling tests). The Rust
package declares 1.74 as its minimum version. From a checkout:

```sh
git clone https://github.com/peterbwire/Sovra.git
cd Sovra
cargo build --locked
cargo run -- run examples/hello-world/main.svr
cargo run -- check --format json examples/functions/main.svr
cargo run -- new target/contributor-hello
cargo run -- test target/contributor-hello
```

Expect `Hello, Sovra!`, a successful JSON check and a passing generated language
test. `new` refuses existing paths: choose another fresh path on repeat runs.
These commands exercise executable source, not Fielddesk's proposed application
syntax. Compiler/CLI work can start on Linux, macOS or Windows; WPF, installer
and real Windows accessibility/DPI checks require Windows. Native builds need a
compatible linker; they are not necessary for a documentation/diagnostic change.

## Pick a bounded task

The audit's G01–G07 items are useful starting points for documentation, regression
tests, platform evidence, benchmarking and fuzzing. They are local task notes,
not already assigned GitHub issues. Check existing issues/PRs and explain your
intended scope before duplicating work. For component paths and acceptance
criteria, use the audit's source map/task board.

For performance work, use the versioned
[compiler/runtime corpus](benchmarks/compiler-runtime/README.md). It verifies
outputs and records process time/memory with hardware/toolchain and input hashes;
do not turn local timings into universal speed claims or arbitrary CI thresholds.

Add a focused regression for a bug, make the smallest coherent fix and update
the relevant contract/reference/example. Fundamental syntax, type/memory-model
and compatibility changes need an ADR discussion; do not expand the grammar
incidentally while fixing tooling or application examples. Keep platform-specific
code at host/backend boundaries and preserve the Rust-owned trusted pipeline.

## Validate and submit

For documentation/example changes, run `node scripts/check-doc-links.mjs` and
`node scripts/test-example-catalog.mjs` after building the compiler. The link
checker validates local file/directory targets, not remote URLs or heading
fragments. Executable example smoke tests compare interpreter/Node output;
checker-only and proposed application samples are not executed as services.

Follow `AGENTS.md` and the current development status. Preserve established
syntax and keep specification, examples, tests and course lessons synchronized.

Please include a concise description, validation commands, and any compatibility
considerations in pull requests. See [SECURITY.md](SECURITY.md) for security
reports.

Use locked, full-target checks for a compiler PR:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cargo test --locked --doc
```

Then run the relevant integration scripts. Examples: `node scripts/test-lsp.mjs`
for IDE changes, `node scripts/test-ui-protocol.mjs` for shared UI boundaries,
`node scripts/test-runtime-matrix.mjs` for backend behavior, and
`node scripts/test-dogfood.mjs` for the executable application corpus. Build first.
Read each script's prerequisites; browser/VS Code tests use optional external
test tools and native WPF tests require Windows. A source/reference compilation
or configured CI job does not count as executed platform acceptance.

In your PR describe the problem, final behavior, tests actually run, platform
limits and compatibility impact. Link the issue/task/ADR where relevant. It is
fine to state that a platform test needs CI; do not weaken or label a skipped
test as verified. Never upload secrets or undisclosed vulnerability reproductions
to public issue/PR logs. Security and conduct channels/support policy need the
maintainer clarifications recorded in task G17; this guide invents no contacts.

Keep generated builds, packages, probes and local qualification reports under
`target/`. Keep application data such as the root `tasks.txt` out of source
control; expected transcripts and intentional fixtures belong with their tests.
Before deleting old artifacts, retain the current verified package and its
checksum/report, inspect references, and verify cleanup paths stay within the
workspace. Preserve working compatibility adapters until their replacement is
qualified. An old filename or an unreferenced manual test is not sufficient
evidence that source code is obsolete.

