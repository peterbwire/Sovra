// Local process-level observations; no speed/security thresholds or ranking.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync, mkdirSync, mkdtempSync, rmSync } from 'node:fs';
import { cpus, totalmem, platform, release, arch, tmpdir } from 'node:os';
import { dirname, resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const options = new Map();
const args = process.argv.slice(2);
assert.equal(args.length % 2, 0, 'Options require values');
for (let i = 0; i < args.length; i += 2) {
    assert.ok(['--compiler', '--samples', '--report', '--python'].includes(args[i]) && !options.has(args[i]), 'Unknown/duplicate option');
    options.set(args[i], args[i + 1]);
}
const compiler = resolve(options.get('--compiler') || join(root, 'target/release', platform() === 'win32' ? 'svr.exe' : 'svr'));
const python = options.get('--python') || (platform() === 'win32' ? 'python' : 'python3');
const count = Number(options.get('--samples') || 5);
assert.ok(Number.isInteger(count) && count >= 1 && count <= 30, 'Samples must be 1..30');
const reportPath = resolve(options.get('--report') || join(root, 'target/benchmarks/compiler-runtime.json'));
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const command = (binary, argv) => {
    const result = spawnSync(binary, argv, { cwd: root, encoding: 'utf8', timeout: 40000, maxBuffer: 8 * 1024 * 1024, windowsHide: true });
    assert.ifError(result.error);
    assert.equal(result.status, 0, result.stderr);
    return result.stdout;
};
const optional = (binary, argv) => { try { return command(binary, argv).trim(); } catch { return null; } };
const manifestPath = join(root, 'benchmarks/compiler-runtime/corpus.json');
const manifestBytes = readFileSync(manifestPath);
const corpus = JSON.parse(manifestBytes);
assert.equal(corpus.schema, 1);
const work = mkdtempSync(join(tmpdir(), 'sovra-language-bench-'));
const measurements = [];
const report = {
    schema: 1, corpus_version: corpus.version, recorded_utc: new Date().toISOString(),
    success: false, samples_per_operation: count, warmups_per_operation: 1,
    machine: { os: platform(), release: release(), architecture: arch(), cpu: cpus()[0]?.model, logical_cpus: cpus().length, total_memory_bytes: totalmem() },
    compiler: { path: compiler, sha256: hash(readFileSync(compiler)), version: command(compiler, ['--version']).trim(), build_flags: 'not inferred; record build command separately' },
    tools: { node: process.version, python: command(python, ['--version']).trim(), rustc_available: optional('rustc', ['--version']), cargo_available: optional('cargo', ['--version']) },
    source_revision: optional('git', ['rev-parse', 'HEAD']), working_tree_status: optional('git', ['status', '--porcelain']),
    corpus_sha256: hash(manifestBytes), runner_sha256: hash(readFileSync(fileURLToPath(import.meta.url))),
    process_helper_sha256: hash(readFileSync(join(root, 'scripts/benchmark-process.py'))),
    memory_method: ['linux', 'darwin'].includes(platform()) ? 'POSIX RUSAGE_CHILDREN maximum resident set for one fresh direct child (not aggregate process-tree memory)' : 'unavailable; null values',
    limits: 'Warm process/filesystem observations including child startup. Check/build are whole CLI operations, not isolated compiler stages. Interpreter run includes checking. No cold-start, native backend, browser, mobile or performance threshold claim.',
    measurements,
};
function measure(id, operation, binary, argv, verify, input) {
    const samples = [];
    for (let i = -1; i < count; i++) {
        const result = JSON.parse(command(python, [join(root, 'scripts/benchmark-process.py'), binary, ...argv]));
        assert.equal(result.status, 0, `${id}/${operation}: ${result.stderr}`);
        assert.equal(result.stderr, '');
        verify(result.stdout.replaceAll('\r\n', '\n'));
        assert.ok(Number.isFinite(result.wall_ms) && result.wall_ms >= 0);
        assert.ok(result.peak_rss_bytes === null || result.peak_rss_bytes > 0);
        if (i >= 0) samples.push({ wall_ms: result.wall_ms, peak_rss_bytes: result.peak_rss_bytes });
    }
    const sorted = samples.map(value => value.wall_ms).sort((a, b) => a - b);
    const middle = Math.floor(sorted.length / 2);
    const median = sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
    measurements.push({ case: id, operation, input_sha256: hash(input), input_bytes: Buffer.byteLength(input), samples, min_ms: sorted[0], median_ms: median, max_ms: sorted.at(-1) });
    console.log(`${id}/${operation}: median ${median.toFixed(2)} ms`);
}
try {
    measure('process', 'compiler-version-startup', compiler, ['--version'], out => assert.equal(out.trim(), report.compiler.version), '');
    for (const item of corpus.cases) {
        const source = item.source ? readFileSync(join(root, 'benchmarks/compiler-runtime', item.source), 'utf8') :
            Array.from({ length: item.declarations }, (_, i) => `fn value_${i}(x: Int) -> Int { return x + ${i} }\n`).join('') + `fn main() { print(value_${item.declarations - 1}(1)) }\n`;
        const path = join(work, `${item.id}.svr`);
        writeFileSync(path, source);
        const verifyCheck = out => { const result = JSON.parse(out); assert.equal(result.success, true); assert.deepEqual(result.diagnostics, []); };
        measure(item.id, 'check-json', compiler, ['check', '--format', 'json', path], verifyCheck, source);
        const js = command(compiler, ['build', '--emit', 'js', path]);
        measure(item.id, 'build-js', compiler, ['build', '--emit', 'js', path], out => assert.equal(out, js.replaceAll('\r\n', '\n')), source);
        const generated = join(work, `${item.id}.cjs`);
        writeFileSync(generated, js);
        measure(item.id, 'interpreter-run', compiler, ['run', path], out => assert.equal(out, item.stdout), source);
        measure(item.id, 'node-generated-js', process.execPath, [generated], out => assert.equal(out, item.stdout), source);
        measurements.at(-1).generated_js_bytes = Buffer.byteLength(js);
        measurements.at(-1).generated_js_sha256 = hash(js);
    }
    report.success = true;
} catch (error) {
    report.failure = error.message;
    throw error;
} finally {
    rmSync(work, { recursive: true, force: true });
    mkdirSync(dirname(reportPath), { recursive: true });
    writeFileSync(reportPath, JSON.stringify(report, null, 2) + '\n');
}
