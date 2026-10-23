// Developer-host qualification. Does not install tools or change rustup defaults.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, mkdtempSync, readFileSync, readdirSync, writeFileSync, existsSync } from 'node:fs';
import { setTimeout as pause } from 'node:timers/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

assert.equal(process.platform, 'win32', 'MSVC qualification requires Windows');
const args = process.argv.slice(2);
assert.ok(args.length === 0 || (args.length === 1 && args[0] === '--wait-for-tools'),
    'Usage: node scripts/test-msvc.mjs [--wait-for-tools]');
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const output = join(root, 'target/msvc-qualification');
const toolchain = 'stable-x86_64-pc-windows-msvc';
const compiler = join(output, 'debug/svr.exe');
const locator = join(process.env['ProgramFiles(x86)'] ?? '', 'Microsoft Visual Studio/Installer/vswhere.exe');
mkdirSync(output, { recursive: true });
const report = {
    schema_version: 1, kind: 'local-msvc-qualification', started_at: new Date().toISOString(),
    status: args[0] === '--wait-for-tools' ? 'waiting' : 'running',
    toolchain, success: false, rust_tests: null, compiler_sha256: null, stages: [],
    hosted_ci: false, clean_machine_qualified: false,
};
writeFileSync(join(output, 'verification.json'), JSON.stringify(report, null, 2) + '\n');
async function waitForTools() {
    const deadline = Date.now() + 60 * 60 * 1000;
    let inactiveChecks = 0;
    while (Date.now() < deadline) {
        const found = existsSync(locator) ? spawnSync(locator, ['-products', '*', '-find',
            'VC\\Tools\\MSVC\\**\\bin\\Hostx64\\x64\\link.exe'], {
            encoding: 'utf8', windowsHide: true, timeout: 10000,
        }) : null;
        const linker = found?.status === 0 && found.stdout.trim().split(/\r?\n/).some(path => existsSync(path));
        const kits = join(process.env['ProgramFiles(x86)'] ?? '', 'Windows Kits/10/Lib');
        const sdk = existsSync(kits) && readdirSync(kits, { withFileTypes: true }).some(entry =>
            entry.isDirectory() && existsSync(join(kits, entry.name, 'um/x64/kernel32.lib'))
            && existsSync(join(kits, entry.name, 'ucrt/x64/ucrt.lib')));
        const tasks = spawnSync('tasklist.exe', ['/FI', 'IMAGENAME eq setup.exe', '/FO', 'CSV', '/NH'], {
            encoding: 'utf8', windowsHide: true, timeout: 10000,
        });
        assert.ifError(tasks.error);
        assert.equal(tasks.status, 0, 'Cannot determine installer state safely');
        const installer = /^"setup\.exe",/im.test(tasks.stdout);
        const readiness = { checked_at: new Date().toISOString(), linker: Boolean(linker),
            sdk: Boolean(sdk), installer_running: installer, ready: Boolean(linker && sdk && !installer) };
        writeFileSync(join(output, 'readiness.json'), JSON.stringify(readiness, null, 2) + '\n');
        if (readiness.ready) return;
        inactiveChecks = installer ? 0 : inactiveChecks + 1;
        assert.ok(inactiveChecks < 2, 'Installer exited without usable MSVC linker/SDK; inspect installer logs');
        console.log(`Waiting for MSVC: linker=${Boolean(linker)}, SDK=${Boolean(sdk)}, installer=${installer}`);
        await pause(20000);
    }
    throw new Error('MSVC readiness wait exceeded one hour; no installation or process was stopped');
}
function run(name, binary, args) {
    console.log(`MSVC: ${name}`);
    const result = spawnSync(binary, args, {
        cwd: root, encoding: 'utf8', timeout: 600000, maxBuffer: 32 * 1024 * 1024,
        windowsHide: true,
        env: { ...process.env, CARGO_TARGET_DIR: output, SOVRA_REQUIRE_CLI_EXECUTION: '1',
            RUSTUP_AUTO_INSTALL: '0', RUSTUP_TOOLCHAIN: toolchain },
    });
    const log = `${result.stdout ?? ''}${result.stderr ?? ''}`;
    writeFileSync(join(output, `${name}.log`), log);
    const stage = { name, exit_code: result.status, signal: result.signal, error: result.error?.message ?? null };
    report.stages.push(stage);
    assert.ifError(result.error);
    assert.equal(result.signal, null, `${name}: terminated`);
    assert.equal(result.status, 0, `${name} failed; inspect target/msvc-qualification/${name}.log\n${log.slice(-4000)}`);
    return log;
}
try {
    if (args[0] === '--wait-for-tools') await waitForTools();
    report.status = 'running';
    writeFileSync(join(output, 'verification.json'), JSON.stringify(report, null, 2) + '\n');
    assert.ok(existsSync(locator), 'Visual Studio C++ Build Tools are not installed');
    const instances = JSON.parse(run('visual-studio', locator, [
        '-products', '*', '-requires', 'Microsoft.VisualStudio.Component.VC.Tools.x86.x64', '-format', 'json',
    ]));
    assert.ok(instances.some(instance => instance.isComplete && instance.isLaunchable),
        'No complete Visual Studio C++ installation; finish component installation/repair first');
    report.visual_studio = instances.map(({ installationVersion, installationPath, isComplete }) => ({
        installationVersion, installationPath, isComplete,
    }));
    const linkers = run('msvc-linker', locator, [
        '-products', '*', '-requires', 'Microsoft.VisualStudio.Component.VC.Tools.x86.x64',
        '-find', 'VC\\Tools\\MSVC\\**\\bin\\Hostx64\\x64\\link.exe',
    ]).trim().split(/\r?\n/).filter(Boolean);
    assert.ok(linkers.some(path => existsSync(path)),
        'Visual Studio is registered but its x64 MSVC linker is missing; finish installation/repair');
    report.linkers = linkers;
    const rust = run('rust-version', 'rustc.exe', [`+${toolchain}`, '-vV']);
    assert.match(rust, /^host: x86_64-pc-windows-msvc\r?$/m);
    report.rust_version = rust.trim();
    const tests = run('rust-tests', 'cargo.exe', [`+${toolchain}`, 'test', '--locked', '--all-targets']);
    const totals = [...tests.matchAll(/test result: ok\. (\d+) passed; 0 failed; (\d+) ignored/g)];
    assert.ok(totals.length >= 3, 'Expected library, CLI and example test summaries');
    assert.ok(totals.every(match => Number(match[2]) === 0), 'Ignored tests do not qualify a host');
    report.rust_tests = totals.reduce((sum, match) => sum + Number(match[1]), 0);
    run('documentation-tests', 'cargo.exe', [`+${toolchain}`, 'test', '--locked', '--doc']);
    run('build', 'cargo.exe', [`+${toolchain}`, 'build', '--locked']);
    report.compiler_sha256 = createHash('sha256').update(readFileSync(compiler)).digest('hex');
    run('formatter', process.execPath, ['scripts/test-formatter.mjs', '--compiler', compiler]);
    run('dogfood', process.execPath, ['scripts/test-dogfood.mjs', '--compiler', compiler]);
    run('depth', process.execPath, ['scripts/probe-compiler-depth.mjs', compiler, '--assert-limits']);
    const depth = JSON.parse(readFileSync(join(root, 'target/depth-probe/results-debug.json'), 'utf8'));
    assert.equal(depth.compiler_sha256, report.compiler_sha256);
    assert.equal(depth.success, true);
    report.depth_cases = depth.total;
    writeFileSync(join(output, 'depth-results.json'), JSON.stringify(depth, null, 2) + '\n');
    const proof = mkdtempSync(join(output, 'native-proof-'));
    const native = join(proof, 'hello.exe');
    run('native-build', compiler, ['build', '--emit', 'exe', '--output', native,
        'toolchain/templates/console/main.svr']);
    const greeting = run('native-run', native, []);
    assert.equal(greeting.trim(), 'Hello, Sovra!');
    report.native_executable = native;
    report.native_sha256 = createHash('sha256').update(readFileSync(native)).digest('hex');
    report.success = true;
    console.log(`PASS local MSVC: ${report.rust_tests} Rust tests and ${report.depth_cases} depth cases.`);
} catch (error) {
    report.failure = String(error);
    console.error(report.failure);
    process.exitCode = 1;
} finally {
    if (existsSync(compiler) && report.compiler_sha256 === null) {
        report.compiler_sha256 = createHash('sha256').update(readFileSync(compiler)).digest('hex');
    }
    report.finished_at = new Date().toISOString();
    report.status = report.success ? 'passed' : 'failed';
    writeFileSync(join(output, 'verification.json'), JSON.stringify(report, null, 2) + '\n');
}
