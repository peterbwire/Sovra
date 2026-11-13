import assert from 'node:assert/strict';
import { readFileSync, readdirSync, existsSync, mkdtempSync, writeFileSync, mkdirSync, rmSync } from 'node:fs';
import { resolve, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { tmpdir } from 'node:os';

const root = fileURLToPath(new URL('..', import.meta.url));
const examples = join(root, 'examples');
const compiler = join(root, 'target/debug', process.platform === 'win32' ? 'svr.exe' : 'svr');
const catalogPath = join(examples, 'catalog.json');
const catalog = JSON.parse(readFileSync(catalogPath, 'utf8'));
assert.equal(catalog.schema_version, 1);
const directories = readdirSync(examples, { withFileTypes: true }).filter(entry => entry.isDirectory()).map(entry => entry.name).sort();
assert.deepEqual(catalog.examples.map(entry => entry.directory).sort(), directories, 'Every example directory needs one classification');
assert.deepEqual([...catalog.internal_tools].sort(), readdirSync(examples).filter(name => name.endsWith('.rs')).sort());
for (const tool of catalog.internal_tools) assert.ok(existsSync(join(examples, tool)));
const kinds = new Set(['executable', 'executable-package', 'host-component', 'checker-only', 'proposal']);
const scratch = mkdtempSync(join(tmpdir(), 'sovra-example-catalog-'));
const results = [];
function run(binary, args) {
    const result = spawnSync(binary, args, { cwd: root, encoding: 'utf8', timeout: 15000, maxBuffer: 8 * 1024 * 1024 });
    assert.ifError(result.error); assert.equal(result.status, 0, `${binary} ${args.join(' ')}\n${result.stdout}\n${result.stderr}`);
    return result.stdout.replaceAll('\r\n', '\n');
}
try {
    for (const entry of catalog.examples) {
        assert.match(entry.directory, /^[a-z][a-z0-9-]*$/); assert.ok(kinds.has(entry.kind));
        const directory = join(examples, entry.directory);
        const target = entry.entry ? resolve(directory, entry.entry) : directory;
        assert.ok(!relative(directory, target).startsWith('..') && existsSync(target));
        const args = ['check', '--format', 'json'];
        if (entry.kind === 'checker-only') args.push('--service-calls');
        const report = JSON.parse(run(compiler, [...args, target]));
        assert.equal(report.success, true);
        const result = { directory: entry.directory, kind: entry.kind, checked: true, executed: false };
        if (['executable', 'executable-package', 'host-component'].includes(entry.kind)) {
            const state = join(scratch, `${entry.directory}.txt`);
            const programArgs = entry.kind === 'host-component' ? ['ui-layout-view', state] : [];
            const interpreted = run(compiler, ['run', target, ...(programArgs.length ? ['--', ...programArgs] : [])]);
            const javascript = join(scratch, `${entry.directory}.cjs`);
            writeFileSync(javascript, run(compiler, ['build', '--emit', 'js', target]), { mode: 0o600 });
            assert.equal(run(process.execPath, [javascript, ...programArgs]), interpreted);
            if (entry.kind === 'host-component') assert.equal(existsSync(state), false);
            result.executed = true; result.engines = ['interpreter', 'node-javascript'];
        }
        results.push(result);
    }
    const output = join(root, 'target/docs'); mkdirSync(output, { recursive: true });
    writeFileSync(join(output, 'examples-verification.json'), JSON.stringify({ schema_version: 1, success: true, compiler_sha256: createHash('sha256').update(readFileSync(compiler)).digest('hex'), catalog_sha256: createHash('sha256').update(readFileSync(catalogPath)).digest('hex'), native_ui_executed: false, proposal_execution_attempted: false, examples: results }, null, 2) + '\n');
    console.log(`PASS example catalog: ${results.length} classified directories, executable backend parity, checker-only coverage and proposal wiring check. No proposal/service/native-UI execution claimed.`);
} finally { rmSync(scratch, { recursive: true, force: true }); }
