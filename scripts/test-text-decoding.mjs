// Large-input differential checks for strict integer text parsing.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
if (args.length !== 0 && (args.length !== 2 || args[0] !== '--compiler')) {
    throw new Error('Usage: node scripts/test-text-decoding.mjs [--compiler PATH]');
}
const compiler = args.length ? resolve(args[1]) :
    join(root, 'target', 'debug', process.platform === 'win32' ? 'svr.exe' : 'svr');
const source = join(root, 'tests', 'fixtures', 'parse-int-file.svr');
const directory = mkdtempSync(join(tmpdir(), 'sovra-decode-'));
const run = (binary, command) => {
    const result = spawnSync(binary, command, {
        cwd: root, encoding: 'utf8', timeout: 30000,
        maxBuffer: 4 * 1024 * 1024, windowsHide: true,
    });
    assert.ifError(result.error);
    assert.equal(result.status, 0, result.stderr);
    assert.equal(result.stderr, '');
    return result.stdout.replaceAll('\r\n', '\n');
};
try {
    const js = join(directory, 'program.cjs');
    writeFileSync(js, run(compiler, ['build', '--emit', 'js', source]));
    const input = join(directory, 'input.txt');
    for (const [name, text, expected] of [
        ['minimum', '-9223372036854775808', 'true\n-9223372036854775808\n'],
        ['overflow', '9223372036854775808', 'false\n0\n'],
        ['long-decimal', '9'.repeat(1024 * 1024), 'false\n0\n'],
        ['long-invalid', '9'.repeat(1024 * 1024) + 'x', 'false\n0\n'],
    ]) {
        writeFileSync(input, text);
        assert.equal(run(compiler, ['run', source, '--', input]), expected, `interpreter ${name}`);
        assert.equal(run(process.execPath, [js, input]), expected, `JavaScript ${name}`);
        console.log(`PASS text decoding/${name}`);
    }
} finally {
    rmSync(directory, { recursive: true, force: true });
}
