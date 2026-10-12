# Sovra HTTP Server

Status: **Partial Node-hosted loopback server**. Node accepts bounded UTF-8
requests and owns sockets. The Sovra app calls the reusable local `httplib`
package to choose status, content type and body for `GET /health`, `GET /about`,
and `POST /echo`; other routes return 404. Generated JavaScript or the
interpreter executes the same Sovra route function for every request.

```text
cargo build --locked
node dogfood/sovra-http-server/host/server.mjs
node scripts/test-sovra-http.mjs
```

The reusable private `host/adapter.mjs` accepts any Sovra program exposing the
bounded `SVR-HTTP-1` response stream. Another local Sovra app can import
`httplib::http::Response` and `route` through a path dependency, or reuse the
Node adapter with its own compatible handler. The behavioral test also starts
an unrelated Sovra handler through that same adapter. This protocol is not public or
stable. Requests are limited to 4 KiB of UTF-8 body and a 2,048-character
target; responses are limited to 64 KiB. The host binds loopback only. There
is no native Sovra socket API, TLS, concurrency contract, arbitrary headers,
JSON handling or production deployment claim. The general HTTP runtime
decisions remain in `docs/REAL_HTTP_APPLICATION_PLAN.md`.
