import { createServer } from 'node:http';
import { spawn } from 'node:child_process';
import { randomBytes, timingSafeEqual } from 'node:crypto';
import { mkdtemp, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const assets = new Map(['index.html', 'app.js', 'style.css', 'workspace.js'].map(name =>
    [`/${name === 'index.html' ? '' : name}`, new URL(`web/${name}`, import.meta.url)]));
const LIMIT = 1024 * 1024;
const decoder = new TextDecoder('utf-8', { fatal: true });

// Local single-document host. No caller-supplied paths, commands, or compiler flags.
export async function startEditor({ compiler = resolve('target/debug', process.platform === 'win32' ? 'svr.exe' : 'svr'), port = 0, timeout = 10000 } = {}) {
    compiler = resolve(compiler);
    const token = randomBytes(32).toString('hex');
    let active;
    let origin;
    const server = createServer(async (request, response) => {
        response.setHeader('Cache-Control', 'no-store');
        response.setHeader('X-Content-Type-Options', 'nosniff');
        response.setHeader('Referrer-Policy', 'no-referrer');
        response.setHeader('Content-Security-Policy', "default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'none'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'");
        const send = (status, value) => {
            response.writeHead(status, { 'Content-Type': 'application/json; charset=utf-8' });
            response.end(JSON.stringify(value));
        };
        try {
            if (request.headers.host !== new URL(origin).host) return send(403, { error: 'Invalid host' });
            if (request.method === 'GET' && assets.has(request.url)) {
                const name = assets.get(request.url);
                let body = await readFile(name, 'utf8');
                if (request.url === '/') body = body.replace('__TOKEN__', token);
                response.writeHead(200, { 'Content-Type': request.url.endsWith('.js') ? 'text/javascript' : request.url.endsWith('.css') ? 'text/css' : 'text/html; charset=utf-8' });
                return response.end(body);
            }
            if (request.method !== 'POST' || !['/compile', '/stop'].includes(request.url)) return send(404, { error: 'Not found' });
            const supplied = request.headers['x-sovra-token'];
            if (request.headers.origin !== origin || typeof supplied !== 'string' || !/^[a-f0-9]{64}$/.test(supplied) || !timingSafeEqual(Buffer.from(supplied), Buffer.from(token))) return send(403, { error: 'Invalid editor session' });
            if (request.url === '/stop') {
                active?.kill();
                return send(200, { stopped: Boolean(active) });
            }
            if (active) return send(409, { error: 'Compiler is busy' });
            if (request.headers['content-type'] !== 'application/json') return send(415, { error: 'Expected JSON' });
            let size = 0;
            const chunks = [];
            for await (const chunk of request) {
                size += chunk.length;
                if (size > LIMIT) return send(413, { error: 'Editor request exceeds 1 MiB' });
                chunks.push(chunk);
            }
            let input;
            try { input = JSON.parse(decoder.decode(Buffer.concat(chunks))); }
            catch { return send(400, { error: 'Invalid UTF-8 JSON' }); }
            const commands = { check: ['check', '--format', 'json'], ir: ['build', '--emit', 'ir'], js: ['build', '--emit', 'js'], run: ['run'] };
            if (!Object.hasOwn(commands, input?.action) || typeof input.source !== 'string' || /[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/u.test(input.source)) return send(400, { error: 'Invalid source or action' });
            // Reserve before asynchronous filesystem work so concurrent requests cannot overlap.
            if (active) return send(409, { error: 'Compiler is busy' });
            let cancelled = false;
            active = { kill() { cancelled = true; } };
            let directory;
            let result;
            try {
                directory = await mkdtemp(join(tmpdir(), 'sovra-editor-'));
                const source = join(directory, 'main.svr');
                await writeFile(source, input.source, { mode: 0o600 });
                if (cancelled) result = { status: null, error: 'Execution stopped' };
                else result = await new Promise(resolveResult => {
                    const child = spawn(compiler, [...commands[input.action], source], { cwd: directory, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
                    active = child;
                    const stdout = [], stderr = [];
                    let total = 0, reason;
                    const timer = setTimeout(() => { reason = 'Execution time limit reached'; child.kill(); }, timeout);
                    for (const [stream, parts] of [[child.stdout, stdout], [child.stderr, stderr]]) stream.on('data', chunk => {
                        total += chunk.length;
                        if (total > LIMIT) { reason = 'Output limit reached'; child.kill(); }
                        else parts.push(chunk);
                    });
                    child.on('error', () => { reason = 'Unable to start compiler'; });
                    child.on('close', code => {
                        clearTimeout(timer);
                        try { resolveResult({ status: code, stdout: decoder.decode(Buffer.concat(stdout)), stderr: decoder.decode(Buffer.concat(stderr)), error: reason }); }
                        catch { resolveResult({ status: code, error: 'Compiler returned invalid UTF-8' }); }
                    });
                });
            } finally {
                try { if (directory) await rm(directory, { recursive: true, force: true }); }
                finally { active = undefined; }
            }
            send(200, result);
        } catch { if (!response.headersSent) send(500, { error: 'Editor request failed' }); else response.end(); }
    });
    server.requestTimeout = 15000;
    server.headersTimeout = 10000;
    await new Promise((accept, reject) => { server.once('error', reject); server.listen(port, '127.0.0.1', accept); });
    origin = `http://127.0.0.1:${server.address().port}`;
    return { url: origin, async close() { active?.kill(); await new Promise(done => server.close(done)); } };
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
    const editor = await startEditor({ compiler: process.argv[2] });
    console.log(`Sovra Editor: ${editor.url}`);
    console.log('Local trusted-code preview. Press Ctrl+C to close.');
    process.once('SIGINT', async () => { await editor.close(); process.exit(0); });
}
