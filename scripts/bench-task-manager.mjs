// Repeatable local measurements for the repository-owned Task Manager CLI.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';
import { performance } from 'node:perf_hooks';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
const options = new Map();
if (args.length % 2 !== 0) {
    throw new Error('Usage: node scripts/bench-task-manager.mjs [--compiler PATH] [--source PATH]');
}
for (let index = 0; index < args.length; index += 2) {
    if (!['--compiler', '--source'].includes(args[index]) || options.has(args[index])) {
        throw new Error('Usage: node scripts/bench-task-manager.mjs [--compiler PATH] [--source PATH]');
    }
    options.set(args[index], args[index + 1]);
}
const compiler = options.has('--compiler') ? resolve(options.get('--compiler')) :
    join(root, 'target', 'debug', process.platform === 'win32' ? 'svr.exe' : 'svr');
const source = options.has('--source') ? resolve(options.get('--source')) :
    join(root, 'dogfood', '01-task-manager', 'main.svr');
const NL = String.fromCharCode(10);
const execute = (binary, arguments_, suffix) => {
    const start = performance.now();
    const result = spawnSync(binary, arguments_, {
        cwd: root, encoding: 'utf8', timeout: 10000,
        maxBuffer: 4 * 1024 * 1024, windowsHide: true,
    });
    const ms = Math.round(performance.now() - start);
    if (result.error?.code === 'ETIMEDOUT') return { ms, timed_out: true };
    assert.ifError(result.error);
    assert.equal(result.status, 0, result.stderr);
    assert.equal(result.stderr, '');
    assert.ok(result.stdout.replaceAll(String.fromCharCode(13, 10), NL).endsWith(suffix));
    return { ms, timed_out: false };
};
const temp = mkdtempSync(join(tmpdir(), 'sovra-task-bench-'));
try {
    const compiled = spawnSync(compiler, ['build', '--emit', 'js', source], {
        cwd: root, encoding: 'utf8', timeout: 60000, windowsHide: true,
    });
    assert.ifError(compiled.error);
    assert.equal(compiled.status, 0, compiled.stderr);
    const js = join(temp, 'task-manager.cjs');
    writeFileSync(js, compiled.stdout);
    const measurements = [];
    const timedOutEngines = new Set();
    for (const count of [100, 1000, 5000]) {
        const rows = Array.from({ length: count }, (_, index) =>
            String(index + 1) + '|0|Task ' + String(index + 1) + NL).join('');
        for (const [engine, binary, prefix] of [
            ['interpreter', compiler, ['run', source, '--']],
            ['javascript', process.execPath, [js]],
        ]) {
            if (timedOutEngines.has(engine)) continue;
            const path = join(temp, engine + '-' + count + '.txt');
            writeFileSync(path, 'SVR-TASKS-2' + NL + String(count + 1) + NL + rows);
            const list = execute(binary, [...prefix, 'list', path], 'count ' + count + NL);
            const entry = { engine, tasks: count, list };
            if (!list.timed_out) {
                entry.add = execute(binary, [...prefix, 'add', path, 'Next'], 'added ' + (count + 1) + NL);
                if (!entry.add.timed_out) {
                    entry.restart_list = execute(binary, [...prefix, 'list', path], 'count ' + (count + 1) + NL);
                }
            }
            if (list.timed_out || entry.add?.timed_out || entry.restart_list?.timed_out) {
                timedOutEngines.add(engine);
            }
            measurements.push(entry);
            console.log(engine + ' ' + count + ' tasks: ' + JSON.stringify(entry));
        }
    }
    const report = join(root, 'target', 'task-manager-benchmark-' +
        basename(source, '.svr') + '.json');
    mkdirSync(dirname(report), { recursive: true });
    writeFileSync(report, JSON.stringify({ schema_version: 1, measurements }, null, 2) + NL);
    console.log('Benchmark report: ' + report);
} finally {
    rmSync(temp, { recursive: true, force: true });
}
