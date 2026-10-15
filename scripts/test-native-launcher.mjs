// Exercise the actual Windows terminal launcher without touching user task data.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

assert.equal(process.platform, 'win32', 'This launcher test requires Windows');
assert.equal(process.argv.length, 3, 'Usage: node scripts/test-native-launcher.mjs PACKAGE_DIRECTORY');
const packageRoot = resolve(process.argv[2]);
assert.ok(existsSync(join(packageRoot, 'start-task-manager.cmd')), 'Package must contain the launcher');
const scratch = mkdtempSync(join(tmpdir(), 'sovra-launcher-'));
try {
    const app = join(scratch, 'app with spaces and !');
    const localData = join(scratch, 'private data with spaces');
    mkdirSync(app);
    for (const name of ['task-manager.exe', 'start-task-manager.cmd']) {
        copyFileSync(join(packageRoot, name), join(app, name));
    }
    const env = { ...process.env };
    for (const key of Object.keys(env)) {
        if (['path', 'localappdata'].includes(key.toLowerCase())) delete env[key];
    }
    env.PATH = join(process.env.SystemRoot, 'System32');
    env.LOCALAPPDATA = localData;
    function launch(input) {
        const result = spawnSync(process.env.ComSpec, ['/d', '/s', '/c', `""${join(app, 'start-task-manager.cmd')}""`], {
            env, cwd: scratch, input, encoding: 'utf8', windowsHide: true,
            windowsVerbatimArguments: true, timeout: 30000,
        });
        assert.ifError(result.error);
        assert.equal(result.signal, null);
        return result;
    }
    const first = launch('add\nLauncher persistence\nlist\nquit\n');
    assert.equal(first.status, 0, first.stderr + first.stdout);
    assert.match(first.stdout, /1 \| open \| Launcher persistence/);
    const state = join(localData, 'Sovra', 'Task Manager', 'tasks.txt');
    assert.ok(existsSync(state));
    const next = launch('list\nquit\n');
    assert.equal(next.status, 0, next.stderr + next.stdout);
    assert.match(next.stdout, /1 \| open \| Launcher persistence/);
    writeFileSync(state, 'invalid persisted data');
    const bad = launch('\n');
    assert.equal(bad.status, 1, bad.stderr + bad.stdout);
    assert.match(bad.stdout, /error: invalid task file/);
    assert.equal(readFileSync(state, 'utf8'), 'invalid persisted data');
    console.log('PASS native launcher: spaced paths, restricted PATH, persistence, restart and visible failure');
} finally {
    rmSync(scratch, { recursive: true, force: true });
}
