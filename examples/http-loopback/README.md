# Bounded Sovra HTTP/JSON demonstration

This Linux experimental data-profile example runs nine sequential connections,
one fixed-length or restricted chunked HTTP/1.1 request per connection, then
exits. Sovra code owns routing and ordinary Result-based handler recovery. The trusted network host
owns sockets, framing, deadlines and response validation.

From this directory, after building/installing `svr`:

```sh
svr lock --experimental-generics ./app
svr run --experimental-generics --experimental-data --locked --host-policy ./policy.json ./app
```

While it is running, use a separate terminal:

```sh
python3 - <<'PYTHON'
import http.client, json
for method, path, body in [('GET','/health',None), ('HEAD','/health',None),
                           ('POST','/echo',b'{"message":"Sovra"}'),
                           ('POST','/echo',[b'{"chunked":true}']),
                           ('POST','/echo',{'trailers':b'{"trailers":true}'}),
                           ('POST','/echo',{'expect':'100-continue','payload':b'{"continue":true}'}),
                           ('POST','/echo',{'expect':'unsupported','payload':b'{"never":true}'}),
                           ('GET','/missing',None), ('GET','/failure',None)]:
    client = http.client.HTTPConnection('127.0.0.1', 47123, timeout=5)
    if isinstance(body, dict) and 'expect' in body:
        payload = body['payload']
        client.putrequest(method, path)
        client.putheader('Content-Length', str(len(payload)))
        client.putheader('Expect', body['expect'])
        client.endheaders()
        if body['expect'] == '100-continue':
            interim = client.sock.makefile('rb')
            assert interim.readline(128) == b'HTTP/1.1 100 Continue\r\n'
            assert interim.readline(128) == b'\r\n'
            interim.close()
            client.send(payload)
    elif isinstance(body, dict):
        payload = body['trailers']
        client.putrequest(method, path)
        client.putheader('Transfer-Encoding', 'chunked')
        client.putheader('Trailer', 'X-Trace')
        client.endheaders()
        client.send(('%x\r\n' % len(payload)).encode('ascii') + payload +
                    b'\r\n0\r\nX-Trace: discarded\r\n\r\n')
    else:
        client.request(method, path, body, encode_chunked=True)
    response = client.getresponse()
    data = response.read()
    print(method, path, response.status, data)
    client.close()
PYTHON
```

Expected statuses are 200, 200, 200, 200, 200, 200, 417, 404 and 500. HEAD emits no body but retains
its representation length. `/echo` parses and reserializes JSON; `/failure`
deliberately raises an ordinary JSON Result error, which Sovra handles as a
fixed generic 500 response. Routes use the exact raw target; this demo does not
normalize paths, strip queries or implement a general method/router framework.

For independent native startup:

```sh
svr build --experimental-generics --experimental-data --locked --emit exe --output ./http-server ./app
./http-server --host-policy ./policy.json
```

The policy stays external. The generated executable does not need Node or the
original source tree. Do not pass policy grants to check/test/build. The catalog
checks this example without running it or opening a listener.

The [HTTP exchange contract](../../docs/reference/http-exchange-v4.md) describes
bounds and refusal semantics. This is a restricted loopback demo, not a production
server: trailer integrity/retention, transfer codings beyond chunked, persistence, upgrades,
TLS, async/concurrent scheduling,
authentication and production origin/Host policy remain unimplemented.

Chunked trailers are validated and discarded under [explicit limits](../../docs/reference/http-trailers-v4.md);
they never override request headers or supply verified integrity metadata.

The host performs a bounded [100-continue handshake](../../docs/reference/http-expect-v4.md).
Unsupported expectations receive 417 before body transfer. Sovra catches ordinary
receive failures after host socket cleanup and continues to the next connection;
VM faults/control cancellation and listener/respond failures still terminate.
