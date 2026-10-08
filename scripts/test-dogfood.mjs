// Execute real Sovra programs through the repository compiler and both backends.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync, mkdtempSync } from 'node:fs';
import { dirname, join, resolve, relative, isAbsolute } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
if (args.length !== 0 && (args.length !== 2 || args[0] !== '--compiler')) {
    throw new Error('Usage: node scripts/test-dogfood.mjs [--compiler PATH]');
}
const compiler = args.length === 2 ? resolve(args[1])
    : join(root, 'target', 'debug', process.platform === 'win32' ? 'svr.exe' : 'svr');
const base = join(root, 'dogfood');
const output = join(root, 'target', 'dogfood');
mkdirSync(output, { recursive: true });
const artifacts = mkdtempSync(join(output, 'run-'));
const report = { schema_version: 1, compiler, platform: process.platform, applications: [], success: false };
const normalize = text => text.replaceAll('\r\n', '\n');

function insideBase(path) {
    assert.equal(typeof path, 'string');
    const absolute = resolve(base, path);
    const rel = relative(base, absolute);
    assert.ok(rel !== '' && rel !== '..' && !rel.startsWith(`..${process.platform === 'win32' ? '\\' : '/'}`) && !isAbsolute(rel), 'suite path must stay inside dogfood');
    return absolute;
}

function run(executable, arguments_, input = '') {
    const result = spawnSync(executable, arguments_, {
        cwd: root, encoding: 'utf8', timeout: 30000,
        maxBuffer: 4 * 1024 * 1024, windowsHide: true, input,
    });
    assert.ifError(result.error);
    assert.equal(result.signal, null, `terminated by ${result.signal}`);
    assert.equal(result.status, 0, `${executable} ${arguments_.join(' ')}\n${result.stderr}\n${result.stdout}`);
    assert.equal(result.stderr, '', `unexpected stderr: ${result.stderr}`);
    return normalize(result.stdout);
}

try {
    report.compiler_version = run(compiler, ['--version']).trim();
    const suite = JSON.parse(readFileSync(join(base, 'suite.json'), 'utf8'));
    assert.equal(suite.schema_version, 1);
    assert.ok(Array.isArray(suite.applications) && suite.applications.length > 0, 'empty dogfood suite');
    const seen = new Set();
    for (const application of suite.applications) {
        assert.match(application.id, /^[a-z0-9-]+$/);
        assert.ok(!seen.has(application.id), 'duplicate application id');
        seen.add(application.id);
        const result = { id: application.id, passed: false, stages: [] };
        report.applications.push(result);
        try {
            const source = insideBase(application.source);
            const check = JSON.parse(run(compiler, ['check', '--format', 'json', source]));
            assert.equal(check.success, true);
            result.stages.push('source-check');
            const ir = run(compiler, ['build', '--emit', 'ir', source]);
            assert.ok(ir.trim().length > 0, 'empty IR');
            writeFileSync(join(artifacts, `${application.id}.ir`), ir);
            result.stages.push('ir-build');
            const javascript = run(compiler, ['build', '--emit', 'js', source]);
            const file = join(artifacts, `${application.id}.js`);
            writeFileSync(file, javascript);
            result.stages.push('javascript-build');
            assert.ok(Array.isArray(application.cases) && application.cases.length > 0,
                'application has no behavioral cases');
            result.cases = [];
            const seenCases = new Set();
            for (const testCase of application.cases) {
                assert.match(testCase.id, /^[a-z0-9-]+$/);
                assert.ok(!seenCases.has(testCase.id), 'duplicate case id');
                seenCases.add(testCase.id);
                const expected = normalize(readFileSync(insideBase(testCase.stdout), 'utf8'));
                const input = testCase.stdin === undefined ? ''
                    : readFileSync(insideBase(testCase.stdin), 'utf8');
                const args = testCase.args ?? [];
                assert.ok(Array.isArray(args) && args.every(arg => typeof arg === 'string'));
                const programArgs = args.length ? ['--', ...args] : [];
                assert.equal(run(compiler, ['run', source, ...programArgs], input), expected,
                    `interpreter behavior differs in ${testCase.id}`);
                assert.equal(run(process.execPath, [file, ...args], input), expected,
                    `JavaScript behavior differs in ${testCase.id}`);
                result.cases.push({ id: testCase.id, passed: true });
                console.log(`PASS ${application.id}/${testCase.id}`);
            }
            result.stages.push('interpreter-and-javascript-behavior');
            result.passed = true;
            console.log(`PASS ${application.id}: ${result.stages.join(', ')}`);
        } catch (error) {
            result.error = error.stack ?? String(error);
            console.error(`FAIL ${application.id}: ${result.error}`);
        }
    }
    report.success = report.applications.every(application => application.passed);
} catch (error) {
    report.error = error.stack ?? String(error);
    console.error(report.error);
} finally {
    writeFileSync(join(artifacts, 'report.json'), JSON.stringify(report, null, 2) + '\n');
    console.log(`Dogfood report: ${join(artifacts, 'report.json')}`);
    if (!report.success) process.exitCode = 1;
}
