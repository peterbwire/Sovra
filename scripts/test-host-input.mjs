// Differential subprocess regressions for approved CLI input and streaming output.
import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');
const compiler = join(root, 'target', 'debug', process.platform === 'win32' ? 'svr.exe' : 'svr');
const source = join(root, 'tests', 'fixtures', 'process-input.svr');
const errorSource = join(root, 'tests', 'fixtures', 'process-arg-error.svr');
const directory = mkdtempSync(join(tmpdir(), 'sovra-host-input-'));

function run(executable, args, input = '') {
    const result = spawnSync(executable, args, {
        cwd: root, input, encoding: 'utf8', timeout: 30000,
        maxBuffer: 4 * 1024 * 1024, windowsHide: true,
    });
    assert.ifError(result.error);
    assert.equal(result.signal, null);
    return result;
}

const js = join(directory, 'process-input.js');
const jsError = join(directory, 'process-arg-error.js');
for (const [entry, output] of [[source, js], [errorSource, jsError]]) {
    const build = run(compiler, ['build', '--emit', 'js', entry]);
    assert.equal(build.status, 0, build.stderr);
    writeFileSync(output, build.stdout);
}

const hosts = [
    { name: 'interpreter', call: (entry, args) => [compiler, ['run', entry, '--', ...args]] },
    { name: 'javascript', call: (entry, args) => [process.execPath, [entry === source ? js : jsError, ...args]] },
];
const delimiter = run(compiler, ['run', source, '--', '--help']);
assert.equal(delimiter.status, 0, delimiter.stderr);
assert.ok(delimiter.stdout.startsWith('1\n--help\n'));
for (const host of hosts) {
    const [exe, args] = host.call(source, ['café']);
    const lines = run(exe, args, '\nhello\r\n');
    assert.equal(lines.status, 0, lines.stderr);
    assert.equal(lines.stdout.replaceAll('\r\n', '\n'), '1\ncafé\nfalse\n\nfalse\nhello\ntrue\n');
    const empty = run(...host.call(source, []));
    assert.equal(empty.status, 0, empty.stderr);
    assert.equal(empty.stdout.replaceAll('\r\n', '\n'), '0\ntrue\n\ntrue\n\ntrue\n');
    const badEncoding = run(exe, host.call(source, [])[1], Buffer.from([0xff, 0x0a]));
    assert.equal(badEncoding.status, 1);
    assert.equal(badEncoding.stdout.replaceAll('\r\n', '\n'), '0\n');
    assert.match(badEncoding.stderr, /standard input is not valid UTF-8/);
    const tooLong = run(exe, host.call(source, [])[1], Buffer.alloc(1048578, 0x61));
    assert.equal(tooLong.status, 1);
    assert.match(tooLong.stderr, /standard input line exceeds 1048576 bytes/);
    const exactLimit = run(exe, host.call(source, [])[1],
        Buffer.concat([Buffer.alloc(1048576, 0x61), Buffer.from('\n')]));
    assert.equal(exactLimit.status, 0, exactLimit.stderr);
    assert.ok(exactLimit.stdout.startsWith('0\nfalse\n'));
    assert.ok(exactLimit.stdout.endsWith('\ntrue\n\ntrue\n'));
    assert.equal(Buffer.byteLength(exactLimit.stdout, 'utf8'),
        Buffer.byteLength('0\nfalse\n\ntrue\n\ntrue\n') + 1048576);
    const missingArg = run(...host.call(errorSource, []));
    assert.equal(missingArg.status, 1);
    assert.equal(missingArg.stdout.replaceAll('\r\n', '\n'), 'before\n');
    assert.match(missingArg.stderr, /program argument index out of bounds/);
    console.log(`PASS ${host.name}: UTF-8, empty/CRLF/EOF, bounds, retained output`);
}

async function promptBeforeInput(host) {
    const [exe, args] = host.call(source, []);
    await new Promise((done, reject) => {
        const child = spawn(exe, args, { cwd: root, stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true });
        let stdout = '';
        let stderr = '';
        let wroteInput = false;
        const timer = setTimeout(() => { child.kill(); reject(new Error(`${host.name}: prompt did not arrive before input`)); }, 5000);
        child.stdout.on('data', chunk => {
            stdout += chunk.toString();
            if (!wroteInput && stdout.includes('0\n')) {
                wroteInput = true;
                child.stdin.end('ready\n');
            }
        });
        child.stderr.on('data', chunk => { stderr += chunk.toString(); });
        child.on('error', error => { clearTimeout(timer); reject(error); });
        child.on('close', status => {
            clearTimeout(timer);
            try {
                assert.equal(status, 0, stderr);
                assert.equal(stdout.replaceAll('\r\n', '\n'), '0\nfalse\nready\ntrue\n\ntrue\n');
                done();
            } catch (error) { reject(error); }
        });
    });
    console.log(`PASS ${host.name}: output visible before stdin arrives`);
}
for (const host of hosts) await promptBeforeInput(host);
