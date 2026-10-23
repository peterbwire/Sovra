// Build against a relocated runtime snapshot, then discard all build dependencies.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, renameSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

assert.equal(process.platform, 'win32', 'Runtime payload packaging currently targets Windows');
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const scratch = mkdtempSync(join(tmpdir(), 'sovra-runtime-payload-'));
function execute(binary, args, timeout = 30000, env = process.env) {
    const result = spawnSync(binary, args, {
        cwd: scratch, env, timeout, encoding: 'utf8', windowsHide: true, maxBuffer: 4 * 1024 * 1024,
    });
    assert.ifError(result.error);
    assert.equal(result.signal, null);
    return result;
}
function files(directory, prefix = '') {
    return readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
        assert.ok(!entry.isSymbolicLink(), 'Payload must not contain links');
        const path = prefix + entry.name;
        return entry.isDirectory() ? files(join(directory, entry.name), path + '/') : [path];
    }).sort();
}
try {
    const initial = join(scratch, 'initial');
    const script = join(root, 'scripts', 'build-native-runtime-payload.ps1');
    // Hosted Windows packaging has encountered a host without Get-FileHash.
    // Remove that optional function while retaining the inbox commands used by
    // the builder, then verify every resulting digest independently with Node.
    const quote = value => "'" + value.replaceAll("'", "''") + "'";
    const withoutFileHash = `
$ErrorActionPreference = 'Stop'
Import-Module Microsoft.PowerShell.Utility
Import-Module Microsoft.PowerShell.Management
Remove-Item Function:\\Get-FileHash -ErrorAction SilentlyContinue
$PSModuleAutoLoadingPreference = 'None'
if (Get-Command Get-FileHash -ErrorAction SilentlyContinue) { throw 'Hash regression precondition failed' }
& ${quote(script)} -Output ${quote(initial)}
`;
    const built = execute('powershell.exe', ['-NoProfile', '-ExecutionPolicy', 'Bypass',
        '-EncodedCommand', Buffer.from(withoutFileHash, 'utf16le').toString('base64')]);
    assert.equal(built.status, 0, built.stderr + built.stdout);
    const manifestBytes = readFileSync(join(initial, 'payload.json'));
    const manifest = JSON.parse(manifestBytes);
    assert.equal(manifest.schema, 1);
    assert.equal(manifest.kind, 'sovra-native-runtime-source');
    assert.deepEqual(manifest.files.map(file => file.path).sort(),
        files(initial).filter(path => !['payload.json', 'payload.sha256'].includes(path)));
    for (const file of manifest.files) {
        assert.match(file.sha256, /^[a-f0-9]{64}$/);
        assert.equal(createHash('sha256').update(readFileSync(join(initial, file.path))).digest('hex'), file.sha256);
    }
    const refused = execute('powershell.exe', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File',
        script, '-Output', initial]);
    assert.notEqual(refused.status, 0);
    assert.match(refused.stderr, /Output already exists/);
    assert.deepEqual(readFileSync(join(initial, 'payload.json')), manifestBytes);
    assert.deepEqual(readdirSync(scratch).filter(name => name.startsWith('.sovra-runtime-stage-')), []);

    const payload = join(scratch, 'relocated runtime');
    renameSync(initial, payload);
    const source = join(scratch, 'task-manager.svr');
    copyFileSync(join(root, 'dogfood', '01-task-manager', 'main.svr'), source);
    const build = join(scratch, 'application-build');
    const linked = execute('cargo', ['run', '--offline', '--locked',
        '--manifest-path', join(payload, 'Cargo.toml'), '--target-dir', join(scratch, 'bootstrap'),
        '--example', 'native_builder', '--', source, build], 600000);
    assert.equal(linked.status, 0, linked.stderr + linked.stdout);
    const distribution = join(scratch, 'distribution');
    mkdirSync(distribution);
    const executable = join(distribution, 'task-manager.exe');
    copyFileSync(join(build, 'target', 'release', 'sovra-native-proof.exe'), executable);
    rmSync(payload, { recursive: true });
    rmSync(build, { recursive: true });
    rmSync(join(scratch, 'bootstrap'), { recursive: true });
    rmSync(source);
    assert.ok(existsSync(executable));
    const parity = execute(process.execPath, [join(root, 'scripts', 'test-native-task-manager.mjs'),
        '--native', executable], 120000);
    assert.equal(parity.status, 0, parity.stderr + parity.stdout);
    console.log(parity.stdout.trim());
    console.log('PASS runtime payload: complete hashes, overwrite refusal, relocation, offline linking and source-independent application');
} finally {
    rmSync(scratch, { recursive: true, force: true });
}
