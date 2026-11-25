# Trusted HTTP Bearer-verifier boundary

Implemented as a Rust embedding foundation. A trusted application may install
`Box<dyn HttpBearerVerifier>` with `NetworkHostV4::set_http_bearer_verifier` after
setting HTTP access policy and before creating socket owners. Registration with
live owners or without access policy refuses. The verifier is owned by the
execution scope; its trait requires Send + Sync, preserving the host's existing
auto traits. Provider debug formatting is opaque.

This is an integration boundary, not a bundled credential backend. External JSON
policies, Sovra source and generated native images cannot register Rust callbacks.
An explicit [external command adapter](http-command-authentication-v4.md) now
provides bounded delegation for CLI/native launches; other unknown authentication
configuration still refuses. Hosts without a verifier retain prior behavior. No credentials,
cryptography, sessions, issuer configuration or implicit grants are introduced.

## Checked input and outcomes

The receiver validates the head and configured Host/Origin/method/target grants
first. It then admits exactly one Authorization field with a case-insensitive
Bearer scheme, one or more SP separators, and a 1..256-byte opaque credential.
The syntax follows the b64token form in
[RFC 6750 section 2.1](https://www.rfc-editor.org/rfc/rfc6750.html#section-2.1):
ASCII letters/digits, `-._~+/`, with optional trailing `=` padding after at least
one base character. The host does not decode or verify this credential itself.
Duplicate, absent, malformed or oversized credentials refuse without invoking
the provider. Header grammar/control-byte refusals still happen earlier.

The provider receives a borrowed `HttpBearerRequest` with exact checked method,
raw target, checked Host authority, optional admitted Origin and credential accessors. It receives no body or socket handle. Debug
output omits credentials and URI values. Its outcomes are:

| Decision | Host behavior |
| --- | --- |
| Granted(HttpPrincipal) | Continue with the validated public identity |
| Unauthenticated | Empty final 401 with static `WWW-Authenticate: Bearer realm="Sovra"` |
| Forbidden | Empty final 403 |
| Unavailable | Empty final 503 |

A provider-issued principal is a private validated type containing a public
1..64-byte ASCII label (letters/digits/dot/underscore/hyphen). Wire headers cannot
construct the trusted outer result field. The provider must verify credentials
and authorize this method/target using its reviewed backend. The interface does
not choose an equality algorithm, token format, key store or permission model.
Fixture providers in tests are not production credential verification examples.

## Audience and actual transport context

`authority()` returns the original Host value after configured access admission.
`origin()` returns the original admitted Origin, or None for explicitly permitted
absence. Values are not normalized or inferred from Referer, request targets,
Forwarded or X-Forwarded-* fields. The verifier may bind a verified credential to
a particular admitted audience even when the outer access policy permits several
Host/Origin values. These client-supplied values remain forgeable; admission does
not itself prove credential issuer/audience claims or client identity.

`transport()` returns `HttpBearerTransport::LoopbackPlaintext`, derived from the
actual owned socket path. An HTTPS Origin or forwarded protocol claim cannot
promote it to secure transport. A provider requiring a confidential/authenticated
channel can refuse it before body/Continue. No TLS or proxy trust is implemented.
The enum is non-exhaustive; future variants require actual qualified host support.
Debug formatting continues to omit all request context values.

## Publication, budgets and cleanup

Verification runs before 100 Continue, waiting for/decoding a body, trailer
processing or handler publication. An initial bounded head read may already
contain body bytes; they are not processed before admission. On success the
returned request description has a new outer `principal` String, and Authorization
is omitted from its header array. Other wire headers remain untrusted metadata;
an X-Principal header cannot override the outer field. Without a configured
verifier, the original description/header behavior remains unchanged.

Framework admission precharges `48 * head_field_count + authority_bytes +
origin_bytes + 2 * authorization_value_bytes + method_bytes + target_bytes + 512` from the same HTTP work budget used by head,
access and chunk/trailer parsing. This covers bounded framework work, not arbitrary
provider CPU/I/O. Provider implementations are trusted, synchronous and must
return promptly. Cancellation and the aggregate accept-to-response deadline are
checked before and after each provider call; a late result cannot publish an
identity or send Continue. There is no hard preemption of a blocked or panicking
provider, and this API is not an untrusted plugin sandbox.

Refusal, provider unavailability, timeout or cancellation drops the claimed socket.
Automatic final errors/challenges remain best-effort under response byte/field/
work, I/O and deadline limits. Messages contain no provider or peer text. The
host does not copy credentials into its returned VM value, echo them in errors,
or call provider debug formatting. This does not zeroize transport buffers or
control logging/copying inside a trusted provider. Explicit raw socket authority
still permits raw protocols and is not constrained by this HTTP boundary.

## Evidence and remaining work

Eleven Rust tests cover principal/registration limits, preserved Send/Sync,
opaque debug output, malformed/duplicate credential refusal, fixed provider
outcomes, redacted results, verified Continue, access/shared-work ordering,
post-provider cancellation/timeout, cleanup and an actual Sovra source consumer.
The consumer receives its trusted identity without Authorization metadata.
Additional audience/transport cases verify per-Host/Origin refusal, exact raw
context, missing Origin preservation and rejection of forged secure forwarding
claims before body transfer.
Existing CLI/native tests exercise unchanged default behavior without a provider.

Production credential backends require a separate reviewed contract,
including credential storage/lifecycle, comparison/cryptography dependencies,
identity permissions, offline native runtime packaging and independent real
credential clients. Bearer use also requires a reviewed secure transport model;
RFC 6750's TLS requirements are not fulfilled by this loopback-only foundation.
These and production/conformance/platform gates remain open under the
[G14 contract](../design/G14_COMPLETION_CONTRACT.md).

I/O providers can implement the additive `verify_controlled` method and poll
`HttpBearerControl::check()` while running. The default preserves original
synchronous implementations. The explicit command adapter uses this hook for
child cleanup on HTTP cancellation/deadline, not only a post-return check.
