import assert from 'node:assert/strict';
import { request as httpRequest } from 'node:http';
import { startEditor } from '../toolchain/editor/server.mjs';

const editor = await startEditor({ timeout: 2000 });
try {
    const page = await fetch(editor.url);
    assert.match(page.headers.get('content-security-policy'), /frame-ancestors 'none'/);
    const html = await page.text();
    const token = html.match(/name="sovra-token" content="([a-f0-9]{64})"/)[1];
    const request = (body, headers = {}) => fetch(`${editor.url}/compile`, { method: 'POST', headers: { 'content-type': 'application/json', origin: editor.url, 'x-sovra-token': token, ...headers }, body: typeof body === 'string' || Buffer.isBuffer(body) ? body : JSON.stringify(body) });
    assert.equal((await request({ action: 'check', source: '' }, { origin: 'https://evil.example' })).status, 403);
    assert.equal((await request({}, { 'x-sovra-token': '0'.repeat(64) })).status, 403);
    const wrongHost = await new Promise((accept, reject) => {
        const outgoing = httpRequest(editor.url, { headers: { Host: 'evil.example' } }, response => { response.resume(); accept(response.statusCode); });
        outgoing.on('error', reject); outgoing.end();
    });
    assert.equal(wrongHost, 403);
    assert.equal((await request(Buffer.from([0xff]))).status, 400);
    assert.equal((await fetch(`${editor.url}/server.mjs`)).status, 404);
    assert.equal((await request({ action: '__proto__', source: '' })).status, 400);
    assert.equal((await request({ action: 'check', source: '\ud800' })).status, 400);
    assert.equal((await request('{')).status, 400);
    assert.equal((await request('x'.repeat(1024 * 1024 + 1))).status, 413);
    const compile = async (action, source) => {
        const response = await request({ action, source });
        assert.equal(response.status, 200);
        return response.json();
    };
    const valid = 'fn main() { print("Hello, editor! λ😀") }';
    const checked = await compile('check', valid);
    assert.equal(checked.status, 0);
    assert.equal(JSON.parse(checked.stdout).success, true);
    const invalid = await compile('check', 'fn main() { print("λ😀"); print(missing) }');
    const diagnostic = JSON.parse(invalid.stdout).diagnostics[0];
    assert.equal(invalid.status, 1);
    assert.ok(diagnostic.location.start > 0);
    assert.match(diagnostic.message, /missing/);
    assert.match((await compile('run', valid)).stdout, /Hello, editor! λ😀/);
    assert.match((await compile('js', valid)).stdout, /Hello, editor!/);
    assert.ok((await compile('ir', valid)).stdout.length > 0);
    const looping = 'fn main() { while true {} }';
    const timed = await compile('run', looping);
    assert.equal(timed.error, 'Execution time limit reached');
    const pending = compile('run', looping);
    await new Promise(done => setTimeout(done, 100));
    assert.equal((await request({ action: 'check', source: valid })).status, 409);
    const stopped = await fetch(`${editor.url}/stop`, { method: 'POST', headers: { origin: editor.url, 'x-sovra-token': token } });
    assert.equal((await stopped.json()).stopped, true);
    assert.notEqual((await pending).status, 0);
    assert.equal((await compile('check', valid)).status, 0);
    const flood = await compile('run', 'fn main() { while true { print("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx") } }');
    assert.equal(flood.error, 'Output limit reached');
    console.log('Editor: real check/run/IR/JS, diagnostics, authentication, admission limits, timeout, stop, concurrency, output cap passed.');
} finally { await editor.close(); }
