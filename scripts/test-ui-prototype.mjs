// Prove a private Sovra view and event work through a reusable web renderer.
import assert from 'node:assert/strict';
import { randomBytes } from 'node:crypto';
import { createServer } from 'node:http';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createSovraProgram } from '../dogfood/01-task-manager/gui/bridge.mjs';
import { decodeUiView, renderUiView } from '../dogfood/01-task-manager/gui/ui-view.mjs';

const root = resolve(fileURLToPath(new URL('..', import.meta.url)));
const compiler = resolve(process.argv[2] ?? `${root}/target/debug/${process.platform === 'win32' ? 'svr.exe' : 'svr'}`);
const source = resolve(root, 'dogfood/01-task-manager/gui/ui-fixture.svr');
const token = randomBytes(32).toString('hex');
const programs = [];
try {
    for (const engine of ['javascript', 'interpreter']) {
        const program = createSovraProgram({ compiler, source, engine });
        programs.push(program);
        const initial = program.run();
        assert.equal(initial.status, 0, initial.stderr);
        const nodes = decodeUiView(initial.stdout.replaceAll('\r\n', '\n'));
        const html = renderUiView(nodes, token);
        assert.match(html, /<h1>Sovra component<\/h1>/);
        assert.match(html, /Ready &lt;from&gt; Sovra/);
        assert.ok(!html.includes('<from>'));
        assert.match(html, /name="action" value="complete"/);
        const completed = program.run(['complete']);
        assert.equal(completed.status, 0, completed.stderr);
        assert.match(renderUiView(decodeUiView(completed.stdout.replaceAll('\r\n', '\n')), token),
            /Completed in Sovra/);
    }
    assert.equal(programs[0].run(['complete']).stdout, programs[1].run(['complete']).stdout);
    for (const invalid of [
        '', 'SVR-UI-2\nheading|x\n', 'SVR-UI-1\ntext|x\n',
        'SVR-UI-1\nheading|x\nbutton|bad key|Go\n',
        'SVR-UI-1\nheading|x\nscript|alert(1)\n',
        'SVR-UI-1\nheading|x\nbutton|go|Bad\u001b\n',
    ]) assert.throws(() => decodeUiView(invalid));
    assert.throws(() => decodeUiView(`SVR-UI-1\nheading|${'x'.repeat(65536)}\n`));

    const program = programs[0];
    const server = createServer(async (request, response) => {
        try {
            let args = [];
            if (request.method === 'POST' && request.url === '/action') {
                const chunks = [];
                for await (const chunk of request) chunks.push(chunk);
                const form = new URLSearchParams(Buffer.concat(chunks).toString('utf8'));
                if (form.get('token') !== token || form.get('action') !== 'complete') {
                    response.writeHead(403).end('Forbidden'); return;
                }
                args = ['complete'];
            } else if (request.method !== 'GET' || request.url !== '/') {
                response.writeHead(404).end('Not found'); return;
            }
            const result = program.run(args);
            if (result.status !== 0) throw new Error(result.stderr);
            const html = renderUiView(decodeUiView(result.stdout.replaceAll('\r\n', '\n')), token);
            response.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' }).end(html);
        } catch (error) {
            response.writeHead(500).end(String(error));
        }
    });
    try {
        await new Promise(resolveListen => server.listen(0, '127.0.0.1', resolveListen));
        const url = `http://127.0.0.1:${server.address().port}/`;
        assert.match(await (await fetch(url)).text(), /Ready &lt;from&gt; Sovra/);
        const clicked = await fetch(`${url}action`, {
            method: 'POST', body: new URLSearchParams({ token, action: 'complete' }),
        });
        assert.equal(clicked.status, 200);
        assert.match(await clicked.text(), /Completed in Sovra/);
        assert.equal((await fetch(`${url}action`, {
            method: 'POST', body: new URLSearchParams({ token: 'wrong', action: 'complete' }),
        })).status, 403);
    } finally {
        await new Promise(resolveClose => server.close(resolveClose));
    }
    console.log('PASS private Sovra UI component: two engines, web event, renderer validation');
} finally {
    for (const program of programs) program.close();
}
