// Isolated depth evidence; --assert-limits enforces supported command behavior.
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve, join, basename, dirname } from 'node:path';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';

const arguments_ = process.argv.slice(2);
const enforceLimits = arguments_.includes('--assert-limits');
const executable = resolve(arguments_.find(value => value !== '--assert-limits') ?? `target/debug/svr${process.platform === 'win32' ? '.exe' : ''}`);
const directory = resolve('target/depth-probe');
mkdirSync(directory, { recursive: true });
const results = [];
for (const shape of ['grouping', 'calls', 'binary', 'arrays', 'array-inference', 'if-blocks', 'while-blocks']) {
    for (const depth of [32, 127, 128, 129, 256, 512, 2048]) {
        const expression = shape === 'grouping' ? '('.repeat(depth) + '1' + ')'.repeat(depth)
            : shape === 'calls' ? 'identity('.repeat(depth) + '1' + ')'.repeat(depth)
            : shape === 'arrays' ? '['.repeat(depth) + '1' + ']'.repeat(depth)
            : shape === 'array-inference' ? '[' + '['.repeat(depth - 1) + ']'.repeat(depth - 1) + ', ' +
                '['.repeat(depth - 1) + '1.0' + ']'.repeat(depth - 1) + ']'
            : Array(depth + 1).fill('1').join(' + ');
        const source = shape.endsWith('-blocks')
            ? `fn main() { ${(shape === 'if-blocks' ? 'if (true) {' : 'while (false) {').repeat(depth)} let value = 1; ${'}'.repeat(depth)} }`
            : `fn identity(value: Int) -> Int { return value }\nfn main() { let value = ${expression}; }\n`;
        const file = join(directory, `${shape}-${depth}.svr`);
        writeFileSync(file, source);
        for (const command of ['check', 'build', 'fmt']) {
            const start = Date.now();
            const result = spawnSync(executable, [command, file], {
                encoding: 'utf8', timeout: 10000, maxBuffer: 4 * 1024 * 1024,
                windowsHide: true,
            });
            const record = { shape, depth, command, exit: result.status,
                signal: result.signal, error: result.error?.code ?? null,
                milliseconds: Date.now() - start, stderr: result.stderr?.trim() ?? '' };
            results.push(record);
            console.log(JSON.stringify(record));
            if (command === 'fmt') {
                if (readFileSync(file, 'utf8') !== source) {
                    throw new Error('Formatting depth probe mutated its input');
                }
                if (result.status === 0) {
                    const formatted = join(directory, `${shape}-${depth}-formatted.svr`);
                    writeFileSync(formatted, result.stdout);
                    const checked = spawnSync(executable, ['fmt', '--check', formatted], {
                        encoding: 'utf8', timeout: 10000, windowsHide: true,
                    });
                    const fixedPoint = { shape, depth, command: 'fmt --check', exit: checked.status,
                        signal: checked.signal, error: checked.error?.code ?? null,
                        milliseconds: null, stderr: checked.stderr?.trim() ?? '' };
                    results.push(fixedPoint);
                    console.log(JSON.stringify(fixedPoint));
                } else if (result.stdout) {
                    throw new Error('Rejected formatting depth probe emitted partial source');
                }
            }
        }
    }
}
const failed = results.filter(result => {
    const accepted = result.depth <= (result.shape === 'grouping' ? 128 : 127);
    return result.error !== null || result.signal !== null ||
        result.exit !== (accepted ? 0 : 1) || (!accepted && !result.stderr.includes('E2007'));
});
writeFileSync(join(directory, `results-${basename(dirname(executable))}.json`), JSON.stringify({
    schema_version: 1, kind: 'compiler-depth-probe', executable,
    compiler_sha256: createHash('sha256').update(readFileSync(executable)).digest('hex'),
    platform: process.platform, architecture: process.arch, limits_enforced: enforceLimits,
    success: failed.length === 0, total: results.length, failed: failed.length, results,
}, null, 2) + '\n');
if (enforceLimits) {
    if (failed.length) {
        console.error(`${failed.length} structural-depth probe cases violated the expected limit behavior`);
        process.exitCode = 1;
    }
}
