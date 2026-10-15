// Public language test command: real execution and schema-one subprocess reports.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
assert.ok(args.length === 0 || (args.length === 2 && args[0] === '--compiler'),
    'Usage: node scripts/test-language-tests.mjs [--compiler PATH]');
const compiler = args.length ? resolve(args[1]) : join(root, 'target', 'debug', process.platform === 'win32' ? 'svr.exe' : 'svr');
const scratch = mkdtempSync(join(tmpdir(), 'sovra-language-tests-'));
function execute(args, expectedStatus) {
    const result = spawnSync(compiler, args, { cwd: scratch, encoding: 'utf8', timeout: 30000, windowsHide: true });
    assert.ifError(result.error); assert.equal(result.signal, null);
    assert.equal(result.status, expectedStatus, result.stderr + result.stdout);
    return result;
}
function report(path, expectedStatus) {
    const result = execute(['test', '--format', 'json', path], expectedStatus);
    assert.equal(result.stderr, '');
    const parsed = JSON.parse(result.stdout);
    assert.equal(parsed.schema_version, 1);
    assert.equal(parsed.success, expectedStatus === 0);
    assert.equal(parsed.total, parsed.tests.length);
    assert.equal(parsed.passed + parsed.failed, parsed.total);
    return parsed;
}
try {
    const source = join(scratch, 'suite.svr');
    writeFileSync(source, 'fn main() { std::set_exit_code(1); } type Done = Unit; fn test_pass() -> Done { print("line\\nquote\\\"tail"); } fn test_exit() { std::set_exit_code(3); } fn test_error() { print("before error"); let zero = 0; print(1 / zero); } fn test_after() { print(std::arg_count()); } fn test_stdin() { std::read_line(); }');
    const mixed = report(source, 1);
    assert.equal(mixed.passed, 2); assert.equal(mixed.failed, 3);
    assert.equal(mixed.tests[0].output[0], 'line\nquote"tail');
    assert.equal(mixed.tests[1].exit_code, 3);
    assert.equal(mixed.tests[2].exit_code, null);
    assert.deepEqual(mixed.tests[2].output, ['before error']);
    assert.match(mixed.tests[2].error, /division by zero/);
    assert.equal(mixed.tests[3].passed, true);
    assert.deepEqual(mixed.tests[3].output, ['0']);
    assert.match(mixed.tests[4].error, /stdin|standard input/);
    const human = execute(['test', source], 1).stdout;
    assert.ok(human.includes('PASS test_pass'));
    assert.ok(human.includes('line\\u{000A}quote"tail'));
    assert.ok(human.includes('2 passed; 3 failed; 5 total'));
    const marker = join(scratch, 'must-not-exist.txt');
    writeFileSync(source, `fn test_effect() { std::write_text(${JSON.stringify(marker.replaceAll('\\', '/'))}, "changed"); } fn test_bad(value: Int) {}`);
    const invalid = report(source, 1);
    assert.equal(invalid.total, 0); assert.equal(invalid.diagnostics[0].code, 'E5001');
    assert.ok(invalid.diagnostics[0].source_file); assert.ok(invalid.diagnostics[0].span.end > 0);
    assert.ok(!existsSync(marker));
    writeFileSync(source, 'fn main() {}');
    assert.equal(report(source, 1).diagnostics[0].code, 'E5002');
    writeFileSync(source, 'fn test_bad() { let broken = }');
    assert.match(report(source, 1).diagnostics[0].code, /^E200/);
    for (const command of [[], ['--format', 'yaml', source], ['--format', 'json', '--format', 'human', source],
        ['--unknown', source], [source, source]]) execute(['test', ...command], 2);
    const project = join(scratch, 'hello');
    execute(['new', project], 0);
    assert.equal(report(project, 0).tests[0].name, 'test_greeting');
    execute(['run', project], 0);
    for (const application of ['sovra-budget', 'sovra-notes']) {
        const packageReport = report(join(root, 'dogfood', application, 'tests'), 0);
        assert.equal(packageReport.kind, 'package'); assert.equal(packageReport.total, 3);
    }
    console.log('PASS language tests: JSON/human reports, isolation, preflight errors, template and reusable packages');
} finally { rmSync(scratch, { recursive: true, force: true }); }
