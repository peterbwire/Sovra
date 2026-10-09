// Verify the exact candidate archive after extraction outside the repository.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, delimiter, dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const usage = 'Usage: node scripts/verify-candidate-archive.mjs --archive PATH --name NAME --version VERSION';
const arguments_ = process.argv.slice(2);
assert.equal(arguments_.length, 6, usage);
const options = new Map();
for (let index = 0; index < arguments_.length; index += 2) {
    const option = arguments_[index];
    assert.ok(['--archive', '--name', '--version'].includes(option) && !options.has(option), usage);
    options.set(option, arguments_[index + 1]);
}
for (const option of ['--archive', '--name', '--version']) assert.ok(options.get(option), usage);
const archive = resolve(options.get('--archive'));
const name = options.get('--name');
const version = options.get('--version');
assert.match(name, /^[A-Za-z0-9][A-Za-z0-9._-]*$/, 'unsafe candidate directory name');
assert.equal(basename(archive), `${name}.tar.gz`, 'archive name does not match candidate');

const expectedHashLine = readFileSync(`${archive}.sha256`, 'ascii').trim();
const actualHash = createHash('sha256').update(readFileSync(archive)).digest('hex');
assert.equal(expectedHashLine, `${actualHash}  ${basename(archive)}`, 'candidate checksum mismatch');

const invoke = (binary, args, cwd, expected = 0, env = process.env) => {
    const result = spawnSync(binary, args, {
        cwd, env, encoding: 'utf8', timeout: 60000, windowsHide: true,
        maxBuffer: 8 * 1024 * 1024,
    });
    assert.ifError(result.error);
    assert.equal(result.status, expected, `${binary} ${args.join(' ')}\n${result.stderr}\n${result.stdout}`);
    assert.equal(result.stderr, '', `${binary} ${args.join(' ')}`);
    return result.stdout.replaceAll('\r\n', '\n');
};

const unpack = mkdtempSync(join(tmpdir(), 'sovra-candidate-verify-'));
try {
    const listing = invoke('tar', ['-tzf', archive], unpack);
    const entries = listing.trimEnd().split('\n');
    assert.ok(entries.length > 1, 'candidate archive is empty');
    for (const entry of entries) {
        assert.ok(entry === `${name}/` || entry.startsWith(`${name}/`), `unexpected archive entry: ${entry}`);
        assert.ok(!entry.split('/').includes('..') && !entry.includes('\\'), `unsafe archive entry: ${entry}`);
    }
    invoke('tar', ['-xzf', archive, '-C', unpack], unpack);
    const installed = join(unpack, name);
    const compiler = join(installed, process.platform === 'win32' ? 'svr.exe' : 'svr');
    const hello = join(installed, 'hello.svr');
    const taskManager = join(installed, 'task-manager.svr');
    for (const file of [compiler, hello, taskManager, 'README.md', 'task-manager-README.md',
        'LICENSE', 'LICENSE-MIT', 'LICENSE-APACHE']) {
        const path = file.startsWith(installed) ? file : join(installed, file);
        assert.ok(existsSync(path) && statSync(path).isFile(), `missing packaged file: ${file}`);
    }
    assert.equal(invoke(compiler, ['--version'], installed), `svr ${version}\n`);
    assert.equal(invoke(compiler, ['run', hello], installed), 'Hello, Sovra!\n');
    const report = JSON.parse(invoke(compiler, ['check', '--format', 'json', hello], installed));
    assert.equal(report.schema_version, 1);
    assert.equal(report.success, true);
    assert.equal(report.kind, 'source');
    assert.deepEqual(report.diagnostics, []);
    const javascript = invoke(compiler, ['build', '--emit', 'js', hello], installed);
    const generated = join(installed, 'hello.cjs');
    writeFileSync(generated, javascript);
    assert.equal(invoke(process.execPath, [generated], installed), 'Hello, Sovra!\n');
    invoke(compiler, ['check', taskManager], installed);
    const persistence = invoke(process.execPath, [join(root, 'scripts/test-task-persistence.mjs'),
        '--compiler', compiler, '--source', taskManager], installed);
    assert.match(persistence, /PASS Task Manager persistence\/interpreter\n/);
    assert.match(persistence, /PASS Task Manager persistence\/javascript\n/);
    const work = join(unpack, 'fresh-working-directory');
    mkdirSync(work);
    const pathKey = Object.keys(process.env).find(key => key.toLowerCase() === 'path') ?? 'PATH';
    const env = { ...process.env, [pathKey]: `${installed}${delimiter}${process.env[pathKey] ?? ''}` };
    const command = process.platform === 'win32' ? 'svr.exe' : 'svr';
    assert.equal(invoke(command, ['--version'], work, 0, env), `svr ${version}\n`);
    assert.equal(invoke(command, ['run', taskManager, '--', 'add', 'tasks.txt', 'Installed task'], work, 0, env), 'added 1\n');
    assert.equal(invoke(command, ['run', taskManager, '--', 'list', 'tasks.txt'], work, 0, env),
        'tasks:\n1 | open | Installed task\ncount 1\n');
    assert.ok(existsSync(join(work, 'tasks.txt')), 'relative state path was not created in the working directory');
    console.log(`PASS candidate archive ${name}: checksum, compiler, JavaScript, Task Manager persistence, portable PATH use`);
} finally {
    rmSync(unpack, { recursive: true, force: true });
}
