// Restart and corruption checks for the actual Task Manager program in both engines.
import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { join, resolve, dirname } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const options = new Map();
const args = process.argv.slice(2);
if (args.length % 2 !== 0) {
    throw new Error('Usage: node scripts/test-task-persistence.mjs [--compiler PATH] [--source PATH]');
}
for (let index = 0; index < args.length; index += 2) {
    const key = args[index];
    if (!['--compiler', '--source'].includes(key) || options.has(key)) {
        throw new Error('Usage: node scripts/test-task-persistence.mjs [--compiler PATH] [--source PATH]');
    }
    options.set(key, args[index + 1]);
}
const compiler = options.has('--compiler') ? resolve(options.get('--compiler')) :
    join(root, 'target', 'debug', process.platform === 'win32' ? 'svr.exe' : 'svr');
const source = options.has('--source') ? resolve(options.get('--source')) :
    join(root, 'dogfood', '01-task-manager', 'main.svr');
const run = (executable, args, input = '', expectedStatus = 0) => {
    const result = spawnSync(executable, args, { cwd: dirname(source), input, encoding: 'utf8', timeout: 30000, windowsHide: true });
    assert.ifError(result.error);
    assert.equal(result.status, expectedStatus, result.stderr + result.stdout);
    assert.equal(result.stderr, '');
    return result.stdout.replaceAll('\r\n', '\n');
};
const waitForText = (child, expected) => new Promise((resolveWait, rejectWait) => {
    let output = '';
    const timer = setTimeout(() => rejectWait(new Error(`Timed out waiting for ${expected}: ${output}`)), 10000);
    child.stdout.on('data', chunk => {
        output += chunk.toString();
        if (output.includes(expected)) {
            clearTimeout(timer);
            resolveWait(output);
        }
    });
    child.once('error', rejectWait);
    child.once('exit', code => {
        if (!output.includes(expected)) rejectWait(new Error(`Exited ${code} before ${expected}: ${output}`));
    });
});
const directory = mkdtempSync(join(tmpdir(), 'sovra-tasks-'));
try {
    const js = join(directory, 'task-manager.cjs');
    writeFileSync(js, run(compiler, ['build', '--emit', 'js', source]));
    for (const [engine, executable, prefix] of [
        ['interpreter', compiler, ['run', source, '--']],
        ['javascript', process.execPath, [js]],
    ]) {
        const path = join(directory, `${engine}.txt`);
        const invoke = (input, status = 0) => run(executable, [...prefix, 'interactive', path], input, status);
        const directPath = join(directory, `${engine}-direct.txt`);
        const direct = (...arguments_) => run(executable, [...prefix, ...arguments_]);
        const directError = (...arguments_) => run(executable, [...prefix, ...arguments_], '', 1);
        assert.equal(direct('list', directPath), 'tasks:\ncount 0\n');
        assert.match(direct('add', directPath, 'Write tests'), /added 1\n/);
        assert.match(direct('add', directPath, 'Café|猫'), /added 2\n/);
        const beforeDuplicate = readFileSync(directPath, 'utf8');
        assert.match(directError('add', directPath, 'Write tests'), /error: duplicate title\n/);
        assert.equal(readFileSync(directPath, 'utf8'), beforeDuplicate);
        for (const title of ['Injected\nTask', 'Injected\rTask']) {
            assert.equal(directError('add', directPath, title), 'error: title contains a line break\n');
            assert.equal(readFileSync(directPath, 'utf8'), beforeDuplicate);
        }
        assert.match(direct('complete', directPath, 'Café|猫'), /completed 2\n/);
        assert.match(direct('list', directPath), /2 \| done \| Café\|猫\ncount 2\n/);
        assert.match(direct('delete', directPath, 'Write tests'), /deleted 1\n/);
        assert.match(direct('list', directPath), /2 \| done \| Café\|猫\ncount 1\n/);
        assert.match(directError('delete', directPath, 'missing'), /error: missing title missing\n/);
        assert.match(run(executable, [...prefix, 'interactive', directPath], 'add\nFrom interactive\nquit\n'), /added 3\n/);
        assert.match(direct('list', directPath), /3 \| open \| From interactive\ncount 2\n/);
        const sharedPath = join(directory, `${engine}-shared.txt`);
        const session = spawn(executable, [...prefix, 'interactive', sharedPath], {
            cwd: dirname(source), stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true,
        });
        let sessionOutput = '';
        let sessionError = '';
        session.stdout.on('data', chunk => { sessionOutput += chunk.toString(); });
        session.stderr.on('data', chunk => { sessionError += chunk.toString(); });
        await waitForText(session, 'command:\n');
        session.stdin.write('add\n');
        await waitForText(session, 'title:\n');
        assert.match(direct('add', sharedPath, 'Outside'), /added 1\n/);
        session.stdin.end('Inside\nquit\n');
        const sessionStatus = await new Promise((resolveExit, rejectExit) => {
            session.once('error', rejectExit);
            session.once('close', resolveExit);
        });
        assert.equal(sessionStatus, 1, sessionError + sessionOutput);
        assert.match(sessionOutput, /error: task file changed\n/);
        assert.equal(direct('list', sharedPath), 'tasks:\n1 | open | Outside\ncount 1\n');
        const existingSession = spawn(executable, [...prefix, 'interactive', sharedPath], {
            cwd: dirname(source), stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true,
        });
        let existingOutput = '';
        let existingError = '';
        existingSession.stdout.on('data', chunk => { existingOutput += chunk.toString(); });
        existingSession.stderr.on('data', chunk => { existingError += chunk.toString(); });
        await waitForText(existingSession, 'command:\n');
        existingSession.stdin.write('add\n');
        await waitForText(existingSession, 'title:\n');
        assert.match(direct('add', sharedPath, 'Second outside'), /added 2\n/);
        const externalState = readFileSync(sharedPath, 'utf8');
        existingSession.stdin.end('Second inside\nquit\n');
        const existingStatus = await new Promise((resolveExit, rejectExit) => {
            existingSession.once('error', rejectExit);
            existingSession.once('close', resolveExit);
        });
        assert.equal(existingStatus, 1, existingError + existingOutput);
        assert.match(existingOutput, /error: task file changed\n/);
        assert.equal(readFileSync(sharedPath, 'utf8'), externalState);
        const listingSession = spawn(executable, [...prefix, 'interactive', sharedPath], {
            cwd: dirname(source), stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true,
        });
        let listingOutput = '';
        let listingError = '';
        listingSession.stdout.on('data', chunk => { listingOutput += chunk.toString(); });
        listingSession.stderr.on('data', chunk => { listingError += chunk.toString(); });
        await waitForText(listingSession, 'command:\n');
        assert.match(direct('add', sharedPath, 'Third outside'), /added 3\n/);
        listingSession.stdin.end('list\nquit\n');
        const listingStatus = await new Promise((resolveExit, rejectExit) => {
            listingSession.once('error', rejectExit);
            listingSession.once('close', resolveExit);
        });
        assert.equal(listingStatus, 0, listingError + listingOutput);
        assert.match(listingOutput, /3 \| open \| Third outside\ncount 3\n/);
        const unsavedDirect = directError('add', join(directory, 'missing-parent', `${engine}-direct.txt`), 'Unsaved');
        assert.match(unsavedDirect, /error: save not_found\n/);
        assert.doesNotMatch(unsavedDirect, /added 1\n/);
        const validDirect = readFileSync(directPath, 'utf8');
        writeFileSync(directPath, 'SVR-TASKS-2\n4\n3|0|A\n3|0|B\n');
        assert.equal(directError('list', directPath), 'error: invalid task file\n');
        assert.equal(readFileSync(directPath, 'utf8'), 'SVR-TASKS-2\n4\n3|0|A\n3|0|B\n');
        writeFileSync(directPath, validDirect);
        assert.equal(directError('list'), 'error: expected list <path>\n');
        assert.equal(directError('add', directPath), 'error: expected add <path> <title>\n');
        assert.equal(directError('unknown'), 'error: expected help, demo, interactive, list, add, complete or delete\n');
        assert.equal(direct('help'), 'Task Manager\nusage: interactive [path] | list <path> | add <path> <title>\n       complete <path> <title> | delete <path> <title> | demo\n');
        const unwritable = join(directory, 'missing-parent', `${engine}.txt`);
        const failedSave = run(executable, [...prefix, 'interactive', unwritable], 'add\nUnsaved\nquit\n', 1);
        assert.match(failedSave, /error: save not_found\n/);
        assert.doesNotMatch(failedSave, /added 1\n/);
        assert.match(invoke('add\nCafé|猫\nquit\n'), /added 1\n/);
        assert.match(invoke('list\nquit\n'), /1 \| open \| Café\|猫\ncount 1\n/);
        assert.match(invoke('complete\nCafé|猫\nquit\n'), /completed 1\n/);
        assert.match(invoke('list\nquit\n'), /1 \| done \| Café\|猫\ncount 1\n/);
        assert.match(invoke('delete\nCafé|猫\nquit\n'), /deleted 1\n/);
        assert.match(invoke('add\nReplacement\nquit\n'), /added 2\n/);
        assert.match(invoke('list\nquit\n'), /2 \| open \| Replacement\ncount 1\n/);
        const saved = readFileSync(path, 'utf8');
        assert.equal(saved, 'SVR-TASKS-2\n3\n2|0|Replacement\n');
        const many = Array.from({ length: 25 }, (_, index) => `Task ${index + 1}`);
        assert.match(invoke(many.map(title => `add\n${title}\n`).join('') + 'quit\n'), /added 27\n/);
        assert.match(invoke('list\nquit\n'), /27 \| open \| Task 25\ncount 26\n/);
        assert.match(invoke('complete\nTask 20\ndelete\nTask 5\nquit\n'), /completed 22\n[\s\S]*deleted 7\n/);
        assert.match(invoke('list\nquit\n'), /22 \| done \| Task 20\n/);
        assert.match(invoke('list\nquit\n'), /count 25\n/);
        assert.doesNotMatch(invoke('list\nquit\n'), /7 \| open \| Task 5\n/);
        const old = 'SVR-TASKS-1\n8\n5|0|Legacy\n0|0|\n7|1|Done\n';
        writeFileSync(path, old);
        assert.match(invoke('list\nquit\n'), /5 \| open \| Legacy\n7 \| done \| Done\ncount 2\n/);
        assert.equal(readFileSync(path, 'utf8'), old);
        assert.match(invoke('add\nMigrated\nquit\n'), /added 8\n/);
        assert.equal(readFileSync(path, 'utf8'), 'SVR-TASKS-2\n9\n5|0|Legacy\n7|1|Done\n8|0|Migrated\n');
        for (const invalid of [
            'wrong\n3\n2|0|Replacement\n0|0|\n0|0|\n',
            'SVR-TASKS-1\n3\n2|0|Same\n2|0|Same\n0|0|\n',
            'SVR-TASKS-1\n3\n2|0|Replacement\n0|0|\n',
            'SVR-TASKS-1\n999999999999999999999\n0|0|\n0|0|\n0|0|\n',
            'SVR-TASKS-2\n4\n1|0|One\n1|1|Duplicate\n',
            'SVR-TASKS-2\n4\n1|0|Same\n2|0|Same\n',
            'SVR-TASKS-2\n4\n4|0|Future\n',
            'SVR-TASKS-2\n4\n1|0|No newline',
            'SVR-TASKS-2\n4\n0|0|\n',
        ]) {
            writeFileSync(path, invalid);
            assert.match(invoke('add\nLost\nquit\n', 1), /error: invalid task file\n/);
            assert.equal(readFileSync(path, 'utf8'), invalid);
        }
        const scaleRows = Array.from({ length: 1000 }, (_, index) =>
            `${index + 1}|0|Scaled ${index + 1}\n`).join('');
        writeFileSync(path, `SVR-TASKS-2\n1001\n${scaleRows}`);
        assert.match(invoke('list\nquit\n'), /1000 \| open \| Scaled 1000\ncount 1000\n/);
        assert.match(invoke('add\nScaled 1001\nquit\n'), /added 1001\n/);
        assert.match(invoke('complete\nScaled 1000\ndelete\nScaled 500\nquit\n'), /completed 1000\n[\s\S]*deleted 500\n/);
        assert.match(invoke('list\nquit\n'), /1000 \| done \| Scaled 1000\n[\s\S]*count 1000\n/);
        for (const corrupt of [
            `SVR-TASKS-2\n1002\n${scaleRows}1001|0|Scaled 1\n`,
            `SVR-TASKS-2\n1002\n${scaleRows}1|0|Unique\n`,
        ]) {
            writeFileSync(path, corrupt);
            assert.equal(directError('list', path), 'error: invalid task file\n');
            assert.equal(readFileSync(path, 'utf8'), corrupt);
        }
        const exhausted = 'SVR-TASKS-2\n9223372036854775807\n9223372036854775806|0|Last\n';
        writeFileSync(path, exhausted);
        assert.equal(directError('add', path, 'Cannot allocate'), 'error: task ID exhausted\n');
        assert.equal(readFileSync(path, 'utf8'), exhausted);
        assert.match(direct('complete', path, 'Last'), /completed 9223372036854775806\n/);
        assert.equal(directError('add', path, 'Still exhausted'), 'error: task ID exhausted\n');
        const boundary = join(directory, `${engine}-boundary.txt`);
        const boundaryHeader = 'SVR-TASKS-2\n2\n1|0|';
        const withinLimit = boundaryHeader +
            'a'.repeat(16 * 1024 * 1024 - Buffer.byteLength(boundaryHeader) - 1) + '\n';
        writeFileSync(boundary, withinLimit);
        const boundaryArgs = [...prefix, 'interactive', boundary];
        assert.match(run(executable, boundaryArgs, 'quit\n'), /Task Manager\n/);
        const rejectedGrowth = run(executable, boundaryArgs, 'add\nBeyond limit\nquit\n', 1);
        assert.match(rejectedGrowth, /error: save too_large\n/);
        assert.doesNotMatch(rejectedGrowth, /added 2\n/);
        assert.equal(readFileSync(boundary, 'utf8'), withinLimit);
        writeFileSync(boundary, withinLimit + 'x');
        assert.match(run(executable, boundaryArgs, 'quit\n', 1), /error: load too_large\n/);
        console.log(`PASS Task Manager persistence/${engine}`);
    }
} finally {
    rmSync(directory, { recursive: true, force: true });
}
