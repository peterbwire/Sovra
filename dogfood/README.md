# Sovra dogfood applications

These applications exercise the real compiler and execution engines. Application
logic belongs in `.svr` files, never in compiler branches or expected-output generators.
Only verified runnable slices appear in `suite.json`; planned applications are
tracked in [the status document](../docs/DOGFOOD_STATUS.md).

From the repository root, with Rust and Node.js installed:

```text
cargo build --locked
node scripts/test-dogfood.mjs
```

An explicit compiler can be selected with `--compiler PATH`, including a locally
built release binary. The default is this repository's `target/debug/svr` (`.exe`
on Windows), never a compiler discovered through PATH. Build before testing to
avoid testing a stale binary. Node executes the generated JavaScript as well as
running the test harness; it does not implement application operations.

Every registered application must pass source checking, IR generation, interpreter
execution, JavaScript generation and JavaScript execution. Each case supplies
optional program arguments and a stdin fixture; both outputs must match the
independently maintained expected transcript exactly (CRLF is normalized).
Nonzero exits, stderr, missing executables, timeouts and output over 4 MiB fail;
there is no execution-skip fallback. Each subprocess has a 30-second timeout.
Artifacts and versioned JSON evidence go to unique `target/dogfood/run-*` directories.
CI runs this after building on Windows, Linux and macOS, and uploads the evidence.

To register another runnable application, add its ID, source and behavioral cases
to `suite.json`. Each case has an ID and expected stdout, plus optional `args`
and `stdin` path. Paths are relative to `dogfood/`. Expected output is reviewed
test data; the runner never updates it automatically. Document limitations and
scenario coverage before registering a passing slice. Add error-exit scenarios
to the suite schema when applications gain a process-error contract.

When a failure reveals a compiler bug, minimize it into a compiler regression
test before fixing it, rerun that test, then rerun dogfood and the relevant suite.
Fundamental language choices still require approval under AGENTS.md.
