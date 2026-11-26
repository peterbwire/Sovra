# Explicit external HTTP verifier command

Experimental Linux CLI/native launches can delegate Bearer verification and
request authorization to a trusted external program. This uses the existing
owned v3 process adapter privately inside the v4 HTTP host. It does not expose
general process authority to Sovra source or bundle an issuer, token database,
cryptography or TLS.

## Trusted external configuration

A version-four policy may add:

```json
{
  "version": 4,
  "listen_ports": [47123],
  "http_access": {
    "hosts": ["127.0.0.1:47123"],
    "origins": [],
    "allow_missing_origin": true
  },
  "http_authentication": {
    "executable": "/opt/sovra/verifier",
    "cwd": "/var/lib/sovra/verifier",
    "arguments": [],
    "milliseconds": 500
  }
}
```

The four shown authentication fields are required, with no unknown/duplicate
fields. Optional `max_jobs` is an integer from 0 through 2; omission retains 2.
One permits one live backend job, and zero disables backend launches while
retaining authentication enforcement (requests fail unavailable, never anonymous).
Both blocking and pending verification share this configured admission ceiling.
It cannot raise the hard two-job limit or provide per-identity/distributed quotas.
Paths must be absolute; the program must be a native ELF executable. The host
captures executable/cwd handles before serving requests. Interpreters require
an explicitly fixed trusted script argument. Operators must protect the program,
scripts/resources and credential store against unauthorized modification.
The backend is trusted OS code, not a sandboxed/untrusted plugin.

Arguments are fixed policy Strings: at most 16, at most 8192 aggregate bytes
including terminators, no NULs. There is no implicit shell or inherited environment.
HTTP data never selects program, cwd, arguments or environment. Per-request
credentials go only through stdin, not argv/environment. Keep secret material
out of fixed arguments and policy files; the backend owns its protected store.

The policy requires `http_access`. If the command cannot be captured or the host
is unsupported, policy loading refuses; no unauthenticated fallback is selected.
Sovra check/test/build still cannot acquire runtime policy grants. Rust embedding
callers may construct `HttpCommandVerifier::new` and install it through the
existing trusted verifier API. Opaque Debug output omits command details.

## Strict version-one protocol

Each verification spawns one owned child. UTF-8 JSON goes to stdin, followed by
EOF. A request has exactly these fields:

```json
{"version":1,"method":"GET","target":"/private","authority":"127.0.0.1:47123","origin":null,"transport":"loopback-plaintext","credential":"example-fixture-token"}
```

The credential above is illustrative public test data. Method/target/authority/
Origin preserve the checked context described by the [verifier boundary](http-authentication-v4.md).
The transport describes the actual local socket; forwarding headers do not
promote its security. Missing Origin is null. No request body is sent to the
backend. The backend must verify credentials, bind issuer/audience/expiry and
identity as appropriate, and authorize this exact request using its reviewed
credential system. This adapter does not make those decisions itself.

Successful child exit (zero) and one complete stdout JSON object are required.
Only these exact shapes are admitted:

| Stdout | Outcome |
| --- | --- |
| `{"version":1,"decision":"grant","principal":"client-label"}` | Publish validated public principal |
| `{"version":1,"decision":"unauthenticated"}` | 401 with static Bearer challenge |
| `{"version":1,"decision":"forbidden"}` | 403 |
| `{"version":1,"decision":"unavailable"}` | 503 |

Unknown/extra/missing fields, duplicate JSON keys, trailing data, wrong versions,
invalid principal labels, nonzero exits, spawn/stdio/timeout errors and overflow
fail closed as 503. No backend stdout/stderr/error text is returned to the client
or Sovra handler. Principal is public metadata, never a place to return a token.
The successful request result redacts Authorization as before.

## Bounds, cancellation and custody

Stdin JSON and each stdout/stderr stream are limited to 4096 bytes. Protocol JSON
also has a 64-value ceiling. Oversized context cannot be truncated into a grant;
serialization refuses before spawning. Child duration is independently capped
at 1000ms and may be lowered to zero. This is a processing timeout, not a promise
that a zero-duration policy prevents all spawn effects.

The new cooperative `HttpBearerControl::check()` hook lets I/O providers poll
host controls. `verify_controlled` defaults to the original synchronous verifier
behavior; existing implementations remain source compatible. The command adapter
polls during owned child I/O. Host cancellation remains a distinct callback
failure, and the aggregate HTTP deadline can terminate the child before its own
duration expires. Both refuse before principal/body/Continue publication.
Direct calls to the uncontrolled `verify` method still have the child timeout.

Direct child processes are killed/reaped on refusal, cancellation or timeout.
This is not process isolation, a descendant-process supervisor, filesystem/network
confinement or hard OS preemption. The trusted backend must not leave independent
workers behind. Credentials/transient transport buffers are not zeroized by this
adapter; the backend must avoid logging or echoing secrets.

The private v3 process grant cannot be called through the source-visible v4 host;
selecting this policy does not enable v3 process/file operations for application
code. Explicit raw socket grants remain governed by their existing contract and
are not a protocol sandbox.

## Evidence and remaining work

Five protocol/process tests cover strict metadata/decisions, malformed/failed/
oversized replies, no inherited PATH, fixed configuration limits, private process
authority, child reaping, cancellation and separate child/HTTP timeouts.
Two independent CLI/native tests use a test-only Python verifier and protected
fixture key file: valid identity/redaction, wrong-token 401, request-permission
403 and subsequent service. Bodies are withheld for refusals. Native startup
removes project files and Node/PATH; its bytes are checked for absence of the
fixture credential. The external verifier/key remain explicit dependencies.

No production backend is shipped. Credential issuance/storage/rotation/revocation,
secure transport/TLS, reviewed identity permissions, rate limiting and deployment
acceptance remain open. In particular, this loopback plaintext profile does not
fulfill RFC 6750's TLS requirements. See the [G14 contract](../design/G14_COMPLETION_CONTRACT.md).

[Aggregate HTTP attempt admission](http-admission-v4.md) may be configured to
limit verifier-spawn pressure before parsing/authentication. It is separate from
backend per-identity quotas and does not provide process isolation or fair scheduling.

## Owned process polling prerequisite (2026-11-25)

The existing private process runner now retains child ownership, bounded input/
stdout/stderr buffers, offsets, pipe EOF state, exit status and original deadline
in an opaque internal task. A poll performs at most one stdin write, one bounded
read from each output stream and one nonblocking exit check, without sleeping.
Interrupted/WouldBlock operations yield; output ceilings are checked before append.
After exit, available data is drained over bounded turns until EOF or a quiet
post-exit sweep, preserving the existing direct-child/unsupported-descendant
contract. The result is published once; repeat polling after completion refuses.

The synchronous `process.run` wrapper uses these polls with its existing pacing
and explicit cleanup. Cancellation/failure still kills/reaps the direct child;
Drop remains a fallback. No worker thread, source-visible process endpoint or
extra process grant is introduced. Debug exposes no input/output content.

Tests move a binary-preserving multi-fragment stream through a real native child,
check bounded per-turn output and one-result publication, and verify explicit
kill/reap after control refusal. Existing v3, command protocol and authenticated
CLI/native consumers remain regression coverage. This is the process-state
prerequisite for asynchronous verification: HTTP verification still waits inside
the synchronous wrapper until a pending-verification stage is connected.

## Pending command verification (2026-11-25)

The command backend now starts one owned verification job during cooperative body
preparation instead of waiting in process.run. Pending verification retains only
owned checked/redacted request metadata, the admitted authority/transport context,
a bounded co-read body prefix and the job. It does not parse/process that body,
send Continue or publish principal/handler metadata before a grant.

Each subsequent `http.poll_body` advances one process exchange turn: at most one
stdin write, one read per output stream and one exit check, without sleeping.
Pending returns false. A grant moves into pending-body custody; the following
body turns retain all prior controls and only complete bodies can publish the
request. Denials enter cooperative refusal custody. Receive/raw I/O/head polling
and application responses cannot bypass pending verification. A repeated poll
never starts another child or charges another HTTP admission.

The configured command verifier admits at most `max_jobs` live jobs (default/hard
ceiling two), shared between its
polling and synchronous paths. Exhaustion fails unavailable/503 without spawning;
completion/close/cancellation releases the slot after child cleanup. This is a
per-configured-verifier bound, not a global machine quota or an OS sandbox. Lower
stream limits still apply; larger configurable pools remain a reviewed gate.
No worker threads or new source-visible process authority are introduced.

The child retains captured executable/cwd, fixed arguments, cleared environment,
strict 4096-byte protocol streams and the existing <=1000ms configured deadline.
Controls/deadlines are checked on polls; no OS watchdog or hard preemption between
polls is claimed. Callers must keep scheduling jobs or close them. HTTP accepted-
time deadlines are checked before/after starting/polling and before publishing a
decision. Close/cancellation/timeouts kill/reap the direct child; Drop is an
ownership fallback. Descendant supervision remains outside this contract.

`HttpVerificationJob` and `HttpBearerVerifier::start_verification` provide a trusted
Rust extension hook. Existing providers use the explicit synchronous compatibility
default; arbitrary trusted callbacks are not made asynchronous or preemptible.
Blocking receive keeps its existing synchronous verifier path. Native process
launch itself can still wait on the OS. Pending job/host debug output omits
credentials, private metadata and pipe buffers.

Tests prove an independently gated command yields while another request finishes,
then successfully resumes without a duplicate spawn; two-child admission/refusal
and slot reuse; PID disappearance after close, claimed cancellation and timeout;
and principal/credential privacy. The real authenticated CLI/native fixture uses
six connections: one verifier gated by a fixture file, four other verified attempts
and a final pre-verifier 429. The fast identity request must finish before the
client releases the gated verifier, and exactly five backend invocations occur.
The backend/key remain external test fixtures, not a production credential system.

## Captured launch identity

Native children execute the captured ELF descriptor while argv[0] retains the
fixed absolute executable name supplied at grant time (at most 4096 bytes, no
NUL). This supports programs that dispatch by their invocation name without
reopening a pathname at launch. Source arguments cannot replace argv[0]. A native
regression removes the grant-time alias before execution and verifies both the
captured program and original launch name remain in effect.

### Pending-owner cleanup on abandonment (2026-11-25)

Outstanding verification jobs now receive their explicit `cancel` hook when
pending HTTP custody is abandoned by callback cancellation, aggregate deadline
failure or host destruction, as well as explicit close. Taking the job before
calling cleanup prevents a second host-issued cancellation during destruction.
Explicit close can report cleanup errors; destruction is best-effort and cannot
replace the original cancellation/deadline error. Completed jobs still own their
completion cleanup; trusted jobs must retain safe Drop behavior. This does not
supervise descendants or make OS cleanup preemptible.

A trusted pending-job regression exercises successful and failing cleanup across
explicit close, close-all, callback cancellation, aggregate timeout and host
destruction. Each case observes exactly one host cancellation and no
Continue/handler/principal publication. Close/close-all report cleanup failures;
callback/deadline errors retain priority. After claimed-owner abandonment, tokens refuse reuse and stream reservations
are released even when cleanup reports an error. Cancellation before the call
claims ownership preserves the token and pending job for explicit later cleanup.
Existing command-job tests continue to check direct-child disappearance and
capacity release.

### Startup handoff cleanup (2026-11-25)

A newly returned verification job now enters temporary owned custody before the
host checks provider-reported cancellation, the post-start deadline or metadata
preparation. Any early return invokes the job's cancel hook once; successful
handoff transfers custody to the pending HTTP owner without cancellation. Cleanup
failures do not replace the original callback/deadline error. Trusted job Drop
remains a required safety fallback and neither startup nor cleanup is hard
preemptible. Regression cases cover cancellation observed inside provider startup
and immediately after return, with successful/failing cleanup, empty wire output,
released stream capacity and unusable abandoned handles.

### Disabled admission through real CLI/native servers (2026-11-25)

Independent raw TCP clients now exercise the same compiled Sovra HTTP application
with `max_jobs: 0` after its default-limit authentication checks. Five eligible
requests receive empty 503 responses before body transfer, without Continue or
credential publication; the sixth receives the existing pre-verifier 429 with
`Cache-Control: no-store`. No backend invocation is recorded. Each server exits
at its bounded connection count and releases its listening port.

CLI and standalone native paths both run this policy transition. The native
executable is reused after the project has been removed, with cleared environment
and unavailable Node/PATH, demonstrating that the external policy controls launch
admission rather than a grant embedded in the executable. Default-limit identity,
redaction, denial and delayed-verifier interleaving checks remain in the fixture.

### Single-job admission through real CLI/native servers (2026-11-25)

The independent fixture also reloads external policy with `max_jobs: 1`. A first
verifier records its start and waits behind a file gate. While it remains live,
a second valid request receives an empty final 503 before body transfer, without
Continue or a second backend invocation. Only then does the client release the
first verifier; its identity request must succeed. Dropping the held request
cannot satisfy acceptance. A subsequent identity request proves slot reuse;
credential/permission refusals and pre-verifier admission 429 remain checked.
Exactly four backend invocations serve the six-connection single-job run.

Both actual CLI and source-independent native paths retain the default-two,
configured-one and disabled-zero policy runs against the same application or
executable. This establishes bounded configured admission/reuse, not a queue,
readiness-driven pool, per-identity quota or production fairness guarantee.
