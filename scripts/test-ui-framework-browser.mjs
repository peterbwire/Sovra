// Optional visual acceptance: shared renderer + actual Sovra component/events.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { createServer } from 'node:http';
import { randomBytes } from 'node:crypto';
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createSovraProgram } from '../toolchain/ui/shared/program.mjs';
import { decodeUiView, renderUiView } from '../toolchain/ui/shared/view.mjs';

const { chromium } = createRequire(import.meta.url)('playwright');
const root = fileURLToPath(new URL('..', import.meta.url));
const compiler = join(root, 'target/debug', process.platform === 'win32' ? 'svr.exe' : 'svr');
const scratch = mkdtempSync(join(tmpdir(), 'sovra-ui-browser-'));
const evidence = join(root, 'target/ui-framework');
mkdirSync(evidence, { recursive: true });
const token = randomBytes(32).toString('hex');
const program = createSovraProgram({ compiler, source: join(root, 'examples/ui-desktop/component.svr') });
const state = join(scratch, 'message.txt');
const current = () => {
    const result = program.run(['ui-layout-view', state]);
    assert.equal(result.status, 0, result.stderr);
    return decodeUiView(result.stdout.replaceAll('\r\n', '\n'), { version: 2 });
};
const server = createServer(async (request, response) => {
    try {
        let notice = '';
        if (request.method === 'POST' && request.url === '/ui-action') {
            const chunks = [];
            let bytes = 0;
            for await (const chunk of request) {
                bytes += chunk.length;
                if (bytes > 16384) throw new Error('Test event body too large');
                chunks.push(chunk);
            }
            const form = new URLSearchParams(Buffer.concat(chunks).toString('utf8'));
            if (form.get('token') !== token) { response.writeHead(403).end(); return; }
            const node = current().find(node => ['input', 'button'].includes(node.kind) && node.action === form.get('action'));
            if (!node || (node.kind === 'input') !== form.has('value')) { response.writeHead(400).end(); return; }
            const result = program.run(['ui-event', state, node.action, ...(node.kind === 'input' ? [form.get('value')] : [])]);
            notice = (result.stdout + result.stderr).trim();
        } else if (request.method !== 'GET' || request.url !== '/') {
            response.writeHead(404).end(); return;
        }
        const view = renderUiView(current(), token, '/ui-action', notice);
        response.writeHead(200, {
            'Content-Type': 'text/html; charset=utf-8', 'Cache-Control': 'no-store',
            'Content-Security-Policy': "default-src 'none'; style-src 'unsafe-inline'; form-action 'self'; base-uri 'none'; frame-ancestors 'none'",
        });
        response.end(`<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Sovra UI</title><style>body{margin:0;background:#f1f5f9;color:#18243b;font:16px system-ui,sans-serif}main{max-width:760px;margin:64px auto;padding:24px}header{display:flex;justify-content:space-between;align-items:center;color:#53647e;font-size:13px;margin-bottom:32px}header strong{font-size:20px;color:#243f86}h1{font-size:30px;letter-spacing:-1px;margin-bottom:24px}input{padding:12px;border:1px solid #aebcd0;border-radius:8px;background:white}button{padding:10px 18px;border:0;border-radius:8px;background:#304c99;color:white;cursor:pointer}button:focus-visible,input:focus-visible{outline:3px solid #f59e0b;outline-offset:3px}[role=status]{padding:12px 16px;border-radius:8px;background:#e2eadd}.sovra-ui-card{--sovra-ui-surface:white;--sovra-ui-border:#d2dbe8}@media(max-width:500px){main{margin:20px auto;padding:16px}h1{font-size:25px}}</style></head><body><main><header><strong>Sovra</strong><span>Shared component</span></header>${view}</main></body></html>`);
    } catch (error) { response.writeHead(500, { 'Content-Type': 'text/plain' }).end(String(error)); }
});
let browser;
try {
    await new Promise(resolveListen => server.listen(0, '127.0.0.1', resolveListen));
    browser = await chromium.launch({ headless: true,
        executablePath: process.env.SOVRA_BROWSER_EXECUTABLE || undefined, args: ['--no-sandbox'] });
    const page = await browser.newPage({ viewport: { width: 1040, height: 760 } });
    await page.goto(`http://127.0.0.1:${server.address().port}/`);
    await page.getByRole('heading', { name: 'Sovra reusable component' }).waitFor();
    assert.equal(await page.locator('[data-sovra-group="message-actions"]').evaluate(node => getComputedStyle(node).flexDirection), 'row');
    await page.getByLabel('Message', { exact: true }).fill('Hello from Sovra');
    await Promise.all([page.waitForNavigation(), page.getByRole('button', { name: 'Save', exact: true }).click()]);
    await page.getByText('Hello from Sovra', { exact: true }).waitFor();
    await page.getByRole('status').getByText('Message saved in Sovra').waitFor();
    await page.screenshot({ path: join(evidence, 'desktop.png'), fullPage: true });
    await page.getByLabel('Message', { exact: true }).fill('<img src=x onerror=alert(1)> λ😀');
    await Promise.all([page.waitForNavigation(), page.getByRole('button', { name: 'Save', exact: true }).click()]);
    assert.equal(await page.locator('img,script').count(), 0);
    await page.getByText('<img src=x onerror=alert(1)> λ😀', { exact: true }).waitFor();
    // The page intentionally has no script/connect permission. Exercise server
    // admission via the test HTTP client without weakening that page policy.
    const rejected = await page.request.post(`http://127.0.0.1:${server.address().port}/ui-action`, {
        form: { token, action: 'missing' },
    });
    assert.equal(rejected.status(), 400);
    await page.setViewportSize({ width: 360, height: 780 });
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    await page.screenshot({ path: join(evidence, 'narrow.png'), fullPage: true });
    await Promise.all([page.waitForNavigation(), page.getByRole('button', { name: 'Reset', exact: true }).click()]);
    await page.getByText('Ready from Sovra', { exact: true }).waitFor();
    writeFileSync(join(evidence, 'verification.json'), JSON.stringify({ success: true,
        native_windows_verified: false, engine: 'javascript', renderer: 'shared',
        checks: ['grouped layout', 'save', 'literal markup/Unicode', 'stale action rejection', 'narrow layout', 'reset'],
    }, null, 2) + '\n');
    console.log(`PASS shared UI browser acceptance; screenshots: ${resolve(evidence)}`);
} finally {
    if (browser) await browser.close();
    if (server.listening) await new Promise(resolveClose => server.close(resolveClose));
    program.close(); rmSync(scratch, { recursive: true, force: true });
}
