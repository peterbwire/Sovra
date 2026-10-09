// Differential subprocess tests for ADR 0015 bounded UTF-8 file operations.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { chmodSync, mkdtempSync, readFileSync, writeFileSync, mkdirSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve, relative } from 'node:path';

const root = resolve(import.meta.dirname, '..');
if (process.platform !== 'win32') process.umask(0o022);
const commandLine = process.argv.slice(2);
if (commandLine.length !== 0 && (commandLine.length !== 2 || commandLine[0] !== '--compiler')) {
    throw new Error('Usage: node scripts/test-text-files.mjs [--compiler PATH]');
}
const compiler = commandLine.length === 2 ? resolve(commandLine[1])
    : join(root, 'target', 'debug', process.platform === 'win32' ? 'svr.exe' : 'svr');
const source = join(root, 'dogfood', '02-file-processor', 'main.svr');
const directory = mkdtempSync(join(tmpdir(), 'sovra-text-files-'));
const generated = join(directory, 'file-processor.js');
const sequenceSource = join(root, 'tests', 'fixtures', 'text-file-sequence.svr');
const sequenceGenerated = join(directory, 'text-file-sequence.js');

function run(executable, args) {
    const result = spawnSync(executable, args, {
        cwd: root, encoding: 'utf8', timeout: 30000,
        maxBuffer: 4 * 1024 * 1024, windowsHide: true,
    });
    assert.ifError(result.error);
    assert.equal(result.signal, null);
    assert.equal(result.status, 0, result.stderr);
    assert.equal(result.stderr, '');
    return result.stdout.replaceAll('\r\n', '\n');
}

writeFileSync(generated, run(compiler, ['build', '--emit', 'js', source]));
writeFileSync(sequenceGenerated, run(compiler, ['build', '--emit', 'js', sequenceSource]));
const hosts = [
    {
        name: 'interpreter',
        copy: (from, to) => run(compiler, ['run', source, '--', 'copy', from, to]),
        select: (from, to, delimiter) => run(compiler, ['run', source, '--', 'select', from, to, delimiter]),
        sequence: (from, to) => run(compiler, ['run', sequenceSource, '--', from, to]),
    },
    {
        name: 'javascript',
        copy: (from, to) => run(process.execPath, [generated, 'copy', from, to]),
        select: (from, to, delimiter) => run(process.execPath, [generated, 'select', from, to, delimiter]),
        sequence: (from, to) => run(process.execPath, [sequenceGenerated, from, to]),
    },
];
for (const host of hosts) {
    const work = join(directory, host.name);
    mkdirSync(work);
    const input = join(work, 'input.txt');
    const output = join(work, 'output.txt');
    writeFileSync(input, 'hello\ncafé\n');
    writeFileSync(output, 'old content');
    if (process.platform !== 'win32') chmodSync(output, 0o600);
    assert.equal(host.copy(input, output), 'copied bytes: 12\n');
    assert.equal(readFileSync(output, 'utf8'), 'hello\ncafé\n');
    if (process.platform !== 'win32') {
        assert.equal(statSync(output).mode & 0o777, 0o600,
            `${host.name} widened a private file's mode during replacement`);
        const fresh = join(work, 'fresh.txt');
        assert.equal(host.copy(input, fresh), 'copied bytes: 12\n');
        assert.equal(statSync(fresh).mode & 0o777, 0o600,
            `${host.name} created a text file with broader permissions than owner-only`);
        chmodSync(output, 0o644);
        assert.equal(host.copy(input, output), 'copied bytes: 12\n');
        assert.equal(statSync(output).mode & 0o777, 0o600,
            `${host.name} did not narrow a shared file's mode during replacement`);
    }
    assert.equal(host.copy(relative(root, input), relative(root, output)), 'copied bytes: 12\n');
    assert.equal(host.copy(join(work, 'missing.txt'), output), 'read error: not_found\n');
    assert.equal(readFileSync(output, 'utf8'), 'hello\ncafé\n');
    assert.equal(host.copy(input, join(work, 'missing-parent', 'out.txt')), 'write error: not_found\n');
    assert.equal(host.copy('', output), 'read error: invalid_path\n');
    assert.equal(host.copy(input, ''), 'write error: invalid_path\n');
    writeFileSync(input, Buffer.from([0xff]));
    assert.equal(host.copy(input, output), 'read error: invalid_utf8\n');
    assert.equal(readFileSync(output, 'utf8'), 'hello\ncafé\n');
    writeFileSync(input, '');
    assert.equal(host.copy(input, output), 'copied bytes: 0\n');
    assert.equal(readFileSync(output, 'utf8'), '');
    writeFileSync(input, 'café|猫\nblue|天空\n');
    assert.equal(host.select(input, output, '|'), 'selected lines written\n');
    assert.equal(readFileSync(output, 'utf8'), 'café\nblue\n');
    writeFileSync(input, 'one::1\ntwo::2');
    assert.equal(host.select(input, output, '::'), 'selected lines written\n');
    assert.equal(readFileSync(output, 'utf8'), 'one\ntwo');
    writeFileSync(input, 'valid|row\ninvalid\n');
    assert.equal(host.select(input, output, '|'), 'select error: missing delimiter on line 2\n');
    assert.equal(readFileSync(output, 'utf8'), 'one\ntwo');
    assert.equal(host.select(input, output, ''), 'select error: empty delimiter\n');
    assert.equal(readFileSync(output, 'utf8'), 'one\ntwo');
    writeFileSync(input, 'a|b\n'.repeat(10001));
    assert.equal(host.select(input, output, '|'), 'select error: more than 10000 lines\n');
    assert.equal(readFileSync(output, 'utf8'), 'one\ntwo');
    writeFileSync(input, 'a'.repeat(1048577));
    assert.equal(host.select(input, output, '|'), 'select error: input exceeds 1048576 bytes\n');
    assert.equal(readFileSync(output, 'utf8'), 'one\ntwo');
    writeFileSync(input, '');
    assert.equal(host.select(input, output, '|'), 'selected lines written\n');
    assert.equal(readFileSync(output, 'utf8'), '');
    writeFileSync(input, Buffer.alloc(16 * 1024 * 1024, 0x61));
    assert.equal(host.copy(input, output), `copied bytes: ${16 * 1024 * 1024}\n`);
    assert.equal(readFileSync(output).length, 16 * 1024 * 1024);
    assert.equal(host.sequence(join(work, 'missing.txt'), join(work, 'absent', 'out.txt')),
        'before\nnot_found\nnot_found\n');
    writeFileSync(input, Buffer.alloc(16 * 1024 * 1024 + 1, 0x61));
    assert.equal(host.copy(input, output), 'read error: too_large\n');
    assert.equal(readFileSync(output).length, 16 * 1024 * 1024);
    console.log(`PASS ${host.name}: copy and select, replacement, Unicode, malformed rows, and size bounds`);
}
