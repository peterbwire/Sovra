// Isolated diagnostic probe, not a passing-test or production-safety claim.
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve, join, basename, dirname } from 'node:path';
import { spawnSync } from 'node:child_process';

const executable = resolve(process.argv[2] ?? `target/debug/svr${process.platform === 'win32' ? '.exe' : ''}`);
const directory = resolve('target/depth-probe');
mkdirSync(directory, { recursive: true });
const results = [];
for (const shape of ['grouping', 'calls', 'binary']) {
    for (const depth of [32, 128, 256, 512, 2048]) {
        const expression = shape === 'grouping' ? '('.repeat(depth) + '1' + ')'.repeat(depth)
            : shape === 'calls' ? 'identity('.repeat(depth) + '1' + ')'.repeat(depth)
            : Array(depth + 1).fill('1').join(' + ');
        const source = `fn identity(value: Int) -> Int { return value }\nfn main() { let value = ${expression}; }\n`;
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
