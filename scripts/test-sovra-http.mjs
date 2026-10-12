// Prove real loopback HTTP requests route through Sovra in both engines.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { resolve } from 'node:path';
import { createSovraProgram } from '../dogfood/01-task-manager/gui/bridge.mjs';
import { createSovraHttpServer } from '../dogfood/sovra-http-server/host/adapter.mjs';

const root = resolve(import.meta.dirname, '..');
const compiler = resolve(process.argv[2] ?? resolve(root, 'target', 'debug',
    process.platform === 'win32' ? 'svr.exe' : 'svr'));
const serverFile = resolve(root, 'dogfood/sovra-http-server/host/server.mjs');
let child;
try {
    for (const engine of ['javascript', 'interpreter']) {
        child = spawn(process.execPath, [serverFile, '--compiler', compiler,
            '--engine', engine, '--port', '0'],
            { cwd: root, stdio: ['ignore', 'pipe', 'pipe'], windowsHide: true });
        const address = await new Promise((resolveAddress, reject) => {
            let output = '';
            let errors = '';
            const timer = setTimeout(() => reject(new Error(`HTTP startup timeout: ${errors}`)), 10000);
            child.stderr.on('data', chunk => { errors += chunk; });
            child.stdout.on('data', chunk => {
                output += chunk;
                const match = output.match(/Sovra HTTP Server \([^)]+\): (http:\/\/127\.0\.0\.1:\d+\/)/);
                if (match) { clearTimeout(timer); resolveAddress(match[1]); }
            });
            child.once('error', reject);
            child.once('exit', code => reject(new Error(`HTTP server exited ${code}: ${errors}`)));
        });
        const health = await fetch(new URL('health', address));
        assert.equal(health.status, 200);
        assert.equal(health.headers.get('content-type'), 'text/plain; charset=utf-8');
        assert.equal(await health.text(), 'ok');
        const about = await fetch(new URL('about', address));
        assert.equal(about.status, 200);
        assert.equal(await about.text(), 'Sovra HTTP Server');
        const payload = 'café\nsecond line';
        const echo = await fetch(new URL('echo', address), { method: 'POST', body: payload });
        assert.equal(echo.status, 200);
        assert.equal(await echo.text(), payload);
        const missing = await fetch(new URL('missing', address));
        assert.equal(missing.status, 404);
        assert.equal(await missing.text(), 'not found');
        const wrongMethod = await fetch(new URL('health', address), { method: 'POST', body: '' });
        assert.equal(wrongMethod.status, 404);
        assert.equal((await fetch(new URL('echo', address), {
            method: 'POST', body: 'x'.repeat(4097),
        })).status, 413);
        assert.equal((await fetch(new URL('echo', address), {
            method: 'POST', body: Buffer.from([0xff]),
        })).status, 400);
        assert.equal((await fetch(new URL('echo', address), {
            method: 'POST', body: Buffer.from([0]),
        })).status, 400);
        const closed = once(child, 'close');
        child.kill();
        await closed;
        child = undefined;
        const alternate = createSovraProgram({ compiler, engine,
            source: resolve(root, 'dogfood/sovra-http-server/host/alternate.svr'), cwd: root });
        const reusableHost = createSovraHttpServer(alternate);
        try {
            reusableHost.listen(0, '127.0.0.1');
            await once(reusableHost, 'listening');
            const alternateResponse = await fetch(`http://127.0.0.1:${reusableHost.address().port}/different`);
            assert.equal(alternateResponse.status, 200);
            assert.equal(await alternateResponse.text(), 'alternate handler');
        } finally {
            reusableHost.close();
            await once(reusableHost, 'close');
            alternate.close();
        }
    }
    console.log('PASS Sovra HTTP Server: loopback routing, UTF-8, limits, both engines');
} finally { child?.kill(); }
