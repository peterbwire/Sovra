// Private grouped views use ordinary Sovra in both execution engines.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const compiler = join(root, 'target/debug', process.platform === 'win32' ? 'svr.exe' : 'svr');
const scratch = mkdtempSync(join(tmpdir(), 'sovra-ui-layout-'));
function execute(binary, args) {
    const result = spawnSync(binary, args, { cwd: root, encoding: 'utf8', timeout: 30000, windowsHide: true });
    assert.ifError(result.error);
    assert.equal(result.signal, null);
    return { status: result.status, stdout: result.stdout.replaceAll('\r\n', '\n'), stderr: result.stderr.replaceAll('\r\n', '\n') };
}
try {
    for (const [name, path, events] of [
        ['tasks', 'dogfood/01-task-manager/main.svr', [['add', 'Layout <data> "quotes" \\'], ['rename-1', 'Edited | title'], ['complete-1'], ['delete-1']]],
        ['component', 'examples/ui-desktop/component.svr', [['save-message', 'Shared | layout'], ['reset-message']]],
    ]) {
        const source = join(root, path);
        const built = execute(compiler, ['build', '--emit', 'js', source]);
        assert.equal(built.status, 0, built.stderr);
        const generated = join(scratch, `${name}.cjs`);
        writeFileSync(generated, built.stdout);
        const evidence = [];
        for (const engine of ['interpreter', 'javascript']) {
            const state = join(scratch, `${name}-${engine}.txt`);
            const run = args => engine === 'interpreter'
                ? execute(compiler, ['run', source, '--', ...args])
                : execute(process.execPath, [generated, ...args]);
            const views = [];
            function view() {
                const grouped = run(['ui-layout-view', state]);
                assert.equal(grouped.status, 0, grouped.stderr + grouped.stdout);
                assert.ok(grouped.stdout.startsWith('SVR-UI-2\n'));
                assert.match(grouped.stdout, /^group\|/m);
                const flat = run(['ui-view', state]);
                assert.equal(flat.status, 0, flat.stderr);
                const flattened = grouped.stdout.replace(/group\|view-options\|[\s\S]*$/, '')
                    .replace(/^SVR-UI-2\n/, 'SVR-UI-1\n')
                    .split('\n').filter(line => !/^(group|end)\|/.test(line) && !/^input\|rename-/.test(line)).join('\n');
                assert.equal(flattened, flat.stdout, 'Layout changed the existing leaf view');
                views.push(grouped.stdout);
            }
            view();
            for (const event of events) {
                const result = run(['ui-event', state, ...event]);
                assert.equal(result.status, 0, result.stderr + result.stdout);
                view();
            }
            if (name === 'tasks') {
                assert.equal(run(['ui-event', state, 'add', 'First']).status, 0);
                assert.equal(run(['ui-event', state, 'add', 'Second']).status, 0);
                const unchanged = readFileSync(state, 'utf8');
                for (const [action, value] of [['rename-2', ''], ['rename-2', 'Second'],
                    ['rename-2', 'bad\ntitle'], ['rename-2', '\x1btitle'], ['rename-999', 'Missing']]) {
                    assert.equal(run(['ui-event', state, action, value]).status, 1);
                    assert.equal(readFileSync(state, 'utf8'), unchanged, 'Rejected rename mutated storage');
                }
                assert.equal(run(['ui-event', state, 'rename-2', 'First']).status, 0);
                assert.equal(readFileSync(state, 'utf8'), unchanged, 'Unchanged rename mutated storage');
                function filtered(action, value, expected, absent) {
                    const args = ['ui-event', state, action];
                    if (value !== undefined) args.push(value);
                    assert.equal(run(args).status, 0);
                    const output = run(['ui-layout-view', state]);
                    assert.equal(output.status, 0);
                    for (const id of expected) assert.ok(output.stdout.includes(`input|rename-${id}|`), output.stdout);
                    for (const id of absent) assert.ok(!output.stdout.includes(`input|rename-${id}|`), output.stdout);
                    assert.equal(readFileSync(state, 'utf8'), unchanged, 'Filtering changed task data');
                    views.push(output.stdout);
                }
                filtered('filter-completed', undefined, [], [2, 3]);
                filtered('filter-open', undefined, [2, 3], []);
                filtered('search', 'First', [2], [3]);
                filtered('search', 'first', [], [2, 3]);
                filtered('clear-search', undefined, [2, 3], []);
                filtered('filter-all', undefined, [2, 3], []);
                assert.equal(run(['ui-event', state, 'complete-3']).status, 0);
                assert.equal(run(['ui-event', state, 'filter-completed']).status, 0);
                const completedView = run(['ui-layout-view', state]);
                assert.ok(completedView.stdout.includes('input|rename-3|') && !completedView.stdout.includes('input|rename-2|'));
                assert.equal(run(['ui-event', state, 'filter-open']).status, 0);
                const openView = run(['ui-layout-view', state]);
                assert.ok(openView.stdout.includes('input|rename-2|') && !openView.stdout.includes('input|rename-3|'));
                const preferences = readFileSync(state + '.view', 'utf8');
                assert.equal(run(['ui-event', state, 'search', 'bad\nquery']).status, 1);
                assert.equal(run(['ui-event', state, 'filter-open', 'unexpected']).status, 1);
                assert.equal(readFileSync(state + '.view', 'utf8'), preferences);
                writeFileSync(state + '.view', 'corrupt preferences');
                assert.equal(run(['ui-layout-view', state]).status, 1);
                assert.equal(run(['ui-event', state, 'search', 'overwrite?']).status, 1);
                assert.equal(readFileSync(state + '.view', 'utf8'), 'corrupt preferences');
                assert.equal(run(['ui-view', state]).status, 0, 'Flat view depended on private preferences');
                writeFileSync(state + '.view', preferences);
            }
            evidence.push({ views, state: readFileSync(state, 'utf8') });
        }
        assert.deepEqual(evidence[0], evidence[1]);
        console.log(`PASS grouped UI parity/${name}: both engines, existing flat view and persisted events`);
    }
} finally { rmSync(scratch, { recursive: true, force: true }); }
