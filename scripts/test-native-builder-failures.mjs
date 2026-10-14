// The internal native builder must preserve existing output and clean failed builds.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const builder = join(root, 'target', 'debug', 'examples',
    process.platform === 'win32' ? 'native_builder.exe' : 'native_builder');
const source = join(root, 'dogfood', '01-task-manager', 'main.svr');
assert.ok(existsSync(builder), `Build the internal example first: ${builder}`);
const scratch = mkdtempSync(join(tmpdir(), 'sovra-native-builder-failures-'));

const run = (output, env = process.env, input = source) => {
    const result = spawnSync(builder, [input, output], {
        cwd: root, env, encoding: 'utf8', timeout: 30000, windowsHide: true,
    });
    assert.ifError(result.error);
    assert.equal(result.status, 1, result.stderr + result.stdout);
    return result.stderr;
};

try {
    const existing = join(scratch, 'existing');
    mkdirSync(existing);
    writeFileSync(join(existing, 'sentinel'), 'keep');
    assert.match(run(existing), /output already exists/);
    assert.equal(readFileSync(join(existing, 'sentinel'), 'utf8'), 'keep');
    assert.ok(!existsSync(join(existing, 'Cargo.toml')));

    const restrictedEnv = { ...process.env };
    for (const key of Object.keys(restrictedEnv)) {
        if (key.toLowerCase() === 'path') delete restrictedEnv[key];
    }
    restrictedEnv.Path = process.platform === 'win32' ?
        `${join(process.env.SystemRoot || 'C:\\Windows', 'System32')};${process.env.SystemRoot || 'C:\\Windows'}` : '';
    const library = join(scratch, 'library.svr');
    writeFileSync(library, 'fn helper() {}\n');
    const noEntry = join(scratch, 'no-entry');
    assert.match(run(noEntry, restrictedEnv, library), /entry function `main` was not found/);
    assert.ok(!existsSync(noEntry), 'Missing-entry build created output');
    assert.deepEqual(readdirSync(scratch).filter(name => name.startsWith('.sovra-native-stage-')), [],
        'Missing-entry build created staging directories');
    const failed = join(scratch, 'failed');
    assert.match(run(failed, restrictedEnv), /cannot launch Cargo/);
    assert.ok(!existsSync(failed), 'Failed build left the requested output directory');
    assert.deepEqual(readdirSync(scratch).filter(name => name.startsWith('.sovra-native-stage-')), [],
        'Failed build left staging directories');
    console.log('PASS native builder failure isolation: existing output, missing entry and missing Cargo');
} finally {
    rmSync(scratch, { recursive: true, force: true });
}
