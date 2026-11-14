# Experimental root-program testing

```sh
svr test --experimental-v2 tests.svr
svr test --experimental-v2 --format json tests.svr
```

This mode checks a regular `.svr` source or a supported [single-entry
package](experimental-v2-packages.md) and discovers root functions
whose names start with `test_`, in source order. Every test must be parameterless
and return Unit (including aliases). `main` is neither required nor automatically
called. Ordinary linked helpers and enum/match/control bodies are supported.
See [the independent fixture](../../examples/enum-tests/README.md).

Invalid source or any invalid test signature prevents all execution. E5001 reports
invalid signatures at original root-source spans; E5002 reports no discovered
tests; E5003 reports exhausted test-count admission. Diagnostic projection keeps
its 64-message cap and bounded messages. Compiler quota failures use E3025 and
unavailable checked evidence uses E3122. Unlocated failures have null spans.

The complete program is lowered once under `ProgramLoweringLimits::DEFAULT`.
Tests then execute sequentially with fresh VM frames, instruction/materialization
fuel, empty program arguments and exit status zero. No input or filesystem host
is granted. All functions are preflighted before each invocation, including unused
functions. Tests may explicitly call `main`; it retains ordinary checked rules.
An ordinary return with status zero passes; a nonzero status or runtime refusal
fails. Execution continues with the next test after failure.

## Limits and reports

The CLI admits at most **32 tests** and retains at most **1,048,576 captured UTF-8
bytes across the suite**, including one LF per line. Each test also receives the
[experimental run profile](experimental-v2-run.md): one million instructions,
one MiB output, one million cumulative value nodes, 16 MiB cumulative text, 256
frames and a fresh 30-second cooperative elapsed allowance. The elapsed allowance
is per invocation, not a whole-suite deadline. Compiler/helper work and cleanup
are not subject to a hard deadline or physical-memory guarantee.

Only successfully returned VM output is available to the report. A runtime
failure discards its captured lines and requested status. A nonzero ordinary
return retains its output and numeric status. If a returned capture exceeds the
remaining suite-output allowance, the runner discards that capture, reports a
test failure with null status, and leaves the remaining allowance unchanged.
Later tests can still run and report output that fits. This cap covers captured
text, not JSON escaping, function names, diagnostics or serialization overhead.

Human reports escape control characters. JSON uses existing [test schema version
1](test-json.md) and emits exactly one document with test output inside it.
Check/read failures also use the test report schema, never a check report.
The v2 VM's failure-output rule differs from the legacy interpreter, which retains
lines emitted before a runtime failure. Default `svr test` is unchanged.
Success returns 0; source/configuration/test/publication failure returns 1; usage
errors return 2 on stderr with no report. A publication failure may partially
write the report; output errors are handled without a panic.

Only the v2 flag, optional package `--locked`, one optional `--format human|json`,
and one input are accepted.
Use `--` before a dash-prefixed source path. Duplicate flags/options, dependency/registered-module
packages are rejected. `CheckedProgramSource::test` exposes the same
host-free runner with explicit lowering/runtime/suite policies. Library runtime
defaults remain unchanged; callers must choose their desired component limits.

G13 remains partial. Dependencies/imports, additional opt-in hosts, IDE selection,
JavaScript/Worker/native v2 backends and installed cross-platform qualification
remain open. This runner provides no subprocess isolation or OS sandbox.

For supported package directories, optional `--locked` verifies the captured
inputs before compilation or execution. See [v2 locks](experimental-v2-locks.md).
