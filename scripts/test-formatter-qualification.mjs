// Regression: an unreadable or unrelated compiler must never produce qualification.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const scratch = mkdtempSync(join(tmpdir(), 'sovra formatter evidence '));
try {
    for (const [name, compiler] of [['missing', join(scratch, 'absent-compiler')], ['wrong', process.execPath]]) {
        const output = join(scratch, `${name}.json`);
        const result = spawnSync(process.execPath, [join(root, 'scripts/qualify-formatter.mjs'), '--compiler', compiler, '--output', output], {
            cwd: scratch, encoding: 'utf8', timeout: 180000, maxBuffer: 1024 * 1024,
        });
        assert.ifError(result.error);
        assert.equal(result.signal, null);
        assert.equal(result.status, 1, result.stderr);
        const report = JSON.parse(readFileSync(output, 'utf8'));
        assert.equal(report.schema_version, 1);
        assert.equal(report.success, false);
        assert.equal(report.write_qualification, 'not_verified');
        if (name === 'missing') {
            assert.ok(report.errors.length > 0);
            assert.equal(report.steps.length, 0);
        } else {
            assert.ok(report.steps.some(step => step.name === 'standalone-import-execution' && !step.success));
            assert.ok(report.steps.some(step => step.name === 'package-checks-writes' && !step.success));
            for (const step of report.steps) assert.ok(readFileSync(step.log).length > 0);
        }
    }
    const malformed = spawnSync(process.execPath, [join(root, 'scripts/qualify-formatter.mjs'), '--compiler'], { encoding: 'utf8' });
    assert.equal(malformed.status, 2);
    console.log('Formatter evidence regressions passed: missing/wrong compiler failures remain unqualified, failure reports/logs retained, malformed options rejected.');
} finally { rmSync(scratch, { recursive: true, force: true }); }
