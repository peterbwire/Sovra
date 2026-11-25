# Explicit loopback policy for CLI and native execution

Linux data-profile programs can now use the version-four network host through
an external trusted policy file. The existing `--host-policy PATH` option selects
exactly the policy's declared version: version 3 retains its file/process/time
services; version 4 supplies loopback networking. Versions do not fall back or
combine grants. Source requests cannot add or widen authority.

```json
{
  "version": 4,
  "listen_ports": [47123],
  "limits": {
    "listeners": 1,
    "streams": 1,
    "bytes": 65536,
    "milliseconds": 2000
  }
}
```

Only `version`, optional `listen_ports`, optional `limits` and the separately
defined [HTTP exchange limits](http-exchange-v4.md) object `http` are accepted.
Port grants are distinct Ints in 0–65535, at most sixteen. Zero grants only
OS-selected ephemeral ports, not arbitrary explicit port numbers. An absent or
empty grant list denies all listens. Limits are nonnegative Ints and may only
narrow the socket defaults: eight listeners, 64 streams, 64 KiB per operation
and 30 seconds per polling operation. Unknown/duplicate fields, mixed versions,
malformed JSON, duplicate ports and widened limits refuse before execution.

```sh
svr run --experimental-generics --experimental-data --locked \
  --host-policy /trusted/network.json ./app
svr build --experimental-generics --experimental-data --locked \
  --emit exe --output ./app-native ./app
./app-native --host-policy /trusted/network.json
```

The program requests its granted port with `net.listen`, then uses
[version-four network requests](network-host-v4.md) for accept/read/write/close.
The native data image retains feature word 2047 because requests use the existing
JSON/data representation. Policy files and live socket ownership are never
embedded. The generated executable needs neither Node nor original project
sources, and accepts grants only through its explicit launch argument. No policy
is selected from environment variables. Without a policy the existing denied
host remains in place; version-four network requests gain no authority.

Check/test/build refuse `--host-policy`; builds create no listener or captured
network grants. Policies are bounded to 64 KiB regular files. Linux opens use
nonblocking/no-follow flags, reject endpoint symlinks and FIFOs, validate the
opened descriptor and cap reads even if the file grows. Windows compile coverage
checks reparse-file refusal; it is not network runtime acceptance. Version-three
service contracts are unchanged, with the same regular-file policy loader now
used by CLI/native version dispatch. Direct trusted Rust v3 APIs remain available.

Real tests start Sovra CLI/native byte servers and use an independent binary TCP
client. Native launch is tested after project deletion, environment clearing and
unavailable Node/PATH. Missing grants/policy, port conflicts and policy options
on non-run commands refuse. Existing v3 source/native authority tests also pass.
[Restricted HTTP exchanges](http-exchange-v4.md) now add framing and Sovra
handlers under aggregate policy. Full protocol, TLS, remote networking, async
scheduling and production deployment remain G14 work under the
[completion contract](../design/G14_COMPLETION_CONTRACT.md).

Targeted qualification passes 741 unique Rust tests (600 library, 77 CLI,
16 generic CLI, 18 native-image, 10 v3 host, seven socket-owner, six network-host,
three policy and four data/network CLI/native tests). Four Node checks cover
compiler security, language workflows, the example catalog and documentation
links. Clippy, formatting, exact Rust 1.74.0 and Windows GNU all-target compilation
pass. Logs and the targeted report are retained under
`target/checked-evidence/g14-external-network-policy/`.

For chunked HTTP input, `http.work` is shared by head validation and incremental
decoding, `body_bytes` caps decoded content and `line_bytes` also caps chunk-size/
extension lines. Encoded body bytes/chunks retain fixed 2 MiB/4096 hard ceilings.
Deadline and I/O counters span the complete exchange; see [chunked scope](http-chunked-v4.md).

Trailer sections inherit the head-byte, line-byte and header-count ceilings with
independent counters, and consume the same encoded-wire/work/deadline/I/O budgets.
Fields are validated and discarded; see [trailer policy](http-trailers-v4.md).

HTTP interim 100 responses honor output/head/line/work ceilings and consume the
same aggregate deadline and I/O operation count as request and final response
processing. No extra grants or policy keys select expectation handling.

Version-four policies may additionally configure the explicit `http_access`
object with required `hosts`, `origins` and `allow_missing_origin` fields. The
[Host/Origin contract](http-access-v4.md) defines exact matching, absence, bounded
work and pre-body refusal. Omitting the object retains existing HTTP behavior.

`http_access` may also supply paired `methods` and `targets` arrays with bounded
exact grants. Omitting both retains Host/Origin-only behavior; supplying only one
refuses. Their comparisons consume shared HTTP work. See [request permissions](http-access-v4.md#optional-exact-method-and-target-grants).

An optional `http_authentication` object now selects a captured, bounded
[external verifier command](http-command-authentication-v4.md). It requires
`http_access` and does not expose v3 process/file endpoints to source. Absence
retains existing authentication behavior; invalid configuration never falls back.
