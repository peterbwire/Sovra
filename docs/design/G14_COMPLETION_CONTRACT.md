# G14 resources, networking and HTTP completion contract

Status: active implementation. The trusted Rust embedding foundation now
provides version-four requests through checked `.svr` data execution with owned
execution-scope handles. Explicit external policies now enable CLI/native
loopback byte serving. A pure bounded fixed-length HTTP request parser is now
implemented, along with bounded final-response rendering and one-request owned
HTTP socket dispatch. Real Sovra handlers pass independent CLI/native HTTP/JSON
acceptance. The [incremental chunked decoder](../reference/http-chunked-v4.md)
now integrates checked head framing, shared HTTP work and socket deadlines;
CLI/native clients cover chunked Unicode JSON and declared trailers.
[Bounded validated-discard trailers](../reference/http-trailers-v4.md) remain
separate from headers and never authorize application behavior. Bounded
[100-continue handshakes](../reference/http-expect-v4.md) now precede body waits;
unsupported expectations refuse with 417 before body transfer. Optional external
[Host/Origin access policy](../reference/http-access-v4.md) now admits exact
request-head grants before Continue/body handling, with optional exact method/
raw-target lists and shared comparison work; a trusted [Bearer-verifier embedding boundary](../reference/http-authentication-v4.md)
now supports pre-body verification, public principal publication and credential
redaction. CLI/native credential backends and production qualification remain open. Trailer integrity/
retention, remaining protocol coverage, production server controls and G14 qualification
remain open.
G13's experimental Linux foundation remains complete under its documented scope.
Task Manager feature work remains paused.

## Implementation sequence

1. Own real loopback listeners and accepted streams under denied-by-default,
   exact-port grants. Bound live resources, byte operations, deadlines and admission
   work. Preserve callback cancellation separately from host errors and release
   reservations on failed publication, refusal and drop.
2. Integrate an explicit network host into checked Sovra execution and native
   images without ambient authority or a copyable/serializable socket value.
   Version and validate every host request and policy. Retain existing v3 host
   behavior; packages and native executables must not embed grants.
3. Implement bounded sequential byte-driven server execution from real `.svr`
   source. Exercise partial reads/writes, EOF, cancellation, port conflicts,
   resource exhaustion and shutdown with an independently implemented client.
4. Implement strict bounded HTTP request/response framing against RFC 9110 and
   RFC 9112, reviewing the primary standards when implementing the parser.
   Specify permitted methods/targets/headers, body framing, ambiguous lengths,
   transfer encodings, malformed/truncated/oversized input and connection closure.
   Add handlers, route misses, handler failures and independent JSON API consumers.
5. Qualify supported CLI/native/host paths, independent startup, limits and cleanup.
   Track additional platforms explicitly. Concurrency/scheduling, backpressure,
   streaming, TLS, remote binding and deployment require their own implemented
   and tested contracts before production-server claims.

## Initial socket boundary

`LoopbackNetwork` defaults to no grants. A trusted embedding caller grants exact
IPv4 loopback ports; zero grants only OS-selected ephemeral listens. It does not
permit an arbitrary explicit port or any remote address, DNS lookup or outbound
connect. At most 16 distinct grants exist in a domain. Unsupported hosts refuse
listening; only Linux runtime behavior is currently qualified.

Listeners and accepted streams are unique Rust owners with private descriptors.
They cannot be cloned, serialized or inserted into existing copy-value VM types.
Listener closure leaves already accepted streams independently owned. Resource
counts belong to the authority domain and outlive its registry object while
owners remain. Pending accepts reserve stream capacity before touching a new
connection. Publication, bind/set-nonblocking errors, cancellation and drop release
reservations. Explicit stream close attempts both-direction shutdown and reports
shutdown errors; descriptor destruction still happens on error.

Hard ceilings are eight listeners, 64 accepted/pending streams, 64 KiB per read
or supplied write and 30 seconds per I/O polling operation. Callers may only
narrow them, including zero admission/deadline limits. Each owner retains its
immutable limits. Atomic slot admission has a finite contention budget.

Nonblocking accept/read/write polling checks callback cancellation and a monotonic
deadline before each syscall attempt, retries Interrupted and waits briefly on
WouldBlock. A read returns a nonempty chunk or distinct EOF; zero requested bytes
is an input error. A write returns its actual progress; callers own the remaining
suffix. Oversized writes send no prefix. Effects already completed are not rolled
back by later cancellation. No background worker or socket scheduler is created.

These bounds cover the adapter's owners and requested buffers, not OS TCP backlog
memory, hard preemption, process isolation, or a whole-connection/request deadline.
A trusted embedder can create more authority domains. HTTP integration must add
aggregate request/connection/lifetime limits rather than treating these operation
limits as a complete server policy.

## Completion evidence

Do not close G14 from Rust-only echo tests or a Node socket adapter. A real Sovra
program must bind, receive and respond through a supported explicit host/native
path; an independent client must assert status, headers, body and JSON behavior.
Retain malformed/ambiguous/truncated input, partial I/O, disconnect, resource,
port-conflict, timeout/cancellation and reliable cleanup evidence. Unsupported
hosts/features must refuse without silently selecting another backend.

## Repeatable restricted-profile qualification

The [G14 qualifier](../G14_QUALIFICATION.md) retains frozen-source Linux library,
network/HTTP, CLI/native and v3 compatibility results plus strict lint/MSRV and
Node checks. It rejects partial/skipped targets, command failures and source
drift. CI uploads independent x86_64/ARM64 reports; configured jobs do not count
as hosted runtime evidence. A passing restricted-profile run does not close
client authentication, full conformance or production/platform gates.

Qualification source capture now refuses linked parents/files, missing tracked
inputs, special-file substitution and read-time identity/content changes. It
bounds manifest/file/aggregate bytes and retains failure reports even before a
source hash can be established. These tooling checks do not add protocol or
production-server guarantees.
