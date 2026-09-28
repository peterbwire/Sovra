# Path to a real HTTP application

Status: PLANNED. No Sovra HTTP server is implemented. Routes and service metadata
in Fielddesk do not listen, receive bytes or send network responses.

## Dependency chain

1. **Executable library boundaries:** load local dependency modules through the
   compiler, preserve exports/private access, package identity and source diagnostics.
   See proposed ADR 0010; application source scanning is insufficient.
2. **Control flow and data:** define and implement conditions, loops, records,
   enums/sum types, collections and byte buffers, including typed lowering and
   interpreter/JS behavior. JSON needs recursive values; text is not an adequate
   substitute for arbitrary network bytes.
3. **Errors and lifetime rules:** specify Option/Result or another approved error
   model, resource handles, cleanup and ownership. Do not invent these inside an
   HTTP builtin. Implement bounded input, nesting and allocation policies.
4. **Host I/O boundary:** a trusted socket adapter needs listen/accept/read/write/
   close, partial-I/O behavior, cancellation/timeouts and explicit host authority.
   Define backend support: a Node adapter is different from a browser runtime;
   unsupported hosts must fail clearly. No ambient package installation scripts.
5. **Protocol and JSON libraries:** implement framing, headers, methods, paths,
   request-size limits, malformed input handling, serialization and structured
   errors. Validate protocol behavior against primary standards at implementation
   time. Avoid pretending string concatenation is a robust protocol parser.
6. **Routing and handler contracts:** resolve handlers and validate bindings under
   ADR 0009. A declared route becomes a real dispatch table only with implemented
   request and response types. Provide ordinary libraries for domain behavior.
7. **Concurrency and deployment:** specify scheduling, synchronization,
   backpressure, cancellation and shutdown before exposing production concurrency.
   A bounded sequential loopback server can prove the earlier I/O chain without
   claiming production throughput, TLS or asynchronous execution.
8. **Distribution:** local consumption first; locked dependency graphs, integrity,
   archives and registry publication follow. `svr add web` remains planned until
   selection, verification, installation and compiler consumption actually work.

Filesystem access is needed for static-file serving/configuration, not for a
minimal in-memory response. Databases, auth providers and AI integrations are
later ordinary dependencies, not HTTP prerequisites or compiler keywords.

## Acceptance evidence

A test must start Sovra code that binds a real loopback socket on an allocated
port, send an HTTP request from an independent client, assert status/headers/body,
and stop the server reliably. Add malformed, oversized and truncated requests,
partial reads/writes, port conflicts and resource cleanup tests. Verify JSON
round trips, route misses and handler errors. Cross-backend support must be
tested per host; unsupported targets must not masquerade as success.

Do not claim HTTP support from emitted route metadata, an external server that
never invokes Sovra code, a browser fetch wrapper or a hard-coded sample response
printed to stdout. Production readiness also requires operational limits and
independent platform/security validation beyond this first functional server.
