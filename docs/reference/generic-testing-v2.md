# Experimental generic source-file test suites

Standalone source files run source-order root tests through the Rust-owned generic compiler and VM. Package directories now use the separate [generic package suite](generic-package-testing-v2.md) with shared graph proofs:

```sh
svr test --experimental-generics --format json main.svr
```

A test name starts with `test_` and must be nongeneric, parameterless and return
Unit. No `main` is required or invoked. Helpers can be generic, and tests can use
checked records, enums, arrays, Option/Result and propagation. Tests signal failure
through runtime refusal; no new assertion builtin is introduced. Each test gets
fresh VM state and finite policy, and later tests continue after runtime failures.
There are no stdin, file, process or network hosts in this profile.

Discovery is bounded before execution. Every selected test root is fully compiled,
including its specialization closure, and its Unit result is validated before any
test executes. All original bodies are symbolically checked. A second compilation
pass over the same immutable source executes one root at a time, avoiding retention
of an entire suite of specialized programs. This deliberately trades compilation
time for bounded program retention. Each compile retains the established finite
stage limits. Test count is hard capped at 32. The default suite byte allowance is
16 MiB across discovery and both passes: `source_bytes * (1 + 2 * tests)` is
admitted before compilation passes. This logical accounting is not a physical
memory or process-isolation guarantee.

Human and schema-one JSON reports reuse `svr test` outcomes and diagnostics.
Exit status is 0 for a nonempty all-passing suite, 1 for input/compile/discovery or
runtime failure, and 2 for invalid command arguments. Discovery uses E5001 for an
invalid test signature, E5002 for no tests, E5003 for suite admission; compilation
uses E3123. Runtime failures appear on their individual outcomes with source-span
information. I/O and package-lock failures produce empty test reports. Report
publication failure returns status 1.

Package directories now use [generic package suites](generic-package-testing-v2.md)
with registered modules/dependencies and shared checked body proofs. The two-pass
source-byte allowance above applies to standalone file tests. Multi-file native
Build and remaining E1/E2/G13 work stay open. Task Manager is unchanged.
