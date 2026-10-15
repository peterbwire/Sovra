# Standalone executable capability and readiness matrix

Updated 2026-10-14. This matrix concerns the accepted internal N1 executable
proof and ADR 0027 public host-native builds, which embed checked Sovra IR and the Rust interpreter. `svr.exe` is
the developer toolchain; a produced application executable is a separate
artifact. The application runs without launching Node, Rust or `svr`. Building
it currently requires Cargo, a compatible host linker and either the repository
or the toolchain's version-matched runtime source payload.

## Implemented foundations and evidence

| Capability | Current support | Executable evidence |
| --- | --- | --- |
| Parsing and semantic validation | Existing source/package compiler checks remain mandatory | Native builder rejects invalid source before linking; missing entry is rejected before staging |
| Application entry | Parameterless `main`, OS arguments and working directory | General runtime fixture and Task Manager command tests |
| Functions and modules | Typed parameters, returns, private helpers and local package linking | Native package consumer calls exported functions through private helpers |
| Scalar values | Signed 64-bit Int, Float, Bool, UTF-8 String and Unit | Runtime values/text/error cases; nominal package Float widening |
| Collections and records | Arrays and nominal records retain interpreter value semantics | Copy isolation, indexed writes, structural equality within one nominal type, exported construction and field reads |
| Control flow | Branches, loops, calls and short-circuit logical operators | Runtime fixture checks loop output and absence of skipped side effects |
| Text and process APIs | All 14 registered standard functions and bare `print` alias | Runtime fixture calls the full registry; the runner rejects missing fixture coverage when registry names change |
| Input and terminal output | Explicit EOF, UTF-8, bounded lines, flushed output and terminal escaping | Runtime stdin cases, retained output on failure and interactive Task Manager parity |
| Files | Bounded UTF-8 reads and replacement writes with explicit result records | Exact 16 MiB boundary, oversized/invalid reads, failure preservation and persisted byte comparisons |
| Runtime errors | Overflow, zero division, bounds, call-depth and invalid host arguments return errors | Expected status/stderr checked against the interpreter |
| Exit status | Approved 0–125 contract | Native exit 125 and rejection of 126 |
| Image loading | Private versioned, bounded format validates declarations, payloads, jumps and calls | Decoder regressions; builder preflights its own image |
| Build isolation | Existing output refused; failed stages cleaned; explicit Cargo target directory | Builder failure tests and redirected-target native parity |
| Distribution independence | Executable contains program and runtime | Package/runtime tests delete their source copies and Cargo projects before restricted-PATH launches |
| Windows qualification archive | Public-command build, compiler/source provenance, checksums, direct imports and self-test | Exact extracted executable passes 18 cases locally; clean VM qualification remains pending |

Evidence is split among `scripts/test-native-runtime.mjs`,
`scripts/test-native-packages.mjs`, `scripts/test-native-task-manager.mjs`,
`scripts/test-native-builder-failures.mjs` and Rust image-decoder tests.
Hosted platform results and minimum-OS support require separate qualification.
Registry coverage proves that the fixture calls each builtin; it does not prove
every possible input or every combination of language features.

## Remaining developer-toolchain work

| Area | Status and next gate |
| --- | --- |
| Installed native build command | Implemented first slice: accepted ADR 0027 `build --emit exe --output PATH`, with installed acceptance tests and host-only output |
| Compiler/runtime distribution | Partial: `svr.exe` installs with a version-matched runtime source snapshot; hashes, complete file list and version checked before offline linking |
| Runtime isolation | Planned: separate build/runtime payload while preserving trusted Rust ownership and existing contracts |
| Language tests | Implemented: `svr test` uses the Rust interpreter; native/JS test targets remain planned |
| Reproducible packages | Partial: local dependencies work; locks, registry fetching and publishing remain planned |
| Source diagnostics in native failures | Partial: native errors provide messages, not source-level stack traces/debug symbols |
| Editor/debugger integration | Planned: language server, debugger and source mapping |
| Release compatibility | Pending: exact-commit CI, clean installation/application VM runs, minimum OS and architecture qualification |

## Later platform capabilities

Native UI, Node-free HTTP/network APIs, async scheduling, resource handles,
database drivers, binary file APIs, WebAssembly and mobile adapters remain
planned. The current browser GUI and HTTP dogfood use Node hosts. These are
useful future capabilities, not implemented promises of the first executable.
Each new capability needs general APIs, a reusable dogfood consumer, failure
tests and approval for any new syntax, type, memory or compatibility contract.

Sovra's existing foundations for humans and agents include versioned JSON
diagnostics, explicit signatures, nominal identity across packages, value
isolation, stable errors and reusable application capabilities. Continue building
on these verified contracts; the future Nova system is not an executable feature.

The next release work is exact-artifact qualification, followed by the
[clean Windows application test](NATIVE_TASK_MANAGER_VM_TEST.md)
and [toolchain installation test](TOOLCHAIN_CLEAN_WINDOWS_TEST.md). Approval of
ADR 0027 approves this first public CLI slice; later target/ABI policy needs review.
