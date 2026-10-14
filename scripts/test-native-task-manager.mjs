// Compare the internal checked-IR executable proof with the Sovra interpreter.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const source = join(root, 'dogfood', '01-task-manager', 'main.svr');
const compiler = join(root, 'target', 'debug', process.platform === 'win32' ? 'svr.exe' : 'svr');
const args = process.argv.slice(2);
assert.ok(args.length === 0 || (args.length === 2 && args[0] === '--native'),
    'Usage: node scripts/test-native-task-manager.mjs [--native PATH]');
const scratch = mkdtempSync(join(tmpdir(), 'sovra-native-parity-'));

const execute = (binary, args, cwd, env = process.env, timeout = 30000, input = '') => {
    const result = spawnSync(binary, args, {
        cwd, env, input, encoding: 'utf8', timeout, windowsHide: true, maxBuffer: 8 * 1024 * 1024,
    });
    assert.ifError(result.error);
    assert.notEqual(result.status, null, `${binary} timed out or was terminated: ${result.signal}`);
    return {
        status: result.status,
        stdout: result.stdout.replaceAll('\r\n', '\n'),
        stderr: result.stderr.replaceAll('\r\n', '\n'),
    };
};

try {
    assert.ok(existsSync(compiler), `Build the debug compiler first: ${compiler}`);
    let native;
    if (args.length === 2) {
        native = resolve(args[1]);
    } else {
        const output = join(scratch, 'native-build');
        const build = execute('cargo', [
            'run', '--locked', '--target-dir', join(root, 'target'),
            '--example', 'native_builder', '--', source, output,
        ], root, { ...process.env, CARGO_TARGET_DIR: join(scratch, 'redirected-target') }, 600000);
        assert.equal(build.status, 0, `Native proof build failed:\n${build.stderr}\n${build.stdout}`);
        native = join(output, 'target', 'release',
            process.platform === 'win32' ? 'sovra-native-proof.exe' : 'sovra-native-proof');
    }
    assert.ok(existsSync(native), `Native proof executable missing: ${native}`);

    const interpreted = join(scratch, 'interpreted');
    const standalone = join(scratch, 'standalone');
    mkdirSync(interpreted);
    mkdirSync(standalone);

    const nativeEnv = { ...process.env };
    if (process.platform === 'win32') {
        for (const key of Object.keys(nativeEnv)) {
            if (key.toLowerCase() === 'path') delete nativeEnv[key];
        }
        const systemRoot = process.env.SystemRoot || 'C:\\Windows';
        nativeEnv.Path = `${join(systemRoot, 'System32')};${systemRoot}`;
    }

    let cases = 0;
    const compare = (args, expectedStatus, input = '') => {
        const interpretedResult = execute(compiler, ['run', source, '--', ...args],
            interpreted, process.env, 30000, input);
        const nativeResult = execute(native, args, standalone, nativeEnv, 30000, input);
        assert.deepEqual(nativeResult, interpretedResult,
            `Native/interpreter output differs for ${args.join(' ')}`);
        assert.equal(nativeResult.status, expectedStatus, `Unexpected status for ${args.join(' ')}`);
        for (const name of ['tasks.txt', 'broken.txt', 'interactive.txt', 'legacy.txt']) {
            const left = join(interpreted, name);
            const right = join(standalone, name);
            assert.equal(existsSync(right), existsSync(left), `State presence differs: ${name}`);
            if (existsSync(left)) {
                assert.deepEqual(readFileSync(right), readFileSync(left),
                    `State bytes differ after ${args.join(' ')}`);
            }
        }
        cases += 1;
    };

    compare(['list', 'tasks.txt'], 0);
    compare(['add', 'tasks.txt', 'Write tests'], 0);
    compare(['add', 'tasks.txt', 'Café'], 0);
    compare(['list', 'tasks.txt'], 0);
    compare(['complete', 'tasks.txt', 'Write tests'], 0);
    compare(['list', 'tasks.txt'], 0);
    compare(['delete', 'tasks.txt', 'Café'], 0);
    compare(['list', 'tasks.txt'], 0);
    compare(['add', 'tasks.txt', 'Write tests'], 1);

    const corrupt = 'SVR-TASKS-2\nnot-a-count\n';
    writeFileSync(join(interpreted, 'broken.txt'), corrupt);
    writeFileSync(join(standalone, 'broken.txt'), corrupt);
    compare(['list', 'broken.txt'], 1);
    assert.equal(readFileSync(join(standalone, 'broken.txt'), 'utf8'), corrupt,
        'Native program changed malformed state');
    compare(['add', join('missing', 'tasks.txt'), 'Cannot save'], 1);
    assert.ok(!existsSync(join(standalone, 'missing', 'tasks.txt')));

    compare(['interactive', 'interactive.txt'], 0,
        'add\nFrom stdin\nlist\ncomplete\nFrom stdin\nquit\n');
    compare(['list', 'interactive.txt'], 0);
    compare(['interactive', 'interactive.txt'], 1, 'add\n');
    compare(['interactive', 'interactive.txt'], 0, '');
    compare(['interactive', 'interactive.txt'], 1, 'add\nBad\u001b[2J\nquit\n');

    const legacy = 'SVR-TASKS-1\n8\n5|0|Legacy\n0|0|\n7|1|Done\n';
    writeFileSync(join(interpreted, 'legacy.txt'), legacy);
    writeFileSync(join(standalone, 'legacy.txt'), legacy);
    compare(['list', 'legacy.txt'], 0);
    assert.equal(readFileSync(join(standalone, 'legacy.txt'), 'utf8'), legacy,
        'Native listing rewrote legacy state');
    compare(['add', 'legacy.txt', 'Migrated'], 0);
    assert.match(readFileSync(join(standalone, 'legacy.txt'), 'utf8'), /^SVR-TASKS-2\n/);

    console.log(`PASS native Task Manager parity: ${cases} cases, matching state bytes and exit status`);
} finally {
    rmSync(scratch, { recursive: true, force: true });
}
