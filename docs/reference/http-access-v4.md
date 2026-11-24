# Explicit HTTP Host and Origin admission

Version-four external policies may configure HTTP head admission separately from
socket port grants:

```json
{
  "version": 4,
  "listen_ports": [47123],
  "http_access": {
    "hosts": ["127.0.0.1:47123"],
    "origins": ["https://client.example"],
    "allow_missing_origin": true
  }
}
```

The three base `http_access` fields are required when the object is present; unknown,
duplicate, missing or mistyped fields refuse before sockets are created. Lists
contain at most 16 unique Strings of at most 256 bytes each. Empty lists are
valid denials. There are no wildcards, regular expressions, DNS resolution or
implicit grants. Hosts must satisfy the existing HTTP authority syntax. Origins
must be a single serialized `http://authority` or `https://authority`, without
paths, query, fragment, user information, spaces or opaque `null`.

Host matching is ASCII-insensitive and otherwise exact, including written port
and trailing dot. Origin matching is byte-exact, including scheme, case and
written/default port differences. The trusted operator must list intended wire
forms explicitly. Noncanonical alternatives never select a normalized fallback.

## Optional exact method and target grants

An access object may additionally configure both `methods` and `targets`:

```json
{
  "hosts": ["127.0.0.1:47123"],
  "origins": [],
  "allow_missing_origin": true,
  "methods": ["GET", "HEAD", "POST"],
  "targets": ["/health", "/echo", "/missing", "/failure"]
}
```

Both lists must be supplied together. If both are absent, the original Host/
Origin-only admission remains. Each list contains at most 16 unique Strings.
Methods are case-sensitive HTTP tokens of at most 32 bytes; targets are valid raw
origin-form targets (or `*`) of at most 256 bytes. An empty list denies all
requests. Rust callers use `HttpAccessPolicy::with_requests(methods, targets)`
before installing the immutable policy.

Admission requires membership in both independent lists: every listed method may
use every listed target. This is a Cartesian product, not per-route method or
identity-based permissions. All matching is exact. Query strings, trailing slash,
percent escapes and their letter case are preserved; there is no URL decoding,
prefix/glob matching or normalization. A literal `*` is not a wildcard; the
parser permits the asterisk request target only with OPTIONS.

For example, granting `/safe` does not admit `/safe?x=1` or `/safe/`. Granting
`/raw%2Fvalue` does not admit `/raw/value` or `/raw%2fvalue`. Operators must list
intended wire targets. Method/target denial is an access decision with bounded
403 closure before Continue/body handling. Application routing still determines
responses such as 404 after admission; the demonstration explicitly grants
`/missing` to exercise that behavior.

Optional request-list comparisons add `32 * method_grants + 256 * target_grants`
logical work units to the existing admission precharge, from the same budget.
Missing/malformed/duplicate or oversized grants refuse policy loading; there is
no partial-list fallback or new network authority. A grant never overrides
existing parser or host protocol restrictions.

## Check ordering and limits

The host validates request-head syntax and framing first. A configured access
policy then checks the single original Host field and at most one Origin field,
before any 100 Continue packet, waiting for or decoding a body, trailer
processing or handler publication. An initial bounded head read may already
contain body bytes; those bytes are not decoded or published before admission. Duplicate Origin fields refuse even if their values match.
Missing Origin is admitted only by the explicit Bool. An empty Origin list denies
all present Origin values; it does not override the absence rule. Forwarded or
X-Forwarded-Host fields do not replace Host for admission. Trailers cannot grant
or change head authority.

Unlisted/malformed origins and unlisted hosts return an `authority` host error,
attempt an empty bounded 403 Forbidden response and close the claimed stream.
The receiver does not wait for the declared body. Other syntax/framing/expectation
refusals can be determined earlier; no invalid request is published. Error output
is best-effort under the remaining response/I/O/deadline budgets.

Admission consumes the same HTTP work budget as head validation and subsequent
chunk/trailer decoding. It precharges a conservative `16 * head_field_count +
256 * (host_grants + origin_grants) + 512`, plus configured request-list costs,
logical units for bounded scans and
comparisons. Exhaustion returns a limit refusal rather than bypassing access.
No clock or counter is reset. This is not a physical instruction count.

Rust embedding callers construct immutable `HttpAccessPolicy::new(...)` values
and call `NetworkHostV4::set_http_access` before creating socket owners. Live-owner
configuration changes refuse. Policy values cannot be constructed by Sovra source,
embedded as grants in native images or selected through environment variables.

## Scope and compatibility

If `http_access` is absent, existing port-granted hosts retain their prior HTTP
behavior; Host/Origin admission is not implicitly enabled. The HTTP example now
configures it explicitly. This optional policy does not restrict an application
that was deliberately granted raw `net.read`/`net.write` authority: raw protocols
remain governed by their existing grants. It is not a protocol sandbox.

Host/Origin matching is not client authentication, complete CSRF protection or
CORS response behavior. Non-browser clients can forge these fields. Allowing
missing Origin permits clients that omit it, including some browser requests.
No credentials, sessions, CORS response headers or TLS are introduced.

## Evidence and remaining work

Six focused tests cover policy field/type/size/duplicate refusal, exact Host/
Origin matching, explicit missing/empty-list behavior, duplicate/null/list origins,
configuration timing, shared work exhaustion and pre-body 403 without 100.
Independent CLI/native clients send unlisted Host and Origin requests with
100-continue, withhold bodies and assert final 403. Subsequent valid requests
still pass. Allowed Origin and explicitly absent Origin paths also run through
actual Sovra handlers; native startup removes project files and Node/PATH.

Application authentication, deployment/CORS policy, broader protocol qualification,
TLS, concurrency/backpressure and additional-host runtime acceptance remain open
under the [G14 contract](../design/G14_COMPLETION_CONTRACT.md).

Validation: 693 unique Rust tests pass (600 library and 93 HTTP/network/v3-host
integration tests), including six access-policy tests and independent CLI/native
403 clients withholding request bodies. Four Node security/language/catalog/
documentation checks, strict Clippy, exact Rust 1.74.0, Windows GNU all-target
compilation and formatting pass. This targeted Linux G14 checkpoint does not
establish Windows runtime, complete HTTP conformance or production readiness.
Local evidence: `target/checked-evidence/g14-http-access/verification.json`.

Latest request-permission validation: 697 unique Rust tests pass (600 library
and 97 HTTP/network/v3-host integration tests), including ten access-policy
tests and independent CLI/native clients withholding bodies for denied methods/
query targets. Four Node security/language/catalog/documentation checks, strict
Clippy, exact Rust 1.74.0, Windows GNU all-target compilation and formatting pass.
This is targeted Linux G14 evidence; client authentication, full HTTP conformance,
production and Windows runtime qualification remain open. Local evidence:
`target/checked-evidence/g14-http-request-access/verification.json`.

The [trusted Bearer-verifier boundary](http-authentication-v4.md) is a separate
Rust embedding facility after access checks. Host/Origin/request lists themselves
still do not authenticate clients; CLI/native credential backends remain open.
