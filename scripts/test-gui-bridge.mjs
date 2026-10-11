// Prove the host bridge works for a Sovra program unrelated to Task Manager.
import assert from 'node:assert/strict';
import { copyFileSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { pathToFileURL } from 'node:url';
import { join, resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');
const options = new Map();
for (let i = 2; i < process.argv.length; i += 2) {
    const key = process.argv[i];
    if (!['--compiler', '--source', '--bridge'].includes(key) || !process.argv[i + 1] || options.has(key)) {
        throw new Error('Usage: node scripts/test-gui-bridge.mjs [--compiler PATH] [--source PATH] [--bridge PATH]');
    }
    options.set(key, process.argv[i + 1]);
}
const compiler = resolve(options.get('--compiler') ?? join(root, 'target', 'debug',
    process.platform === 'win32' ? 'svr.exe' : 'svr'));
const source = resolve(options.get('--source') ?? join(root, 'examples/hello-world/main.svr'));
const bridge = resolve(options.get('--bridge') ?? join(root, 'dogfood/01-task-manager/gui/bridge.mjs'));
const { createSovraProgram } = await import(pathToFileURL(bridge).href);
const temporary = mkdtempSync(join(tmpdir(), 'sovra-bridge-proof-'));
try {
    for (const engine of ['javascript', 'interpreter']) {
        const runSource = engine === 'javascript' ? join(temporary, 'compiled-hello.svr') : source;
        if (engine === 'javascript') copyFileSync(source, runSource);
        const program = createSovraProgram({ compiler, source: runSource, engine });
        if (engine === 'javascript') rmSync(runSource);
        try {
            const output = program.run();
            assert.equal(output.status, 0);
            assert.equal(output.stdout.replaceAll('\r\n', '\n'), 'Hello, Sovra!\n');
            assert.equal(output.stderr, '');
            assert.throws(() => program.run([42]), /arguments must be strings/);
        } finally { program.close(); }
        assert.throws(() => program.run(), /closed/);
    }
    assert.throws(() => createSovraProgram({ compiler, source, engine: 'unknown' }), /Invalid Sovra engine/);
    console.log('PASS reusable GUI bridge: unrelated Sovra program in both engines');
} finally { rmSync(temporary, { recursive: true, force: true }); }
