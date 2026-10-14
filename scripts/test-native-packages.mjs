// Exercise general local-package linking in a source-independent native program.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { copyFileSync, cpSync, existsSync, mkdirSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const compiler = join(root, 'target', 'debug', process.platform === 'win32' ? 'svr.exe' : 'svr');
const scratch = mkdtempSync(join(tmpdir(), 'sovra-native-packages-'));

function execute(binary, args, cwd, env = process.env, timeout = 30000) {
    const result = spawnSync(binary, args, {
        cwd, env, encoding: 'utf8', timeout, windowsHide: true, maxBuffer: 4 * 1024 * 1024,
    });
    assert.ifError(result.error);
    assert.equal(result.signal, null, `${binary}: ${result.signal}`);
    return {
        status: result.status,
        stdout: result.stdout.replaceAll('\r\n', '\n'),
        stderr: result.stderr.replaceAll('\r\n', '\n'),
    };
}

try {
    assert.ok(existsSync(compiler), 'Build the debug compiler first');
    const packages = join(scratch, 'local packages');
    cpSync(join(root, 'examples', 'local-packages'), packages, { recursive: true });
    const source = join(packages, 'app');
    assert.ok(existsSync(join(source, 'sovra.toml')), `Copied package missing: ${source}`);
    const interpreted = execute(compiler, ['run', source], scratch);
    assert.deepEqual(interpreted, { status: 0, stdout: '49\n25\n1.5\n', stderr: '' });

    const distribution = join(scratch, 'distribution');
    mkdirSync(distribution);
    const executable = join(distribution, process.platform === 'win32' ? 'consumer.exe' : 'consumer');
    const built = execute(compiler, ['build', '--emit', 'exe', '--output', executable, source],
        root, { ...process.env, CARGO_TARGET_DIR: join(scratch, 'redirected-target') }, 600000);
    assert.equal(built.status, 0, built.stderr + built.stdout);
    // Public build staging is cleaned; remove source dependencies before launching.
    rmSync(packages, { recursive: true });
    const env = { ...process.env };
    for (const key of Object.keys(env)) {
        if (key.toLowerCase() === 'path') delete env[key];
    }
    const systemRoot = process.env.SystemRoot || 'C:\\Windows';
    env.PATH = process.platform === 'win32' ? `${join(systemRoot, 'System32')};${systemRoot}` : '';
    assert.deepEqual(execute(executable, [], distribution, env), interpreted);
    assert.deepEqual(execute(executable, [], scratch, env), interpreted);
    console.log('PASS native packages: private helper, exported nominal record, Float widening, source-free launches');
} finally {
    rmSync(scratch, { recursive: true, force: true });
}
