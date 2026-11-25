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

All four authentication fields are required, with no unknown/duplicate fields.
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
