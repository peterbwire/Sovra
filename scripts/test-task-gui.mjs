// Exercise the local browser UI against the real Sovra CLI and persisted state.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { copyFileSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');
const temporary = mkdtempSync(join(tmpdir(), 'sovra-task-gui-'));
const options = new Map();
for (let index = 2; index < process.argv.length; index += 2) {
    const key = process.argv[index];
    if (!['--compiler', '--source', '--server'].includes(key) || !process.argv[index + 1] || options.has(key)) {
        throw new Error('Usage: node scripts/test-task-gui.mjs [--compiler PATH] [--source PATH] [--server PATH]');
    }
    options.set(key, process.argv[index + 1]);
}
const compiler = resolve(options.get('--compiler') ?? join(root, 'target', 'debug',
    process.platform === 'win32' ? 'svr.exe' : 'svr'));
const source = resolve(options.get('--source') ?? join(root, 'dogfood/01-task-manager/main.svr'));
const serverFile = resolve(options.get('--server') ?? join(root, 'dogfood/01-task-manager/gui/server.mjs'));
let child;
try {
    const finalStates = [];
    for (const engine of ['javascript', 'interpreter']) {
        const state = join(temporary, `${engine}.txt`);
        const runSource = engine === 'javascript' ? join(temporary, 'compiled-task-manager.svr') : source;
        if (engine === 'javascript') copyFileSync(source, runSource);
        child = spawn(process.execPath, [serverFile, '--compiler', compiler, '--source', runSource,
            '--state', state, '--port', '0', '--engine', engine],
            { stdio: ['ignore', 'pipe', 'pipe'], windowsHide: true });
        const address = await new Promise((resolveAddress, reject) => {
            let output = '';
            const timer = setTimeout(() => reject(new Error('GUI start timed out')), 10000);
            child.stdout.on('data', chunk => {
                output += chunk;
                const match = output.match(new RegExp(`Sovra Task Manager GUI \\(${engine}\\): (http:\\/\\/127\\.0\\.0\\.1:\\d+\\/)`));
                if (match) { clearTimeout(timer); resolveAddress(match[1]); }
            });
            child.once('error', reject);
            child.once('exit', code => reject(new Error(`GUI exited early: ${code}`)));
        });
        if (engine === 'javascript') rmSync(runSource);
        const first = await fetch(address);
        assert.equal(first.status, 200);
        const empty = await first.text();
        assert.match(empty, /No tasks yet/);
        const token = empty.match(/name="token" value="([0-9a-f]+)"/)?.[1];
        assert.ok(token);
        const action = async (command, title, suppliedToken = token) => fetch(new URL('action', address), {
            method: 'POST', headers: { 'content-type': 'application/x-www-form-urlencoded' },
            body: new URLSearchParams({ command, title, token: suppliedToken }),
        });
        assert.equal((await action('add', 'Write <tests>')).status, 200);
        assert.equal((await action('add', 'Write <tests>')).status, 400);
        let html = await (await fetch(address)).text();
        assert.match(html, /Write &lt;tests&gt;/);
        assert.ok(!html.includes('<tests>'), 'title injected HTML');
        assert.equal((await action('complete', 'Write <tests>')).status, 200);
        html = await (await fetch(address)).text();
        assert.match(html, /Done · #1/);
        assert.equal((await action('add', 'Second task')).status, 200);
        assert.equal((await action('add', 'Pipe | tail  ')).status, 200);
        assert.match(await (await fetch(address)).text(), /Pipe \| tail  /);
        assert.equal((await action('add', 'Unicode\u2028separator')).status, 200);
        assert.match(await (await fetch(address)).text(), /Unicode\u2028separator/);
        assert.equal((await action('delete', 'Write <tests>')).status, 200);
        assert.match(await (await fetch(address)).text(), /Second task/);
        assert.ok(!readFileSync(state, 'utf8').includes('Write <tests>'));
        assert.equal((await action('add', 'Blocked', 'wrong-token')).status, 403);
        assert.equal((await fetch(new URL('action', address), {
            method: 'POST', headers: {
                'content-type': 'application/x-www-form-urlencoded', origin: 'https://attacker.example',
            }, body: new URLSearchParams({ command: 'add', title: 'Blocked', token }),
        })).status, 403);
        assert.equal((await action('add', 'Bad\u001b[2J')).status, 400);
        assert.ok(!readFileSync(state, 'utf8').includes('Blocked'));
        assert.ok(!readFileSync(state, 'utf8').includes('Bad'));
        finalStates.push(readFileSync(state, 'utf8'));
        const legacy = 'SVR-TASKS-2\n2\n1|0|Legacy\u001b[2JTitle\n';
        writeFileSync(state, legacy);
        const legacyPage = await (await fetch(address)).text();
        assert.match(legacyPage, /Legacy\\u\{001B\}\[2JTitle/);
        assert.match(legacyPage, /Edit in CLI/);
        assert.ok(!legacyPage.includes('\u001b'));
        assert.equal(readFileSync(state, 'utf8'), legacy);
        writeFileSync(state, 'corrupt\n');
        assert.equal((await fetch(address)).status, 500);
        assert.equal(readFileSync(state, 'utf8'), 'corrupt\n');
        const closed = once(child, 'close');
        child.kill();
        await closed;
        child = undefined;
    }
    assert.equal(finalStates[0], finalStates[1], 'generated JavaScript and interpreter state differ');
    console.log('PASS Task Manager browser GUI: generated JavaScript and interpreter workflows, HTML escaping, controls, CSRF');
} finally {
    child?.kill();
    rmSync(temporary, { recursive: true, force: true });
}
