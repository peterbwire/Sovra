// Application-owned pagination through actual interpreter/JavaScript execution.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const source = join(root, 'dogfood/01-task-manager/main.svr');
const compiler = join(root, 'target/debug', process.platform === 'win32' ? 'svr.exe' : 'svr');
const scratch = mkdtempSync(join(tmpdir(), 'sovra-task-pages-'));
function execute(binary, args) {
    const result = spawnSync(binary, args, { cwd: scratch, encoding: 'utf8', windowsHide: true,
        timeout: 30000, maxBuffer: 4 * 1024 * 1024 });
    assert.ifError(result.error);
    assert.equal(result.signal, null);
    return { status: result.status, stdout: result.stdout.replaceAll('\r\n', '\n'), stderr: result.stderr };
}
try {
    const built = execute(compiler, ['build', '--emit', 'js', source]);
    assert.equal(built.status, 0, built.stderr);
    const generated = join(scratch, 'tasks.cjs'); writeFileSync(generated, built.stdout);
    const evidence = [];
    for (const engine of ['interpreter', 'javascript']) {
        const state = join(scratch, engine + '.txt');
        const original = 'SVR-TASKS-2\n1001\n' + Array.from({ length: 1000 }, (_, index) =>
            `${index + 1}|0|Task ${index + 1}\n`).join('');
        writeFileSync(state, original);
        const run = args => engine === 'interpreter' ? execute(compiler, ['run', source, '--', ...args])
            : execute(process.execPath, [generated, ...args]);
        const snapshots = [];
        function view() {
            const result = run(['ui-layout-view', state]);
            assert.equal(result.status, 0, result.stderr + result.stdout);
            assert.ok(Buffer.byteLength(result.stdout) <= 65536, 'Grouped output exceeds renderer byte limit');
            const leaves = result.stdout.split('\n').filter(line => /^(heading|text|input|button)\|/.test(line));
            assert.ok(leaves.length <= 256, `Grouped view exceeds renderer limit: ${leaves.length} leaves`);
            const ids = [...result.stdout.matchAll(/^input\|rename-(\d+)\|/gm)].map(match => Number(match[1]));
            assert.ok(ids.length <= 25, 'A page contains too many task controls');
            snapshots.push(result.stdout); return { text: result.stdout, ids };
        }
        function event(action, value) {
            const args = ['ui-event', state, action]; if (value !== undefined) args.push(value);
            const result = run(args); assert.equal(result.status, 0, result.stderr + result.stdout);
        }
        writeFileSync(state, 'SVR-TASKS-2\n63\n' + Array.from({ length: 62 }, (_, i) => `${i + 1}|0|Task ${i + 1}\n`).join(''));
        view(); // The original desktop failed at precisely 62 open tasks.
        writeFileSync(state, original);
        let current = view(); assert.deepEqual(current.ids, Array.from({ length: 25 }, (_, i) => i + 1));
        assert.match(current.text, /Page 1 of 40/);
        event('page-next'); current = view(); assert.deepEqual(current.ids, Array.from({ length: 25 }, (_, i) => i + 26));
        assert.match(current.text, /Page 2 of 40/);
        assert.deepEqual(view(), current, 'Restart did not retain the selected page');
        const matching = Array.from({ length: 1000 }, (_, i) => i + 1).filter(id => `Task ${id}`.includes('Task 1'));
        event('search', 'Task 1'); assert.deepEqual(view().ids, matching.slice(0, 25));
        event('page-next'); assert.deepEqual(view().ids, matching.slice(25, 50), 'Filtered pagination skipped/repeated matches');
        event('search', 'Task 1000'); current = view(); assert.deepEqual(current.ids, [1000]);
        assert.match(current.text, /Page 1 of 1/);
        assert.equal(run(['ui-event', state, 'page-next']).status, 1);
        event('clear-search'); assert.deepEqual(view().ids, Array.from({ length: 25 }, (_, i) => i + 1));
        event('page-next'); event('filter-completed'); current = view(); assert.deepEqual(current.ids, []);
        assert.match(current.text, /Page 1 of 1/);
        event('filter-all'); assert.equal(view().ids[0], 1);
        assert.equal(readFileSync(state, 'utf8'), original, 'Navigation changed durable task data');
        const flat = run(['ui-view', state]); assert.equal(flat.status, 0);
        assert.equal([...flat.stdout.matchAll(/^button\|delete-/gm)].length, 1000, 'Flat interface was paginated');
        writeFileSync(state + '.page', 'SVR-TASK-PAGE-1\n39\n');
        assert.equal(view().ids[0], 976);
        const lastPage = readFileSync(state + '.page', 'utf8');
        assert.equal(run(['ui-event', state, 'page-next']).status, 1);
        assert.equal(run(['ui-event', state, 'page-previous', 'unexpected']).status, 1);
        assert.equal(readFileSync(state + '.page', 'utf8'), lastPage, 'Rejected navigation changed page state');
        writeFileSync(state + '.page', 'SVR-TASK-PAGE-1\n9223372036854775807\n');
        assert.equal(view().ids[0], 976, 'Large saved page was not safely clamped');
        event('page-previous'); assert.equal(view().ids[0], 951);
        writeFileSync(state, 'SVR-TASKS-2\n27\n' + Array.from({ length: 26 }, (_, i) => `${i + 1}|0|Task ${i + 1}\n`).join(''));
        assert.deepEqual(view().ids, [26]);
        event('delete-26'); assert.equal(view().ids[0], 1, 'Deleting the final page left an empty out-of-range view');
        writeFileSync(state, 'SVR-TASKS-2\n2\n1|0|' + 'é'.repeat(40000) + '\n');
        current = view(); assert.deepEqual(current.ids, [1]); assert.match(current.text, /long title/i);
        // Exercise long persisted queries without exceeding Windows argv limits.
        writeFileSync(state + '.view', 'SVR-TASK-VIEW-1\nall\n' + 'é'.repeat(40000)); view();
        const saved = readFileSync(state, 'utf8');
        writeFileSync(state + '.page', 'corrupt navigation');
        assert.equal(run(['ui-layout-view', state]).status, 1);
        assert.equal(run(['ui-event', state, 'page-next']).status, 1);
        assert.equal(readFileSync(state + '.page', 'utf8'), 'corrupt navigation');
        assert.equal(readFileSync(state, 'utf8'), saved);
        evidence.push(snapshots);
        console.log(`PASS pagination/${engine}: 1000 tasks, bounded views, navigation, filters, restart, long text and corrupt state`);
    }
    assert.deepEqual(evidence[0], evidence[1], 'Pagination differs between execution engines');
} finally { rmSync(scratch, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 }); }
