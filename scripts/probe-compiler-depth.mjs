// Isolated diagnostic probe, not a passing-test or production-safety claim.
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve, join, basename, dirname } from 'node:path';
import { spawnSync } from 'node:child_process';

const arguments_ = process.argv.slice(2);
const enforceLimits = arguments_.includes('--assert-limits');
const executable = resolve(arguments_.find(value => value !== '--assert-limits') ?? `target/debug/svr${process.platform === 'win32' ? '.exe' : ''}`);
const directory = resolve('target/depth-probe');
mkdirSync(directory, { recursive: true });
const results = [];
for (const shape of ['grouping', 'calls', 'binary', 'arrays', 'if-blocks', 'while-blocks']) {
    for (const depth of [32, 127, 128, 129, 256, 512, 2048]) {
        const expression = shape === 'grouping' ? '('.repeat(depth) + '1' + ')'.repeat(depth)
            : shape === 'calls' ? 'identity('.repeat(depth) + '1' + ')'.repeat(depth)
            : shape === 'arrays' ? '['.repeat(depth) + '1' + ']'.repeat(depth)
            : Array(depth + 1).fill('1').join(' + ');
        const source = shape.endsWith('-blocks')
            ? `fn main() { ${(shape === 'if-blocks' ? 'if (true) {' : 'while (false) {').repeat(depth)} let value = 1; ${'}'.repeat(depth)} }`
            : `fn identity(value: Int) -> Int { return value }\nfn main() { let value = ${expression}; }\n`;
        const file = join(directory, `${shape}-${depth}.svr`);
        writeFileSync(file, source);
        for (const command of ['check', 'build']) {
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
        }
    }
}
writeFileSync(join(directory, `results-${basename(dirname(executable))}.json`), JSON.stringify({
    executable, platform: process.platform, architecture: process.arch, results,
}, null, 2) + '\n');
if (enforceLimits) {
    const failed = results.filter(result => {
        const accepted = result.depth <= (result.shape === 'grouping' ? 128 : 127);
        return result.error !== null || result.signal !== null ||
            result.exit !== (accepted ? 0 : 1) || (!accepted && !result.stderr.includes('E2007'));
    });
    if (failed.length) {
        console.error(`${failed.length} structural-depth probe cases violated the expected limit behavior`);
        process.exitCode = 1;
    }
}
