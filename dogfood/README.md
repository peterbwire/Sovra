# Sovra dogfood applications

These applications exercise the real compiler and execution engines. Application
logic belongs in `.svr` files, never in compiler branches or expected-output generators.
Nine verified runnable slices appear in `suite.json`; planned applications are
tracked in [the status document](../docs/DOGFOOD_STATUS.md).

The original six apps form a capability ladder without changing the established numbered
roadmap: `invoice` validates arguments and typed records; `log-report` aggregates
structured stdin; `02-file-processor` transforms bounded text files;
`01-task-manager` maintains persistent state; `file-manager` combines Sovra file
operations with a browser adapter; and `05-package-consumer` links a separate
local library. The numbers are roadmap identities, not a complexity ranking.

Notes, Budget and HTTP Server extend that ladder with persistent data,
bounded numerical evaluation and a real loopback transport. Every application
added after the original six must declare a `reusable` capability in
`suite.json`: a path to a local Sovra package or a reusable host adapter,
plus a short contract. The dogfood runner enforces this for new IDs. The app
README must explain how another app can use the capability and its limits.
An app-specific command alone does not satisfy this rule.

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

`05-package-consumer` demonstrates the currently supported local-package path:
the application and library have separate manifests, and the suite runs the
application directory as the source. This does not test a remote registry or
published package installation.

`02-file-processor` processes stdin/stdout and copies UTF-8 files through the
approved ADR 0015 text-file API. `scripts/test-text-files.mjs` tests the path-based
mode in both execution engines using isolated temporary files. It is a CLI text
processor, not a browser File Manager.

`file-manager` is an additional, unnumbered local browser prototype. Its Node
host enumerates a trusted workspace; its Sovra program previews, copies and
creates empty UTF-8 files. `scripts/test-file-manager.mjs` exercises both engines.
The shared suite also checks its source, IR, JavaScript, empty view and preview.
It remains unnumbered because `03` is reserved for the HTTP API in the established
application roadmap. Keep both file applications: one tests streaming text
processing, while the other tests the browser UI and named-file operations.

`sovra-notes` is a persistent CRUD CLI backed by a versioned UTF-8 file and an
importable local note-record package. `scripts/test-notes.mjs` exercises state
across independent processes in both engines, including corrupt-file rejection.
`sovra-budget` imports a bounded whole-unit calculation package; the shared
suite tests its numeric cases and invalid inputs. `sovra-http-server` exposes
a reusable private Node loopback adapter and a Sovra router package;
`scripts/test-sovra-http.mjs` sends real requests through both execution
engines. This host adapter does not implement native Sovra networking.
