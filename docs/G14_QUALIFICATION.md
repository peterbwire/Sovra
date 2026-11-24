# G14 loopback HTTP qualification

G14 has a repeatable qualification command for the experimental Linux loopback
subset. This records implemented socket/HTTP behavior; client authentication,
complete HTTP conformance and production server support remain open under the
[completion contract](design/G14_COMPLETION_CONTRACT.md).

```sh
python3 -m unittest discover -s scripts/tests -p test_qualify_g14.py
python3 scripts/qualify-g14.py
# Also requires the installed Rust Windows GNU standard library target:
python3 scripts/qualify-g14.py --windows-compile
```

Prerequisites are a supported Linux x86/x86_64/aarch64 host, procfs, Git, Python 3,
Node, Rust stable with rustfmt/Clippy and exact Rust 1.74.0. Loopback binds and
child processes must be permitted. Unsupported runtime hosts refuse, rather than
count cfg-disabled tests as acceptance. Windows compile checking is explicit and
never establishes Windows runtime support.

## Retained evidence

Each run creates a unique UTC-named directory under
`target/checked-evidence/g14-qualification/`. It never overwrites an earlier run,
removes test executables or edits source files. Each stage retains its command,
exit status, duration and combined stdout/stderr log. `verification.json` starts
with `success:false`, is updated after each stage, and records failure or
interruption before exit. A failed command stops qualification. Individual
commands have a 20-minute timeout; timeout/interrupt handling terminates their
process group and retains the outcome.

The source SHA-256 covers Git-tracked and nonignored new repository inputs,
including documentation, scripts, CI and source; ignored build artifacts and
Python bytecode do not enter it. Relative paths and executable bits contribute;
symlinks are hashed as links, not followed. Source changes before/during/after a
stage invalidate success. Finish source edits before running qualification.
The report retains exact toolchain version logs and the built CLI SHA-256.
This is evidence about observed inputs/outputs, not cryptographic attestation
or protection against a malicious privileged local process.

## Checked scope

The runner performs formatting, locked all-target checking, strict all-target/
all-feature Clippy, library and focused integration tests, exact MSRV checking,
a fresh CLI build and four Node security/language/catalog/documentation checks.
The optional Windows stage adds compile checking only. No test-count threshold
substitutes for coverage: every required Rust target must appear exactly once
with positive passing counts, no failures, no ignored tests and no filtering.
A missing or cfg-disabled target cannot pass qualification.

The Rust suite includes owned sockets, network host/policy, actual network CLI/
native clients, fixed/chunked framing, validated-discard trailers, Continue,
Host/Origin/method/target admission and independent actual HTTP/JSON clients.
Existing v3 host and data CLI/native workflows also run. Native tests delete
project files, clear the environment and remove Node/PATH availability. The
existing G13 complete-source suite and IDE host evidence remain separate.

Eight Python regressions test failure retention/stop behavior, absent commands,
timeouts, source edits/new files, target counting and silent skips. CI runs the
runner separately on Linux x86_64 and ARM64 and uploads reports even on failure.
Configured CI jobs are not proof of a hosted pass; collect actual uploaded
results before claiming hosted ARM64 acceptance.

## Completion boundaries

Implemented and locally qualified behavior covers the documented restricted
loopback HTTP lifecycle and independent CLI/native consumers. The following
remain open and must not be inferred from a passing report:

- Client authentication, identity/per-route authorization and deployment policy.
- Complete HTTP protocol/conformance coverage and retained/verified trailers.
- Concurrency/scheduling, backpressure, streaming, TLS and remote serving.
- Windows/macOS runtime, browser/mobile targets and release qualification.

Optional Host/Origin/method/target policies preserve the documented absent-policy
behavior and do not constrain explicitly granted raw socket protocols. A passing
runner neither enables these policies implicitly nor authenticates clients.
