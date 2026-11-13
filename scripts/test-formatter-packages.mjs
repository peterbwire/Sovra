// Package formatter selection and failure checks, entirely inside a temporary tree.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { chmodSync, linkSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
assert.ok(args.length === 0 || (args.length === 2 && args[0] === '--compiler'));
const compiler = resolve(root, args[1] ?? `target/debug/svr${process.platform === 'win32' ? '.exe' : ''}`);
const scratch = mkdtempSync(join(tmpdir(), 'sovra package formatter '));
const run = args => {
    const result = spawnSync(compiler, ['fmt', ...args], { cwd: scratch, encoding: 'utf8', timeout: 60000, maxBuffer: 1024 * 1024 });
    assert.ifError(result.error);
    assert.equal(result.signal, null);
    return result;
};
const manifest = (entry, dependencies = '') => `[project]\nname = "formatter-fixture"\nentry = "${entry}"\n${dependencies}`;
const packageAt = (name, source, dependencies = '') => {
    const directory = join(scratch, name);
    mkdirSync(join(directory, 'src'), { recursive: true });
    writeFileSync(join(directory, 'sovra.toml'), manifest('src/main.svr', dependencies));
    writeFileSync(join(directory, 'src/main.svr'), source);
    return directory;
};
try {
    const provider = packageAt('provider', 'mod api{export fn value()->Int{return 7;}}');
    const consumer = packageAt('consumer', 'use first::api;use second::api;fn main(){print(first::api::value());}',
        '[dependencies.first]\npath = "../provider"\n[dependencies.second]\npath = "../provider"\n');
    // Graph entry files only: no recursive source discovery or application scanner.
    writeFileSync(join(consumer, 'unrelated.svr'), 'not executable syntax');
    const paths = [join(consumer, 'sovra.toml'), join(consumer, 'src/main.svr'), join(provider, 'sovra.toml'), join(provider, 'src/main.svr')];
    const snapshots = paths.map(path => ({ bytes: readFileSync(path), time: statSync(path, { bigint: true }).mtimeNs }));
    const check = run(['--package', '--check', consumer]);
    assert.equal(check.status, 1, check.stderr);
    assert.equal(check.stdout, '');
    assert.equal((check.stderr.match(/Formatting needed:/g) ?? []).length, 2, check.stderr);
    for (const [index, path] of paths.entries()) {
        assert.deepEqual(readFileSync(path), snapshots[index].bytes);
        assert.equal(statSync(path, { bigint: true }).mtimeNs, snapshots[index].time);
    }
    const broken = join(provider, 'src/main.svr');
    writeFileSync(broken, 'fn main(){let x=;}');
    const failure = run(['--check', '--package', consumer]);
    assert.equal(failure.status, 1);
    assert.equal(failure.stdout, '');
    assert.ok(!failure.stderr.includes('Formatting needed:'));
    assert.ok(failure.stderr.includes(broken), failure.stderr);
    if (process.platform === 'linux' || process.platform === 'win32') {
        const failedWrite = run(['--package', '--write', consumer]);
        assert.equal(failedWrite.status, 1);
        assert.equal(failedWrite.stdout, '');
        assert.deepEqual(readFileSync(paths[1]), snapshots[1].bytes);
        assert.equal(readFileSync(broken, 'utf8'), 'fn main(){let x=;}');
    }
    writeFileSync(broken, snapshots[3].bytes);
    if (process.platform === 'linux') {
        chmodSync(broken, 0o440);
        assert.equal(run(['--package', '--write', consumer]).status, 1);
        assert.deepEqual(readFileSync(paths[1]), snapshots[1].bytes);
        chmodSync(broken, 0o640);
        const hard = join(provider, 'hard.svr');
        linkSync(broken, hard);
        assert.equal(run(['--package', '--write', consumer]).status, 1);
        assert.deepEqual(readFileSync(paths[1]), snapshots[1].bytes);
        rmSync(hard);
    }
    if (process.platform === 'linux' || process.platform === 'win32') {
        const written = run(['--package', '--write', consumer]);
        assert.equal(written.status, 0, written.stderr);
        assert.equal(written.stdout, '');
        assert.equal((written.stderr.match(/Formatted:/g) ?? []).length, 2, written.stderr);
        const beforeNoop = [paths[1], paths[3]].map(path => statSync(path, { bigint: true }));
        assert.equal(run(['--package', '--write', consumer]).status, 0);
        for (const [index, path] of [paths[1], paths[3]].entries()) {
            const after = statSync(path, { bigint: true });
            assert.equal(after.mtimeNs, beforeNoop[index].mtimeNs);
            assert.equal(after.ino, beforeNoop[index].ino);
        }
        assert.deepEqual(readFileSync(paths[0]), snapshots[0].bytes);
        assert.deepEqual(readFileSync(paths[2]), snapshots[2].bytes);
        assert.equal(readFileSync(join(consumer, 'unrelated.svr'), 'utf8'), 'not executable syntax');
    } else {
        assert.equal(run(['--package', '--write', consumer]).status, 1);
        for (const path of [paths[1], paths[3]]) {
            const output = run([path]);
            assert.equal(output.status, 0, output.stderr);
            writeFileSync(path, output.stdout);
        }
    }
    const clean = run(['--check', '--package', consumer]);
    assert.equal(clean.status, 0, clean.stderr);
    assert.equal(clean.stdout, '');
    assert.equal(clean.stderr, '');
    // Syntactic formatting checks do not require valid names/types or main.
    const unresolved = packageAt('unresolved', 'use missing::api; fn f(){unknown();}');
    const syntaxOnly = run(['--package', '--check', unresolved]);
    assert.equal(syntaxOnly.status, 1);
    assert.match(syntaxOnly.stderr, /Formatting needed:/);
    for (const args of [['--package', consumer], ['--package', '--package', '--check', consumer], ['--package', '--check', paths[1]], ['--check', consumer]]) {
        assert.equal(run(args).status, 2, args.join(' '));
    }
    // Missing graph dependencies are resolver errors with no partial report.
    writeFileSync(paths[0], manifest('src/main.svr', '[dependencies.absent]\npath = "../absent"\n'));
    const missing = run(['--package', '--check', consumer]);
    assert.equal(missing.status, 1);
    assert.ok(!missing.stderr.includes('Formatting needed:'));
    writeFileSync(paths[0], snapshots[0].bytes);
    if (process.platform !== 'win32') {
        const manifestCopy = join(provider, 'original.toml');
        writeFileSync(manifestCopy, snapshots[2].bytes);
        rmSync(paths[2]);
        symlinkSync(manifestCopy, paths[2]);
        const linked = run(['--package', '--check', consumer]);
        assert.equal(linked.status, 1);
        assert.match(linked.stderr, /regular file|link/);
    }
    // The formatter's graph cap is deliberately smaller than resolver admission.
    const large = packageAt('large', 'fn main() {}\n');
    let dependencies = '';
    for (let i = 0; i < 128; i++) {
        packageAt(`dep${i}`, '');
        dependencies += `[dependencies.d${i}]\npath = "../dep${i}"\n`;
    }
    writeFileSync(join(large, 'sovra.toml'), manifest('src/main.svr', dependencies));
    const capped = run(['--package', '--check', large]);
    assert.equal(capped.status, 1);
    assert.match(capped.stderr, /128 entries/);
    assert.ok(!capped.stderr.includes('Formatting needed:'));
    const budget = packageAt('budget', '');
    const megabyte = '//' + 'x'.repeat(1024 * 1024 - 3) + '\n';
    let budgetDependencies = '';
    for (let i = 0; i < 16; i++) {
        packageAt(`bytes${i}`, megabyte);
        budgetDependencies += `[dependencies.b${i}]\npath = "../bytes${i}"\n`;
    }
    writeFileSync(join(budget, 'sovra.toml'), manifest('src/main.svr', budgetDependencies));
    const bytesCapped = run(['--package', '--check', budget]);
    assert.equal(bytesCapped.status, 1);
    assert.match(bytesCapped.stderr, /cumulative byte limit/);
    assert.ok(!bytesCapped.stderr.includes('Formatting needed:'));
    assert.ok(!readdirSync(consumer).some(name => name.startsWith('.svr-fmt-')));
    console.log('Package formatter passed: graph selection/deduplication, nested entries, no writes, clean/dirty, source-owned errors, malformed options, missing dependencies, link refusal, guarded writes/no-ops/preflight failures and entry/byte quotas.');
} finally { rmSync(scratch, { recursive: true, force: true }); }
