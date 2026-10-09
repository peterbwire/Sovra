// Install the downloaded Cargo source package in a fresh temporary prefix.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, join, resolve } from 'node:path';

const usage = 'Usage: node scripts/verify-source-package.mjs --crate PATH --version VERSION';
const arguments_ = process.argv.slice(2);
assert.equal(arguments_.length, 4, usage);
const options = new Map();
for (let index = 0; index < arguments_.length; index += 2) {
    const option = arguments_[index];
    assert.ok(['--crate', '--version'].includes(option) && !options.has(option), usage);
    options.set(option, arguments_[index + 1]);
}
for (const option of ['--crate', '--version']) assert.ok(options.get(option), usage);
const crate = resolve(options.get('--crate'));
const version = options.get('--version');
assert.match(version, /^[0-9]+\.[0-9]+\.[0-9]+(?:[-+][A-Za-z0-9.-]+)?$/, 'invalid package version');
assert.equal(basename(crate), `sovra-${version}.crate`, 'unexpected source archive name');
const hash = createHash('sha256').update(readFileSync(crate)).digest('hex');
assert.equal(readFileSync(`${crate}.sha256`, 'ascii').trim(), `${hash}  ${basename(crate)}`,
    'source package checksum mismatch');

const invoke = (binary, args, cwd, env = process.env, timeout = 60000) => {
    const result = spawnSync(binary, args, {
        cwd, env, encoding: 'utf8', timeout, windowsHide: true, maxBuffer: 8 * 1024 * 1024,
    });
    assert.ifError(result.error);
    assert.equal(result.status, 0, `${binary} ${args.join(' ')}\n${result.stderr}\n${result.stdout}`);
    return result.stdout.replaceAll('\r\n', '\n');
};

const scratch = mkdtempSync(join(tmpdir(), 'sovra-source-install-'));
try {
    const listing = invoke('tar', ['-tzf', crate], scratch);
    const entries = listing.trimEnd().split('\n');
    const prefix = `sovra-${version}/`;
    assert.ok(entries.length > 1, 'source package is empty');
    for (const entry of entries) {
        assert.ok(entry.startsWith(prefix), `unexpected source package entry: ${entry}`);
        assert.ok(!entry.split('/').includes('..') && !entry.includes('\\'), `unsafe source package entry: ${entry}`);
    }
    invoke('tar', ['-xzf', crate, '-C', scratch], scratch);
    const source = join(scratch, `sovra-${version}`);
    const taskManager = join(source, 'dogfood', '01-task-manager', 'main.svr');
    assert.ok(existsSync(taskManager), 'Task Manager source missing from Cargo package');
    const install = join(scratch, 'install');
    const env = { ...process.env, CARGO_TARGET_DIR: join(scratch, 'build') };
    invoke('cargo', ['install', '--offline', '--locked', '--path', source, '--root', install],
        scratch, env, 600000);
    const compiler = join(install, 'bin', process.platform === 'win32' ? 'svr.exe' : 'svr');
    const work = join(scratch, 'working-directory');
    mkdirSync(work);
    assert.equal(invoke(compiler, ['--version'], work), `svr ${version}\n`);
    assert.equal(invoke(compiler, ['run', taskManager, '--', 'add', 'tasks.txt', 'Installed task'], work),
        'added 1\n');
    assert.equal(invoke(compiler, ['run', taskManager, '--', 'list', 'tasks.txt'], work),
        'tasks:\n1 | open | Installed task\ncount 1\n');
    const state = join(work, 'tasks.txt');
    const legacy = 'SVR-TASKS-1\n8\n5|0|Legacy\n0|0|\n7|1|Done\n';
    writeFileSync(state, legacy);
    assert.equal(invoke(compiler, ['run', taskManager, '--', 'list', 'tasks.txt'], work),
        'tasks:\n5 | open | Legacy\n7 | done | Done\ncount 2\n');
    assert.equal(readFileSync(state, 'utf8'), legacy, 'listing rewrote legacy state');
    assert.equal(invoke(compiler, ['run', taskManager, '--', 'add', 'tasks.txt', 'Upgraded'], work),
        'added 8\n');
    assert.equal(readFileSync(state, 'utf8'),
        'SVR-TASKS-2\n9\n5|0|Legacy\n7|1|Done\n8|0|Upgraded\n');
    console.log(`PASS Cargo source package sovra-${version}: offline install, fresh state, legacy migration`);
} finally {
    rmSync(scratch, { recursive: true, force: true });
}
