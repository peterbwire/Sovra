// Exercise the real CLI on temporary copies; repository sources remain untouched.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
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
        ['imports', 'use lib::tools; fn main(){}'],
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
    console.log(`Formatter passed: ${corpus.length} real programs, both engines, ${triviaCases} comment placements, fixed points and resource failures.`);
} finally {
    rmSync(scratch, { recursive: true, force: true });
}
