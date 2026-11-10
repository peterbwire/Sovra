// Deterministic adversarial regressions, not a fuzzing or sandbox certification.
import { mkdtempSync, writeFileSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const compiler = resolve(process.argv[2] ?? `target/debug/svr${process.platform === 'win32' ? '.exe' : ''}`);
const directory = mkdtempSync(join(tmpdir(), 'sovra-compiler-security-'));
const cases = [
    ['long-else-chain', `fn main() { ${'if (false) {} else '.repeat(20000)}print(1); }`, 'E2007'],
    ['mixed-depth', `fn main() { ${'while (false) {'.repeat(64)}${'if (false) {} else '.repeat(64)}print(1); ${'}'.repeat(64)} }`, 'E2007'],
    ['array-depth', `fn main() { ${'['.repeat(129)}1${']'.repeat(129)}; }`, 'E2007'],
    ['grouping-depth', `fn main() { ${'('.repeat(129)}1${')'.repeat(129)}; }`, 'E2007'],
    ['integer-range', 'fn main() { print(9223372036854775808); }', 'E3012'],
    ['malformed-declaration', 'fn main( { let = ; }', 'E2002'],
];
let checks = 0;
try {
    for (const [name, source, code] of cases) {
        const file = join(directory, `${name}.svr`);
        writeFileSync(file, source);
        for (const args of [['check', '--format', 'json'], ['run'], ['build'], ['build', '--emit', 'js']]) {
            const result = spawnSync(compiler, [...args, file], {
                encoding: 'utf8', timeout: 10000, maxBuffer: 1024 * 1024,
                windowsHide: true,
            });
            if (result.error || result.signal || result.status !== 1) {
                throw new Error(`${name}/${args.join(' ')}: abnormal rejection: ${result.error ?? result.signal ?? result.status}`);
            }
            if (args[0] === 'check') {
                const report = JSON.parse(result.stdout);
                if (report.success !== false || !report.diagnostics.some(item => item.code === code)) {
                    throw new Error(`${name}: missing ${code}: ${result.stdout}`);
                }
                if (code === 'E2007' && report.diagnostics.length !== 1) {
                    throw new Error(`${name}: depth rejection cascaded`);
                }
            } else if (result.stdout !== '' || !result.stderr.includes(code)) {
                throw new Error(`${name}/${args.join(' ')}: emitted output or missing ${code}`);
            }
            if (readFileSync(file, 'utf8') !== source) throw new Error(`${name}: source changed`);
            checks++;
        }
    }
    console.log(`Compiler security regressions passed: ${checks} isolated checks; no crashes, timeouts or emitted code.`);
} finally {
    rmSync(directory, { recursive: true, force: true });
}
