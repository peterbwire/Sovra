import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { readFile, readdir } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { createHash } from 'node:crypto';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { verifyIdeVsix } from './verify-ide-vsix.mjs';

export async function testInstalledDeveloperTools(prefix) {
    const launcher = join(prefix, 'developer-tools/launch.mjs');
    const ready = spawnSync(process.execPath, [launcher, '--check'], { encoding: 'utf8', timeout: 10000 });
    assert.equal(ready.status, 0, ready.stderr); assert.equal(JSON.parse(ready.stdout).success, true);
    const tools = join(prefix, 'developer-tools');
    const manifest = JSON.parse(await readFile(join(tools, 'manifest.json'), 'utf8'));
    if (manifest.vsix) {
        assert.equal(manifest.vsix.file, `sovra-language-${manifest.ide_version}.vsix`);
        const bytes = await readFile(join(tools, manifest.vsix.file));
        assert.equal(createHash('sha256').update(bytes).digest('hex'), manifest.vsix.sha256);
        const expected = new Map();
        async function visit(directory, relative = '') {
            for (const entry of await readdir(directory, { withFileTypes: true })) {
                const name = relative + entry.name;
                if (entry.isDirectory()) await visit(join(directory, entry.name), `${name}/`);
                else { assert.ok(entry.isFile()); expected.set(`extension/${name === 'LICENSE' ? 'license.txt' : name}`.toLowerCase(), await readFile(join(directory, entry.name))); }
            }
        }
        await visit(join(tools, 'ide'));
        verifyIdeVsix(bytes, expected, JSON.parse(await readFile(join(tools, 'ide/package.json'), 'utf8')));
    }
    const lsp = spawnSync(process.execPath, [fileURLToPath(new URL('test-lsp.mjs', import.meta.url))], { env: { ...process.env, SOVRA_LSP_LAUNCHER: launcher }, encoding: 'utf8', timeout: 60000, maxBuffer: 2 * 1024 * 1024 });
    assert.equal(lsp.status, 0, lsp.stdout + lsp.stderr);
    const child = spawn(process.execPath, [launcher, 'editor'], { stdio: ['ignore', 'pipe', 'pipe'] });
    const closed = new Promise(done => child.once('close', done));
    let stdout = '', stderr = '';
    child.stderr.on('data', chunk => { stderr += chunk; });
    try {
        const url = await new Promise((accept, reject) => {
            const timer = setTimeout(() => reject(new Error(`Installed editor did not start: ${stderr}`)), 10000);
            child.stdout.on('data', chunk => { stdout += chunk; const match = /Sovra Editor: (http:\/\/127\.0\.0\.1:\d+)/.exec(stdout); if (match) { clearTimeout(timer); accept(match[1]); } });
            child.once('error', error => { clearTimeout(timer); reject(error); });
            child.once('close', () => { clearTimeout(timer); reject(new Error(`Installed editor exited early: ${stderr}`)); });
        });
        const html = await (await fetch(url)).text();
        const token = html.match(/name="sovra-token" content="([a-f0-9]{64})"/)[1];
        const response = await fetch(`${url}/compile`, { method: 'POST', headers: { origin: url, 'content-type': 'application/json', 'x-sovra-token': token }, body: JSON.stringify({ action: 'check', source: 'fn main() {}' }) });
        assert.equal(response.status, 200); const result = await response.json();
        assert.equal(result.status, 0, JSON.stringify(result));
        assert.equal(JSON.parse(result.stdout).success, true);
    } finally { child.kill('SIGINT'); await closed; }
    console.log('Installed developer tools: compiler pairing, VSIX (when supplied), full LSP and editor check passed.');
}
if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) await testInstalledDeveloperTools(resolve(process.argv[2]));
