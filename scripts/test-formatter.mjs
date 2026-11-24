// Exercise the real CLI on temporary copies; repository sources remain untouched.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { chmodSync, cpSync, linkSync, lstatSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
assert.ok(args.length === 0 || (args.length === 2 && args[0] === '--compiler'));
const compiler = resolve(root, args[1] ?? `target/debug/svr${process.platform === 'win32' ? '.exe' : ''}`);
const scratch = mkdtempSync(join(tmpdir(), 'sovra-formatter-'));
function execute(binary, argv) {
    const result = spawnSync(binary, argv, {
        cwd: scratch, encoding: 'utf8', timeout: 60000, maxBuffer: 8 * 1024 * 1024, windowsHide: true,
    });
    assert.ifError(result.error);
    assert.equal(result.signal, null);
    return { status: result.status, stdout: result.stdout, stderr: result.stderr };
}
const svr = argv => execute(compiler, argv);
try {
    const corpus = [
        'examples/hello-world/main.svr', 'examples/functions/main.svr',
        'examples/modules/main.svr', 'examples/records/main.svr',
        'examples/strings/main.svr', 'examples/scoped-types/main.svr',
        'dogfood/01-task-manager/main.svr', 'examples/ui-desktop/component.svr',
    ];
    for (const [index, path] of corpus.entries()) {
        const original = join(root, path);
        const before = readFileSync(original);
        const formatted = svr(['fmt', original]);
        assert.equal(formatted.status, 0, `${path}: ${formatted.stderr}`);
        assert.equal(formatted.stderr, '');
        assert.deepEqual(readFileSync(original), before);
        const copy = join(scratch, `${index}.svr`);
        writeFileSync(copy, formatted.stdout);
        assert.equal(svr(['fmt', '--check', copy]).status, 0);
        assert.equal(svr(['fmt', copy]).stdout, formatted.stdout);
        const states = [join(scratch, `${index}-before.txt`), join(scratch, `${index}-after.txt`)];
        const executions = [];
        for (const [version, source] of [original, copy].entries()) {
            const built = svr(['build', '--emit', 'js', source]);
            assert.equal(built.status, 0, `${path}: ${built.stderr}`);
            const generated = join(scratch, `${index}-${version}.cjs`);
            writeFileSync(generated, built.stdout);
            const commands = path.includes('task-manager')
                ? [['add', 'Sovra formatting'], ['complete', 'Sovra formatting'], ['list']]
                : path.includes('component.svr') ? [['ui-event', 'save-message', 'Formatted component'], ['ui-layout-view']]
                    : [[]];
            const engineResults = [];
            for (const engine of ['interpreter', 'javascript']) {
                const state = `${states[version]}-${engine}`;
                const results = [];
                for (const command of commands) {
                    const appArgs = command.length === 0 ? [] : [command[0], state, ...command.slice(1)];
                    const result = engine === 'interpreter'
                        ? svr(['run', source, '--', ...appArgs])
                        : execute(process.execPath, [generated, ...appArgs]);
                    assert.equal(result.status, 0, `${path} ${engine}: ${result.stderr}${result.stdout}`);
                    results.push(result.stdout.replaceAll('\r\n', '\n'));
                }
                engineResults.push(results);
                if (commands[0].length) results.push(readFileSync(state, 'utf8'));
            }
            assert.deepEqual(engineResults[0], engineResults[1], `${path}: execution engines differ`);
            executions.push(engineResults);
        }
        assert.deepEqual(executions[0], executions[1], `${path}: formatting changed execution`);
    }
    if (process.platform === 'linux' || process.platform === 'win32') {
        const path = join(scratch, 'write qualification.svr');
        const source = '// Unicode λ\r\nfn main(){print("unchanged behavior");}';
        writeFileSync(path, source);
        if (process.platform === 'linux') chmodSync(path, 0o640);
        const expected = svr(['fmt', path]);
        assert.equal(expected.status, 0);
        const permissions = statSync(path).mode;
        const written = svr(['fmt', '--write', path]);
        assert.equal(written.status, 0, written.stderr);
        assert.equal(written.stdout, '');
        assert.equal(readFileSync(path, 'utf8'), expected.stdout);
        assert.equal(statSync(path).mode, permissions);
        const before = statSync(path, { bigint: true });
        assert.equal(svr(['fmt', '--write', path]).status, 0);
        const after = statSync(path, { bigint: true });
        assert.equal(after.mtimeNs, before.mtimeNs);
        assert.equal(after.ino, before.ino);
        if (process.platform === 'linux') {
            chmodSync(path, 0o440);
            assert.equal(svr(['fmt', '--write', path]).status, 1);
            chmodSync(path, 0o640);
            const hard = join(scratch, 'hard.svr');
            linkSync(path, hard);
            assert.equal(svr(['fmt', '--write', path]).status, 1);
            rmSync(hard);
            const link = join(scratch, 'linked.svr');
            symlinkSync(path, link);
            assert.equal(svr(['fmt', '--write', link]).status, 1);
            assert.ok(lstatSync(link).isSymbolicLink());
            chmodSync(path, 0o2640);
            assert.equal(svr(['fmt', '--write', path]).status, 1);
            chmodSync(path, 0o640);
            assert.equal(readFileSync(path, 'utf8'), expected.stdout);
        }
        assert.ok(!readdirSync(scratch).some(name => name.startsWith('.svr-fmt-')));
    }
    if (process.platform !== 'linux' && process.platform !== 'win32') {
        const path = join(scratch, 'disabled write.svr');
        const source = 'fn main(){print(1);}';
        writeFileSync(path, source);
        const before = statSync(path, { bigint: true });
        const refused = svr(['fmt', '--write', path]);
        assert.equal(refused.status, 1);
        assert.equal(refused.stdout, '');
        assert.match(refused.stderr, /qualified only/);
        assert.equal(readFileSync(path, 'utf8'), source);
        assert.equal(statSync(path, { bigint: true }).mtimeNs, before.mtimeNs);
    }
    // Format every entry in a copied real package graph, without touching manifests.
    const packageCopy = join(scratch, 'local packages');
    cpSync(join(root, 'examples/local-packages'), packageCopy, { recursive: true });
    const app = join(packageCopy, 'app');
    const packageResults = () => {
        const interpreted = svr(['run', app]);
        assert.equal(interpreted.status, 0, interpreted.stderr);
        const compiled = svr(['build', '--emit', 'js', app]);
        assert.equal(compiled.status, 0, compiled.stderr);
        const generated = join(scratch, 'package.js');
        writeFileSync(generated, compiled.stdout);
        const javascript = execute(process.execPath, [generated]);
        assert.equal(javascript.status, 0, javascript.stderr);
        assert.equal(interpreted.stdout.replaceAll('\r\n', '\n'), javascript.stdout.replaceAll('\r\n', '\n'));
        return interpreted.stdout;
    };
    const beforePackage = packageResults();
    for (const relative of ['app/main.svr', 'utilities/library.svr']) {
        const path = join(packageCopy, relative);
        const manifest = join(dirname(path), 'sovra.toml');
        const manifestBytes = readFileSync(manifest);
        const formatted = svr(['fmt', path]);
        assert.equal(formatted.status, 0, formatted.stderr);
        if (process.platform === 'linux' || process.platform === 'win32') {
            const written = svr(['fmt', '--write', path]);
            assert.equal(written.status, 0, written.stderr);
            assert.equal(readFileSync(path, 'utf8'), formatted.stdout);
        } else writeFileSync(path, formatted.stdout);
        assert.equal(svr(['fmt', '--check', path]).status, 0);
        assert.deepEqual(readFileSync(manifest), manifestBytes);
    }
    assert.equal(packageResults(), beforePackage);
    const importTokens = ['use', 'missing', '::', 'tools', ';', 'fn', 'main', '(', ')', '{', '}'];
    const importPath = join(scratch, 'unresolved imports.svr');
    for (let position = 0; position <= importTokens.length; position++) {
        const source = importTokens.slice(0, position).join(' ') + ' // λ retained\r\n'
            + importTokens.slice(position).join(' ');
        writeFileSync(importPath, source);
        const formatted = svr(['fmt', importPath]);
        assert.equal(formatted.status, 0, formatted.stderr);
        assert.equal(readFileSync(importPath, 'utf8'), source);
        writeFileSync(importPath, formatted.stdout);
        assert.equal(svr(['fmt', '--check', importPath]).status, 0);
    }
    const triviaSource = 'export struct P{x:Int,y:Int} fn sum(p:P)->Int{return p.x+p.y;} fn main(){let p=P{x:1,y:2};let a=[sum(p),3];if a[0]>0{print("ok");}else{print("no");}}';
    const triviaTokens = triviaSource.match(/"(?:[^"\\]|\\.)*"|[A-Za-z_][A-Za-z_0-9]*|[0-9]+|->|::|==|!=|>=|<=|&&|\|\||[^\s]/g);
    const triviaPath = join(scratch, 'trivia.svr');
    let triviaCases = 0;
    for (let position = 0; position <= triviaTokens.length; position++) {
        for (const gap of [' // retained\n', '\n// retained\n', '\n\n// retained\n\n']) {
            const source = triviaTokens.slice(0, position).join(' ') + gap
                + triviaTokens.slice(position).join(' ');
            writeFileSync(triviaPath, source);
            const formatted = svr(['fmt', triviaPath]);
            assert.equal(formatted.status, 0, `Trivia position ${position}: ${formatted.stderr}`);
            assert.match(formatted.stdout, /\/\/ retained\n/);
            assert.equal(readFileSync(triviaPath, 'utf8'), source);
            triviaCases++;
        }
    }
    for (const [name, source] of [
        ['imports', 'use lib::tools fn main(){}'],
        ['oversized-input', ' '.repeat(1024 * 1024 + 1)],
        ['oversized-output', `fn main(){${'if true{'.repeat(64)}${'print(1);'.repeat(18000)}${'}'.repeat(65)}`],
    ]) {
        const path = join(scratch, `${name}.svr`);
        writeFileSync(path, source);
        const result = svr(['fmt', path]);
        assert.equal(result.status, 1, name);
        assert.equal(result.stdout, '', name);
        assert.equal(readFileSync(path, 'utf8'), source);
        assert.match(result.stderr, name === 'imports' ? /import|use/ : /limit|budget|large|MiB/);
    }
    console.log(`Formatter passed: ${corpus.length} real programs, both engines, ${triviaCases} comment placements, import trivia, copied-package behavior, fixed points and resource failures.`);
} finally {
    rmSync(scratch, { recursive: true, force: true });
}
