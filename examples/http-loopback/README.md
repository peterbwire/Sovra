# Bounded Sovra HTTP/JSON demonstration

This Linux experimental data-profile example runs five sequential connections,
one fixed-length HTTP/1.1 request per connection, then exits. Sovra code owns
routing and ordinary Result-based handler recovery. The trusted network host
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
                           ('GET','/missing',None), ('GET','/failure',None)]:
    client = http.client.HTTPConnection('127.0.0.1', 47123, timeout=5)
    client.request(method, path, body)
    response = client.getresponse()
    data = response.read()
    print(method, path, response.status, data)
    client.close()
PYTHON
```

Expected statuses are 200, 200, 200, 404 and 500. HEAD emits no body but retains
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
server: chunked requests, persistence, upgrades, TLS, async/concurrent scheduling,
authentication and production origin/Host policy remain unimplemented.
