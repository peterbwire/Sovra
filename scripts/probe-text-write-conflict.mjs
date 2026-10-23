// Default observes unconditional-write limits; --conditional verifies rejection.
import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
const conditional = args[0] === '--conditional';
if (conditional) args.shift();
assert.ok(args.length === 0 || (args.length === 2 && args[0] === '--compiler'),
    'Usage: node scripts/probe-text-write-conflict.mjs [--conditional] [--compiler PATH]');
const compiler = args.length ? resolve(args[1]) : join(root, 'target/debug', process.platform === 'win32' ? 'svr.exe' : 'svr');
const source = join(root, 'scripts/fixtures/text-write-conflict.svr');
const scratch = mkdtempSync(join(tmpdir(), 'sovra-write-conflict-'));
const evidence = join(root, 'target/text-write-conflict', conditional ? 'conditional.json' : 'observation.json');
const hash = path => createHash('sha256').update(readFileSync(path)).digest('hex');
const report = { schema_version: 1, kind: conditional ? 'conditional-text-conflict-test' : 'text-write-conflict-observation', status: 'running',
    cooperative_conditional_writes: conditional, concurrency_safe: false,
    started_at: new Date().toISOString(), schedules: [] };
mkdirSync(dirname(evidence), { recursive: true });
writeFileSync(evidence, JSON.stringify(report, null, 2) + '\n');
let failure;
try {
    report.compiler_sha256 = hash(compiler);
    report.source_sha256 = hash(source);
    const built = spawnSync(compiler, ['build', '--emit', 'js', source], {
        cwd: root, encoding: 'utf8', timeout: 30000, maxBuffer: 16 * 1024 * 1024, windowsHide: true,
    });
    assert.ifError(built.error);
    assert.equal(built.status, 0, built.stderr);
    const generated = join(scratch, 'program.cjs');
    writeFileSync(generated, built.stdout);
    const engines = {
        interpreter: [compiler, ['run', source, '--']],
        javascript: [process.execPath, [generated]],
    };
    function launch(engine, path, label, snapshot) {
        const [binary, prefix] = engines[engine];
        const child = spawn(binary, [...prefix, path, label, ...(conditional ? ['conditional'] : [])], { cwd: root, windowsHide: true });
        let output = '', error = '', ready = false, problem;
        child.stdout.setEncoding('utf8'); child.stderr.setEncoding('utf8');
        const stopped = new Promise(resolveStop => {
            child.once('error', reason => { problem = reason; });
            child.once('close', (code, signal) => resolveStop({ code, signal }));
        });
        const timer = setTimeout(() => { problem = new Error('Probe process timed out'); child.kill(); }, 15000);
        child.stdout.on('data', chunk => {
            output += chunk;
            ready = output.replaceAll('\r\n', '\n').includes(`snapshot:${snapshot}\n`);
            if (output.length > 16384) { problem = new Error('Probe output exceeded its bound'); child.kill(); }
        });
        child.stderr.on('data', chunk => {
            error += chunk;
            if (error.length > 16384) { problem = new Error('Probe stderr exceeded its bound'); child.kill(); }
        });
        child.stdin.on('error', reason => { problem = reason; });
        return {
            async ready() {
                const deadline = Date.now() + 10000;
                while (!ready && !problem && child.exitCode === null && Date.now() < deadline)
                    await new Promise(resolveTick => setTimeout(resolveTick, 10));
                assert.ifError(problem);
                assert.ok(ready, `Missing snapshot barrier: ${output} ${error}`);
            },
            async release() {
                child.stdin.end('go\n');
                const result = await stopped;
                clearTimeout(timer);
                assert.ifError(problem);
                assert.equal(result.signal, null);
                assert.equal(result.code, 0, error + output);
                return output.replaceAll('\r\n', '\n');
            },
            async cleanup() {
                if (child.exitCode === null && child.signalCode === null) child.kill();
                await stopped; clearTimeout(timer);
            },
        };
    }
    for (const initial of conditional ? ['base', null] : ['base'])
    for (const firstEngine of Object.keys(engines)) for (const secondEngine of Object.keys(engines)) {
        const path = join(scratch, `${initial === null ? 'absent' : 'existing'}-${firstEngine}-${secondEngine}.txt`);
        if (initial !== null) writeFileSync(path, initial);
        const children = [];
        try {
            const first = launch(firstEngine, path, 'first', initial ?? 'missing'); children.push(first);
            const second = launch(secondEngine, path, 'second', initial ?? 'missing'); children.push(second);
            await Promise.all([first.ready(), second.ready()]);
            assert.match(await first.release(), /saved\n/);
            assert.equal(readFileSync(path, 'utf8'), (initial ?? '') + '|first');
            const secondOutput = await second.release();
            assert.match(secondOutput, conditional ? /rejected:conflict\n/ : /saved\n/);
            const finalText = (initial ?? '') + (conditional ? '|first' : '|second');
            assert.equal(readFileSync(path, 'utf8'), finalText);
            report.schedules.push({ first: firstEngine, second: secondEngine,
                initially_exists: initial !== null,
                both_saved: !conditional, final_text: finalText, first_update_lost: !conditional });
        } finally { await Promise.all(children.map(child => child.cleanup())); }
    }
    report.status = conditional ? 'passed' : 'limitation_confirmed';
} catch (error) { failure = error; report.status = 'failed'; report.error = String(error); }
finally {
    try { rmSync(scratch, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 }); }
    catch (error) { failure ??= error; report.status = 'failed'; report.error = String(error); }
    report.finished_at = new Date().toISOString();
    writeFileSync(evidence, JSON.stringify(report, null, 2) + '\n');
}
if (failure) throw failure;
console.log(conditional ? `PASS conditional writes: ${report.schedules.length} same/mixed-engine stale snapshots rejected.` :
    `LIMITATION CONFIRMED: ${report.schedules.length} stale-snapshot schedules lose the first update; no concurrency guarantee is claimed.`);
