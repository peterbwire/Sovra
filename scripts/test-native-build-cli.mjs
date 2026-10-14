// ADR 0027 acceptance through an installed toolchain outside the checkout.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { copyFileSync, cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

assert.equal(process.platform, 'win32', 'Installed native CLI test currently targets Windows');
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const scratch = mkdtempSync(join(tmpdir(), 'sovra-installed-native-'));
const args = process.argv.slice(2);
assert.ok(args.length === 0 || (args.length === 2 && args[0] === '--compiler'),
    'Usage: node scripts/test-native-build-cli.mjs [--compiler PATH]');
const compiler = args.length ? resolve(args[1]) : join(root, 'target', 'debug', 'svr.exe');
function execute(binary, args, cwd = scratch, env = process.env, timeout = 30000) {
    const result = spawnSync(binary, args, {
        cwd, env, timeout, encoding: 'utf8', windowsHide: true, maxBuffer: 8 * 1024 * 1024,
    });
    assert.ifError(result.error);
    assert.equal(result.signal, null);
    return result;
}
function success(result) { assert.equal(result.status, 0, result.stderr + result.stdout); return result; }
function powershell(script, args, cwd = scratch) {
    return execute('powershell.exe', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', script, ...args],
        cwd, process.env, 120000);
}
try {
    const packageRoot = join(scratch, 'package');
    success(powershell(join(root, 'scripts', 'build-windows-toolchain.ps1'),
        ['-Binary', compiler, '-Output', packageRoot], root));
    const installed = join(scratch, 'installed');
    success(powershell(join(packageRoot, 'install.ps1'), ['-Prefix', installed, '-NoPersistPath']));
    const svr = join(installed, 'bin', 'svr.exe');
    const project = join(scratch, 'console');
    success(execute(svr, ['new', project]));
    const source = join(project, 'main.svr');
    const output = join(scratch, 'console.exe');
    const command = ['build', '--emit', 'exe', '--output', output, source];
    function fails(pattern, env = process.env, args = command, status = 1) {
        const result = execute(svr, args, scratch, env);
        assert.equal(result.status, status, result.stderr + result.stdout);
        assert.match(result.stderr, pattern);
        assert.equal(result.stdout, '');
        assert.deepEqual(readdirSync(scratch).filter(name => name.startsWith('.sovra-exe-stage-')), []);
        return result;
    }
    writeFileSync(output, 'keep');
    fails(/output already exists/);
    assert.equal(readFileSync(output, 'utf8'), 'keep');
    rmSync(output);
    const payload = join(installed, 'runtime-source');
    renameSync(payload, payload + '-hidden');
    fails(/payload is missing/);
    renameSync(payload + '-hidden', payload);
    const library = join(payload, 'src', 'lib.rs');
    const original = readFileSync(library);
    writeFileSync(library, 'modified');
    fails(/hash mismatch/);
    writeFileSync(library, original);
    writeFileSync(join(payload, 'unexpected'), 'extra');
    fails(/file list mismatch/);
    rmSync(join(payload, 'unexpected'));
    const manifest = join(payload, 'payload.sha256');
    const manifestBytes = readFileSync(manifest);
    writeFileSync(manifest, manifestBytes.toString().replace('SVR-RUNTIME 1', 'SVR-RUNTIME 2'));
    fails(/version-mismatched/);
    writeFileSync(manifest, manifestBytes);
    const noTools = { ...process.env };
    for (const key of Object.keys(noTools)) if (key.toLowerCase() === 'path') delete noTools[key];
    const systemRoot = process.env.SystemRoot || 'C:\\Windows';
    noTools.PATH = `${join(systemRoot, 'System32')};${systemRoot}`;
    fails(/cannot launch Cargo/, noTools);
    assert.ok(!existsSync(output));
    fails(/must end in .exe/, process.env,
        ['build', '--emit', 'exe', '--output', join(scratch, 'wrong.txt'), source], 2);
    const invalid = join(scratch, 'invalid.svr');
    writeFileSync(invalid, 'fn main() { let broken = }');
    fails(/error\[E200/, process.env, ['build', '--emit', 'exe', '--output', output, invalid]);

    const built = success(execute(svr, command, scratch, process.env, 600000));
    assert.ok(built.stdout.trim().endsWith('console.exe'));
    const result = success(execute(output, [], scratch, noTools));
    assert.equal(result.stdout.replaceAll('\r\n', '\n'), 'Hello, Sovra!\n');
    const taskSource = join(scratch, 'task-manager.svr');
    copyFileSync(join(root, 'dogfood', '01-task-manager', 'main.svr'), taskSource);
    const taskExecutable = join(scratch, 'task-manager.exe');
    success(execute(svr, ['build', '--emit=exe', '--output=' + taskExecutable, taskSource],
        scratch, process.env, 600000));
    const packages = join(scratch, 'local packages');
    cpSync(join(root, 'examples', 'local-packages'), packages, { recursive: true });
    const packageExecutable = join(scratch, 'consumer.exe');
    success(execute(svr, ['build', '--emit', 'exe', '--output', packageExecutable, join(packages, 'app')],
        scratch, process.env, 600000));
    rmSync(installed, { recursive: true });
    rmSync(packageRoot, { recursive: true });
    rmSync(project, { recursive: true });
    rmSync(packages, { recursive: true });
    rmSync(taskSource);
    assert.equal(success(execute(packageExecutable, [], scratch, noTools)).stdout.replaceAll('\r\n', '\n'),
        '49\n25\n1.5\n');
    assert.equal(success(execute(output, [], scratch, noTools)).stdout.replaceAll('\r\n', '\n'), 'Hello, Sovra!\n');
    const parity = success(execute(process.execPath, [join(root, 'scripts', 'test-native-task-manager.mjs'),
        '--native', taskExecutable], root, process.env, 120000));
    console.log(parity.stdout.trim());
    console.log('PASS installed native CLI: failure isolation, template, Task Manager, packages and compiler-free execution');
} finally {
    rmSync(scratch, { recursive: true, force: true });
}
