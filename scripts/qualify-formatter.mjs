// Retain platform-specific formatter evidence; no skipped command can qualify writes.
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { arch, release } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
let compiler = join(root, 'target/debug', process.platform === 'win32' ? 'svr.exe' : 'svr');
let output = join(root, 'target/formatter', `${process.platform}-${arch()}-verification.json`);
const seen = new Set();
for (let i = 0; i < args.length; i += 2) {
    const key = args[i], value = args[i + 1];
    if (!['--compiler', '--output'].includes(key) || !value || seen.has(key)) {
        console.error('Usage: node scripts/qualify-formatter.mjs [--compiler PATH] [--output REPORT.json]');
        process.exit(2);
    }
    seen.add(key);
    if (key === '--compiler') compiler = resolve(value);
    else output = resolve(value);
}
const hash = path => createHash('sha256').update(readFileSync(path)).digest('hex');
const inputs = [
    'Cargo.toml', 'Cargo.lock', 'src/formatter.rs', 'src/formatter/files.rs',
    'src/formatter/layout.rs', 'src/formatter/equivalence.rs', 'src/formatter/packages.rs',
    'src/compiler/project/packages.rs', 'src/compiler/project/packages/executable.rs',
    'scripts/qualify-formatter.mjs', 'scripts/test-formatter-qualification.mjs',
    'scripts/test-formatter.mjs', 'scripts/test-formatter-packages.mjs',
];
const report = {
    schema_version: 1, started_at: new Date().toISOString(), success: false,
    platform: process.platform, architecture: arch(), os_release: release(), node: process.version,
    compiler: { path: compiler }, source_sha256: {}, steps: [], errors: [],
    writes_enabled: ['win32', 'linux'].includes(process.platform),
    write_qualification: 'not_verified',
    scope: 'Formatter behavior on this exact host/compiler; no release, MSRV, desktop or security certification',
};
mkdirSync(dirname(output), { recursive: true });
const logs = `${output}.logs`;
mkdirSync(logs, { recursive: true });
function run(name, binary, argv) {
    const result = spawnSync(binary, argv, {
        cwd: root, encoding: 'utf8', timeout: 180000, maxBuffer: 16 * 1024 * 1024,
        windowsHide: true, env: { ...process.env, SOVRA_REQUIRE_CLI_EXECUTION: '1', CARGO_TERM_COLOR: 'never' },
    });
    const path = join(logs, `${name}.log`);
    writeFileSync(path, `${result.stdout ?? ''}${result.stderr ?? ''}${result.error ? `\n${result.error.message}\n` : ''}`);
    const step = { name, command: [binary, ...argv], exit_code: result.status, signal: result.signal,
        success: !result.error && !result.signal && result.status === 0, log: path, log_sha256: hash(path) };
    report.steps.push(step);
    return step.success;
}
try {
    report.compiler.sha256 = hash(compiler);
    for (const path of inputs) report.source_sha256[path] = hash(join(root, path));
    const compilerVersion = run('compiler-version', compiler, ['--version']);
    const rustVersion = run('rust-version', 'rustc', ['--version']);
    const unit = run('replacement-regressions', 'cargo', ['test', '--locked', '--lib', 'formatter', '--', '--nocapture']);
    const standalone = run('standalone-import-execution', process.execPath, [join(root, 'scripts/test-formatter.mjs'), '--compiler', compiler]);
    const packages = run('package-checks-writes', process.execPath, [join(root, 'scripts/test-formatter-packages.mjs'), '--compiler', compiler]);
    if (hash(compiler) !== report.compiler.sha256) report.errors.push('Compiler changed during qualification');
    for (const path of inputs) {
        if (hash(join(root, path)) !== report.source_sha256[path]) report.errors.push(`Source changed during qualification: ${path}`);
    }
    report.success = compilerVersion && rustVersion && unit && standalone && packages && report.errors.length === 0;
    report.write_qualification = report.success
        ? (report.writes_enabled ? 'verified_on_this_host' : 'disabled_policy_verified') : 'not_verified';
} catch (error) {
    report.errors.push(error.message);
} finally {
    report.finished_at = new Date().toISOString();
    writeFileSync(output, `${JSON.stringify(report, null, 2)}\n`);
}
console.log(`Formatter qualification ${report.success ? 'passed' : 'failed'}: ${output}`);
process.exitCode = report.success ? 0 : 1;
