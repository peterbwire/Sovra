// ADR 0034 behavior in the actual interpreter and generated JavaScript.
import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, statSync, symlinkSync, unlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const arguments_ = process.argv.slice(2);
assert.ok(arguments_.length === 0 || (arguments_.length === 2 && arguments_[0] === '--compiler'),
    'Usage: node scripts/test-conditional-text-files.mjs [--compiler PATH]');
const compiler = arguments_.length ? resolve(arguments_[1]) :
    join(root, 'target/debug', process.platform === 'win32' ? 'svr.exe' : 'svr');
const directory = mkdtempSync(join(tmpdir(), 'sovra-conditional-'));
function run(binary, args, status = 0, input = '') {
    const result = spawnSync(binary, args, { cwd: root, windowsHide: true, encoding: 'utf8',
        timeout: 30000, maxBuffer: 1024 * 1024, input });
    assert.ifError(result.error);
    assert.equal(result.signal, null);
    assert.equal(result.status, status, result.stderr + result.stdout);
    assert.equal(result.stderr, '');
    return result.stdout.replaceAll('\r\n', '\n');
}
try {
    const source = join(directory, 'conditional.svr');
    writeFileSync(source, `fn main() {
        let loaded = std::read_text(std::arg(2));
        let replacement = std::read_text(std::arg(3));
        let mut expected = loaded.text;
        let mut next = replacement.text;
        if std::arg(4) == "grow-expected" { expected = expected + "x"; }
        if std::arg(4) == "grow-replacement" { next = next + "x"; }
        let saved: std::TextWrite = std::write_text_if(std::arg(0), std::arg(1) == "true", expected, next);
        print(saved.ok); print(saved.error);
    }`);
    const generated = join(directory, 'conditional.cjs');
    writeFileSync(generated, run(compiler, ['build', '--emit', 'js', source]));
    const taskSource = join(root, 'dogfood/01-task-manager/main.svr');
    const taskGenerated = join(directory, 'task-manager.cjs');
    writeFileSync(taskGenerated, run(compiler, ['build', '--emit', 'js', taskSource]));
    for (const [engine, binary, prefix] of [
        ['interpreter', compiler, ['run', source, '--']],
        ['javascript', process.execPath, [generated]],
    ]) {
        const work = join(directory, engine); mkdirSync(work);
        const path = join(work, 'state with spaces !');
        const marker = path + '.svr-write-lock';
        const expected = join(work, 'expected'), replacement = join(work, 'replacement');
        function attempt(exists, before, after, error = '', target = path, mode = '') {
            writeFileSync(expected, before); writeFileSync(replacement, after);
            assert.equal(run(binary, [...prefix, target, String(exists), expected, replacement, mode]),
                `${error ? 'false' : 'true'}\n${error}\n`, `${engine}: ${error || 'commit'}`);
        }
        attempt(true, '', 'lost', 'conflict');
        assert.equal(existsSync(path), false);
        attempt(false, '', '');
        attempt(false, '', 'lost', 'conflict');
        attempt(false, 'not empty', 'lost', 'invalid_snapshot');
        attempt(true, '', '\ufeff猫');
        attempt(true, '猫', 'lost', 'conflict');
        attempt(true, '\ufeff猫', 'new');
        if (process.platform !== 'win32') assert.equal(statSync(path).mode & 0o777, 0o600);
        writeFileSync(marker, 'foreign owner');
        attempt(true, 'new', 'lost', 'busy');
        assert.equal(readFileSync(marker, 'utf8'), 'foreign owner');
        const alias = join(work, 'alias');
        symlinkSync(work, alias, process.platform === 'win32' ? 'junction' : 'dir');
        attempt(true, 'new', 'lost', 'busy', join(alias, 'state with spaces !'));
        // Remove only the link; recursive removal of a directory alias is forbidden.
        unlinkSync(alias);
        unlinkSync(marker);
        const holder = spawn(process.execPath, ['-e', `
            const fs = require('node:fs');
            const fd = fs.openSync(process.argv[1], 'wx', 0o600);
            process.stdout.write('held\\n');
            setInterval(() => fs.fstatSync(fd), 1000);
        `, marker], { windowsHide: true });
        let output = '', problem;
        holder.stdout.setEncoding('utf8');
        holder.stdout.on('data', chunk => { output += chunk; });
        holder.once('error', error => { problem = error; });
        const stopped = new Promise(resolveStop => holder.once('close', resolveStop));
        try {
            const deadline = Date.now() + 10000;
            while (!output.includes('held\n') && !problem && holder.exitCode === null && Date.now() < deadline)
                await new Promise(resolveTick => setTimeout(resolveTick, 10));
            assert.ifError(problem); assert.match(output, /held\n/);
            attempt(true, 'new', 'lost', 'busy');
            holder.kill(); await stopped;
            attempt(true, 'new', 'lost', 'busy');
            assert.equal(existsSync(marker), true, 'An orphan marker must not be auto-removed');
            assert.equal(readFileSync(path, 'utf8'), 'new');
        } finally {
            if (holder.exitCode === null && holder.signalCode === null) holder.kill();
            await stopped;
        }
        // Only this owned fixture's stopped writer is being recovered.
        unlinkSync(marker);
        attempt(true, 'new', 'updated', '', relative(root, path));
        attempt(false, '', 'lost', 'invalid_path', work);
        attempt(false, '', 'lost', 'not_found', join(work, 'missing-parent', 'state'));
        if (process.platform !== 'win32') {
            const link = join(work, 'linked-state'); symlinkSync(path, link);
            attempt(true, 'updated', 'lost', 'invalid_path', link);
            assert.equal(readFileSync(path, 'utf8'), 'updated'); unlinkSync(link);
        }
        writeFileSync(path, Buffer.from([255]));
        attempt(true, '', 'lost', 'invalid_utf8');
        assert.deepEqual(readFileSync(path), Buffer.from([255]));
        writeFileSync(path, 'base');
        const full = 'a'.repeat(16 * 1024 * 1024);
        attempt(true, 'base', full);
        attempt(true, full, 'lost', 'too_large', path, 'grow-expected');
        attempt(true, full, full, 'too_large', path, 'grow-replacement');
        assert.equal(readFileSync(path, 'utf8'), full);
        writeFileSync(path, full + 'x');
        attempt(true, '', 'lost', 'too_large');
        assert.equal(existsSync(marker), false);
        const taskPrefix = engine === 'interpreter' ? ['run', taskSource, '--'] : [taskGenerated];
        const tasks = join(work, 'tasks.txt');
        const stored = 'SVR-TASKS-2\n27\n' + Array.from({ length: 26 }, (_, index) => `${index + 1}|0|Task ${index + 1}\n`).join('');
        writeFileSync(tasks, stored);
        writeFileSync(tasks + '.svr-write-lock', 'owned test marker');
        for (const command of [['add', tasks, 'lost'], ['ui-event', tasks, 'add', 'lost']])
            assert.match(run(binary, [...taskPrefix, ...command], 1), /error: save busy\n/);
        assert.match(run(binary, [...taskPrefix, 'interactive', tasks], 1, 'add\nlost\nquit\n'), /error: save busy\n/);
        assert.equal(readFileSync(tasks, 'utf8'), stored);
        unlinkSync(tasks + '.svr-write-lock');
        writeFileSync(tasks + '.view.svr-write-lock', 'owned test marker');
        assert.match(run(binary, [...taskPrefix, 'ui-event', tasks, 'filter-open'], 1), /error: save view busy\n/);
        assert.equal(existsSync(tasks + '.view'), false);
        unlinkSync(tasks + '.view.svr-write-lock');
        writeFileSync(tasks + '.page.svr-write-lock', 'owned test marker');
        assert.match(run(binary, [...taskPrefix, 'ui-event', tasks, 'page-next'], 1), /error: save page busy\n/);
        assert.match(run(binary, [...taskPrefix, 'ui-event', tasks, 'filter-open'], 1), /error: reset page busy\n/);
        assert.equal(readFileSync(tasks + '.view', 'utf8'), 'SVR-TASK-VIEW-1\nopen\n');
        assert.equal(existsSync(tasks + '.page'), false);
        assert.equal(readFileSync(tasks, 'utf8'), stored);
        unlinkSync(tasks + '.page.svr-write-lock');
        console.log(`PASS ${engine}: conditional creation/replacement, snapshots, BOM bytes, busy ownership, aliases, invalid data and size bounds`);
    }
    console.log(run(process.execPath, [join(root, 'scripts/probe-text-write-conflict.mjs'), '--conditional', '--compiler', compiler]).trim());
} finally { rmSync(directory, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 }); }
