# Compiler fuzzing

These isolated cargo-fuzz targets use libFuzzer coverage feedback and AddressSanitizer.
They add no dependency to Sovra's normal package. The separate Cargo.lock pins
fuzzer dependencies; the normal compiler still declares Rust 1.74 compatibility.
Nightly fuzz builds do not qualify that minimum version.

Validated tooling: cargo-fuzz 0.13.2 and nightly-2026-11-10 on Linux x86_64.
Install a compatible C++ toolchain, then:

```sh
rustup toolchain install nightly-2026-11-10 --profile minimal
cargo install cargo-fuzz --version 0.13.2 --locked
cargo +nightly-2026-11-10 run --locked --manifest-path fuzz/Cargo.toml --example seed_corpus
cargo +nightly-2026-11-10 fuzz build
cargo +nightly-2026-11-10 fuzz run source -- -max_total_time=60 -timeout=5 -rss_limit_mb=1024 -max_len=4096 -dict=fuzz/source.dict
cargo +nightly-2026-11-10 fuzz run native_ir -- -max_total_time=60 -timeout=5 -rss_limit_mb=1024 -max_len=4096
```

Run commands from the repository root. Seed generation copies reviewed source
seeds and encodes valid ones into native images using the current compiler.
Generated corpora, targets, coverage and crash artifacts stay ignored. Keep
working corpora between campaigns; coverage feedback grows them. For longer
campaigns increase the time budget explicitly. Harnesses additionally cap source
input at 16 KiB and binary input at 64 KiB; these are testing budgets, not new
language limits or complete resource coverage.

The source target checks parser/semantic diagnostics and byte ranges, then lowers
successful inputs, round-trips native images and renders JavaScript. The native
target decodes arbitrary bytes and round-trips accepted images. Neither target
executes Sovra programs or generated JavaScript, invokes native linking, or
intentionally performs program file I/O. Fuzz in a disposable environment with
no production credentials. AddressSanitizer does not prove absence of all bugs.

The source target additionally exercises opt-in frontend budgets (4,096 source
bytes, 512 tokens and eight diagnostics) and checks bounded failures and ranges.
It runs the normal source pipeline separately. The harness choices are stricter
than the approved default frontend budgets and do not alter those defaults.
The normal failure path also checks the 256-entry report ceiling and the 4 KiB
message length/backing-capacity invariants. Reviewed flood and long-name seeds
exercise these paths; larger-input campaigns can use `-max_len=16384` within
the source harness ceiling.
The source target also exercises checked AST admission (256 nodes) and IR
emission (512 instructions/4 KiB string payload), checking fail-closed behavior
and source ranges without executing any resulting program. These are harness
choices; approved default source-unit quotas are defined in ADR 0037.

On a crash or timeout, retain the exact toolchain, command, seed and artifact.
Minimize it locally, for example:

```sh
cargo +nightly-2026-11-10 fuzz tmin source fuzz/artifacts/source/crash-REPLACE_WITH_ACTUAL_NAME
```

Add a focused failing ordinary regression before fixing the compiler, then replay
the minimized input and rerun normal checks. Treat findings as private until
reviewed and fixed, following [the security policy](../SECURITY.md); do not upload
new crash inputs or raw findings to public issues or CI logs automatically.
Short successful campaigns are evidence about the tested inputs, not security
certification or a replacement for sustained fuzzing and independent review.
