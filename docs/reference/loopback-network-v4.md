# Owned loopback networking foundation

The Rust embedding API exports `LoopbackNetwork`, `LoopbackListener`,
`LoopbackStream`, `NetworkLimits` and `NetworkRead` from `compiler::v2`.
It is an initial G14 resource adapter, currently supported on Linux. The `v4`
module name does not change the existing v3 JSON host schema or install a v4
source/CLI protocol.

Create a denied network with `Default` or lower its finite limits with `new`.
The trusted caller explicitly `grant_port`s each allowed localhost port. Granting
zero allows ephemeral allocation only. `listen` binds IPv4 `127.0.0.1`; `port`
returns the actual allocated port. `accept` issues a uniquely owned stream.
`read(maximum, checkpoint)` returns a nonempty `NetworkRead::Data` or `Eof`.
`write(bytes, checkpoint)` returns a partial byte count. Clients must handle the
remaining suffix themselves. All I/O callbacks preserve their error type:
`Result<Result<T, HostError>, E>` separates cancellation from host refusal.

Limits and ownership semantics are specified in the
[G14 contract](../design/G14_COMPLETION_CONTRACT.md). Both explicit close and
ordinary drop release resource reservations. Accepted streams remain alive after
listener close. A read timeout or cancelled operation does not itself discard the
stream. Callers may explicitly close or drop it afterwards. Limits are immutable
for issued owners, independently lowerable and capped per authority domain.

Real TCP-client tests cover binary chunks, EOF, grouped resource lifetimes,
exact port grants, port conflicts, publication/accept/read/write cancellation,
read/accept deadlines, oversized operations without prefix writes, concurrent
listener admission and reservation recovery. Existing v3 hosts and binary-data
APIs remain separate and receive regression checks.

This is not yet callable from `.svr`, a Sovra-written server, HTTP, outbound
networking, TLS, async scheduling or production deployment. Next is checked host
integration and a bounded Sovra-owned sequential server. Browser/mobile and
additional desktop hosts retain their own explicit support gates.

Targeted Linux qualification passes 631 unique Rust tests: seven new socket
tests, ten existing host tests, fourteen byte-runtime tests and all 600 library
tests. Clippy, formatting, Rust 1.74.0 and a Windows GNU all-target compile check
pass. The cross-check is not Windows runtime acceptance. Evidence and logs:
`target/checked-evidence/g14-loopback-foundation/verification.json`.
