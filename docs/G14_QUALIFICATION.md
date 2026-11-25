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
source symlinks and symlinked parent directories refuse. Source changes before/during/after a
stage invalidate success. Finish source edits before running qualification.
Initial source-capture failures also retain `success:false`, even before a source
hash is available. Tracked missing files and nonregular file substitutions refuse.
Linux capture opens each parent directory and file with no-follow descriptors;
nonblocking file opens prevent FIFO substitution from waiting for a writer.
Regular-file identity, size, mode and modification/change timestamps are checked
around reading and against a guarded path reopen. Replacement or growth refuses.
The Git manifest is capped at 4 MiB, source count at 10,000, each file at 16 MiB
and aggregate source bytes at 128 MiB. Reads stop at the admitted byte bounds.
These checks are not hard preemption of filesystem I/O or an atomic whole-tree
snapshot against a privileged malicious process.

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
Host/Origin/method/target admission, the trusted Bearer-verifier embedding
boundary, strict external verifier command protocol/cleanup, aggregate HTTP admission
(counting/clock boundaries/pre-verifier 429) and independent
actual authenticated CLI/native HTTP/JSON clients. Current coverage includes
owned head/body/Continue/final/refusal polling, two-slot delayed-peer interleaving,
private process-state polls and two-job pending command verification with gated
independent clients and direct-child cleanup. Trusted-provider compatibility
defaults and blocking APIs remain synchronous.
Existing v3 host and data CLI/native workflows also run. Native tests delete
project files, clear the environment and remove Node/PATH availability. The
existing G13 complete-source suite and IDE host evidence remain separate.

Eighteen Python regressions test failure retention/stop behavior, absent commands,
timeouts, source edits/new files, target counting and silent skips, plus linked/
missing/special inputs, parent links, mid-read mutation/replacement and capture
limits. CI runs the
runner separately on Linux x86_64 and ARM64 and uploads reports even on failure.
Configured CI jobs are not proof of a hosted pass; collect actual uploaded
results before claiming hosted ARM64 acceptance.

## Hosted restricted-profile evidence (2026-11-25)

[CI run 45604357126](https://github.com/peterbwire/Sovra/actions/runs/45604357126)
qualified revision `e47470d6fd608ebb0e155ec3a431d2a5557c8904` on both
Linux x86_64 and aarch64. Downloaded artifact reports were inspected, rather than
inferring acceptance from job configuration. Each passed 770 Rust tests and all
13 mandatory stages, with identical start/end source hash
`fa3054691e1a1131d84ee00066b59ecb46cb40910ab16e62f1990377c2ace3fe`.

| Host | Artifact ID | Report directory |
| --- | --- | --- |
| Linux x86_64 | 15017836497 | `20261125T112608.488540Z` |
| Linux aarch64 | 15017317466 | `20261125T112639.048690Z` |

The optional Windows compile stage was not requested by these hosted profiles;
its separate local evidence does not establish Windows runtime acceptance. The
same run passed broader macOS/Ubuntu Rust/tooling jobs and both supported G13
Linux profiles. The Windows job failed two formatter test expectations, addressed
in the subsequent follow-up; the whole CI run is not green. Later edits require
fresh evidence and do not change these reports' qualified revision.

## Consolidated local evidence (2026-11-25)

Revision `55451f1` passed all 14 stages of `--windows-compile` qualification,
including 773 Rust tests, strict Clippy, exact Rust 1.74 checking, Windows GNU
compilation and four Node checks. All 18 qualifier regressions passed separately.
The retained report is `target/checked-evidence/g14-qualification/20261125T182140.765554Z/`.
Its unchanged start/end source hash was independently rechecked after completion:
`7a638b426901be12746c7f5dd6c91681b72822aae66b4d636f0730645aee263e`.
This consolidates verifier startup/abandonment cleanup, configured zero/one/two
job admission, and actual CLI/native policy-transition acceptance.

Subsequent packaging/documentation edits do not change that qualified revision.
Windows hosted run 45673645777 passed Rust and formatter stages but failed
packaging because PowerShell resolved multiple Node installations into one
command path. Its follow-up selects the first executable in PATH order; fresh
Windows packaging execution remains required. Local compile checking never
establishes Windows runtime acceptance or production G14 completion.

## Completion boundaries

Implemented and locally qualified behavior covers the documented restricted
loopback HTTP lifecycle and independent CLI/native consumers. The following
remain open and must not be inferred from a passing report:

- Reviewed production credential backends, issuance/storage/lifecycle,
  identity/per-route authorization and deployment policy.
- Complete HTTP protocol/conformance coverage and retained/verified trailers.
- Production/preemptive scheduling, configurable readiness-driven pools, streaming,
  TLS and remote serving; bounded example I/O interleaving is implemented.
- Windows/macOS runtime, browser/mobile targets and release qualification.

Optional Host/Origin/method/target policies preserve the documented absent-policy
behavior and do not constrain explicitly granted raw socket protocols. A passing
runner neither enables these policies implicitly nor authenticates clients.
