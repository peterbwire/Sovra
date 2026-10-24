// General runtime conformance through one compiled, source-independent program.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const compiler = join(root, 'target', 'debug', process.platform === 'win32' ? 'svr.exe' : 'svr');
const reference = join(root, 'tests', 'fixtures', 'native-runtime.svr');
const scratch = mkdtempSync(join(tmpdir(), 'sovra-native-runtime-'));
function execute(binary, args, cwd, env = process.env, timeout = 30000, input = '') {
    const result = spawnSync(binary, args, {
        cwd, env, timeout, input, encoding: 'utf8', windowsHide: true, maxBuffer: 4 * 1024 * 1024,
    });
    assert.ifError(result.error);
    assert.equal(result.signal, null, `${binary}: ${result.signal}`);
    return { status: result.status, stdout: result.stdout.replaceAll('\r\n', '\n'),
        stderr: result.stderr.replaceAll('\r\n', '\n') };
}
try {
    assert.ok(existsSync(compiler), 'Build the debug compiler first');
    const registered = [...readFileSync(join(root, 'src', 'compiler', 'stdlib.rs'), 'utf8')
        .matchAll(/name: "(std::[^"]+)"/g)].map(match => match[1]).sort();
    const exercised = [...new Set([...readFileSync(reference, 'utf8')
        .matchAll(/(std::\w+)\(/g)].map(match => match[1]))].sort();
    assert.deepEqual(exercised, registered, 'Native fixture must exercise every registered builtin');
    const source = join(scratch, 'runtime.svr');
    copyFileSync(reference, source);
    const executable = join(scratch, process.platform === 'win32' ? 'runtime.exe' : 'runtime');
    const built = execute(compiler, ['build', '--emit', 'exe', '--output', executable, source],
        root, process.env, 600000);
    assert.equal(built.status, 0, built.stderr + built.stdout);
    rmSync(source);
    const env = { ...process.env };
    for (const key of Object.keys(env)) if (key.toLowerCase() === 'path') delete env[key];
    const systemRoot = process.env.SystemRoot || 'C:\\Windows';
    env.PATH = process.platform === 'win32' ? `${join(systemRoot, 'System32')};${systemRoot}` : '';
    const interpreted = join(scratch, 'interpreted');
    const native = join(scratch, 'native');
    for (const directory of [interpreted, native]) {
        mkdirSync(directory);
        writeFileSync(join(directory, 'input.txt'), 'hello\ncaf\u00e9\n');
        writeFileSync(join(directory, 'output.txt'), 'keep until replaced');
        writeFileSync(join(directory, 'invalid.txt'), Buffer.from([0xff]));
        writeFileSync(join(directory, 'boundary.txt'), Buffer.alloc(16 * 1024 * 1024, 0x61));
        writeFileSync(join(directory, 'oversized.txt'), Buffer.alloc(16 * 1024 * 1024 + 1, 0x61));
        writeFileSync(join(directory, 'busy.txt.svr-write-lock'), 'owned test marker');
    }
    const cases = [
        { args: ['arguments', 'caf\u00e9'], status: 0, output: '2\ncaf\u00e9\n' },
        { args: ['text'], status: 0,
            output: 'true\nleft\nright:tail\ntrue\n-9223372036854775808\nfalse\ntrue\nfalse\n\\u{000A}\\u{0009}\n' },
        { args: ['values'], status: 0, output: '1\n9\ntrue\ntrue\nPoint { x: 2, y: 3 }\n6\nshort circuit\n-0\n' },
        { args: ['array-holes'], status: 0, output: '9007199254740992\n9007199254740993\n2.5\n' },
        { args: ['record-alias'], status: 0, output: 'true\ntrue\nPoint { x: 2, y: 3 }\n' },
        { args: ['stdin'], input: 'caf\u00e9\r\n', status: 0, output: 'false\n5\ntrue\n0\n' },
        { args: ['stdin'], input: '\ufeffcaf\u00e9\r\n', status: 0, output: 'false\n8\ntrue\n0\n' },
        { args: ['stdin'], input: '\ufeff', status: 0, output: 'false\n3\ntrue\n0\n' },
        { args: ['stdin'], input: '\ufeff\n\ufeff\n', status: 0, output: 'false\n3\nfalse\n3\n' },
        { args: ['stdin'], status: 0, output: 'true\n0\ntrue\n0\n' },
        { args: ['stdin'], input: Buffer.from([0xff, 0x0a]), status: 1, error: /not valid UTF-8/ },
        { args: ['stdin'], input: Buffer.alloc(1048577, 0x61), status: 1, error: /line exceeds/ },
        { args: ['stdin'], input: Buffer.concat([Buffer.alloc(1048576, 0x61), Buffer.from('\n')]),
            status: 0, output: 'false\n1048576\ntrue\n0\n' },
        { args: ['argument-error'], status: 1, error: /out of bounds/ },
        { args: ['exit-error'], status: 1, error: /0.*125/ },
        { args: ['overflow'], status: 1, error: /overflow/, output: 'before error\n' },
        { args: ['division'], status: 1, error: /division by zero/, output: 'before error\n' },
        { args: ['index'], status: 1, error: /out of bounds/ },
        { args: ['recursion'], status: 1, error: /depth/ },
        { args: ['exit'], status: 125, output: 'requested exit\n' },
        { args: ['read', 'missing.txt'], status: 0, output: 'false\nnot_found\n0\n' },
        { args: ['read', 'invalid.txt'], status: 0, output: 'false\ninvalid_utf8\n0\n' },
        { args: ['read', 'boundary.txt'], status: 0, output: 'true\n\n16777216\n' },
        { args: ['read', 'oversized.txt'], status: 0, output: 'false\ntoo_large\n0\n' },
        { args: ['copy', 'missing.txt', 'output.txt'], status: 1, output: 'not_found\n' },
        { args: ['copy', 'input.txt', 'missing-parent/output.txt'], status: 1, output: 'false\nnot_found\n' },
        { args: ['copy', 'input.txt', 'output.txt'], status: 0, output: 'true\n\n' },
        { args: ['conditional', 'conditional.txt', 'false', '', 'first'], status: 0, output: 'true\n\n' },
        { args: ['conditional', 'conditional.txt', 'true', 'stale', 'lost'], status: 0, output: 'false\nconflict\n' },
        { args: ['conditional', 'conditional.txt', 'true', 'first', 'second'], status: 0, output: 'true\n\n' },
        { args: ['conditional', 'busy.txt', 'false', '', 'lost'], status: 0, output: 'false\nbusy\n' },
    ];
    for (const test of cases) {
        const expected = execute(compiler, ['run', reference, '--', ...test.args],
            interpreted, process.env, 30000, test.input);
        const actual = execute(executable, test.args, native, env, 30000, test.input);
        assert.deepEqual(actual, expected, test.args.join(' '));
        assert.equal(actual.status, test.status);
        if (test.output !== undefined) assert.equal(actual.stdout, test.output);
        if (test.error) assert.match(actual.stderr, test.error);
        else assert.equal(actual.stderr, '');
        assert.deepEqual(readFileSync(join(native, 'output.txt')),
            readFileSync(join(interpreted, 'output.txt')));
        if (existsSync(join(native, 'conditional.txt'))) assert.deepEqual(
            readFileSync(join(native, 'conditional.txt')), readFileSync(join(interpreted, 'conditional.txt')));
        if (test.args[0] === 'copy' && test.status !== 0) {
            assert.equal(readFileSync(join(native, 'output.txt'), 'utf8'), 'keep until replaced');
        }
        assert.ok(!existsSync(join(native, 'missing-parent')));
    }
    assert.equal(readFileSync(join(native, 'output.txt'), 'utf8'), 'hello\ncaf\u00e9\n');
    console.log(`PASS native runtime: ${cases.length} cases, ${registered.length} builtins, values, errors and I/O bounds`);
} finally {
    rmSync(scratch, { recursive: true, force: true });
}
