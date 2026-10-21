// Developer-host qualification. Does not install tools or change rustup defaults.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync, existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

assert.equal(process.platform, 'win32', 'MSVC qualification requires Windows');
assert.equal(process.argv.length, 2, 'Usage: node scripts/test-msvc.mjs');
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const output = join(root, 'target/msvc-qualification');
const toolchain = 'stable-x86_64-pc-windows-msvc';
const compiler = join(output, 'debug/svr.exe');
mkdirSync(output, { recursive: true });
const report = {
    schema_version: 1, kind: 'local-msvc-qualification', started_at: new Date().toISOString(),
    toolchain, success: false, rust_tests: null, compiler_sha256: null, stages: [],
    hosted_ci: false, clean_machine_qualified: false,
};
function run(name, binary, args) {
    console.log(`MSVC: ${name}`);
    const result = spawnSync(binary, args, {
        cwd: root, encoding: 'utf8', timeout: 600000, maxBuffer: 32 * 1024 * 1024,
        windowsHide: true,
        env: { ...process.env, CARGO_TARGET_DIR: output, SOVRA_REQUIRE_CLI_EXECUTION: '1', RUSTUP_AUTO_INSTALL: '0' },
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
    writeFileSync(join(output, 'verification.json'), JSON.stringify(report, null, 2) + '\n');
}
