// Differential execution of the supported source subset in both backends.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
if (args.length !== 0 && (args.length !== 2 || args[0] !== '--compiler')) {
    throw new Error('Usage: node scripts/test-runtime-matrix.mjs [--compiler PATH]');
}
const compiler = args.length ? resolve(args[1]) :
    join(root, 'target', 'debug', process.platform === 'win32' ? 'svr.exe' : 'svr');
const directory = mkdtempSync(join(tmpdir(), 'sovra-runtime-matrix-'));
const normalize = text => text.replaceAll('\r\n', '\n');
function run(binary, command) {
    const result = spawnSync(binary, command, {
        cwd: root, encoding: 'utf8', timeout: 30000,
        maxBuffer: 4 * 1024 * 1024, windowsHide: true,
    });
    assert.ifError(result.error);
    assert.equal(result.signal, null);
    return { status: result.status, stdout: normalize(result.stdout), stderr: normalize(result.stderr) };
}

const cases = [
    {
        name: 'integer-and-precedence',
        source: 'fn main() { print(1 + 2 * 3); print(9223372036854775807); print(8 / 3); }',
        output: '7\n9223372036854775807\n2\n',
    },
    {
        name: 'unicode-ordering',
        source: 'fn main() { print("𐀀" < ""); print("猫" > "café"); print("é" == "e"); }',
        output: 'false\ntrue\nfalse\n',
    },
    {
        name: 'short-circuit',
        source: 'fn side() -> Bool { print("side"); return true; } fn main() { if (false && side()) { print("wrong"); } if (true || side()) { print("done"); } }',
        output: 'done\n',
    },
    {
        name: 'loops-and-functions',
        source: 'fn double(value: Int) -> Int { return value * 2; } fn main() { let mut sum = 0; let mut i = 0; while i < 4 { sum = sum + double(i); i = i + 1; } print(sum); }',
        output: '12\n',
    },
    {
        name: 'array-value-copies',
        source: 'fn main() { let source = [[1, 2], [3, 4]]; let mut copy = source; copy[0] = [9, 8]; print(source[0][0]); print(copy[0][0]); print(source == [[1, 2], [3, 4]]); }',
        output: '1\n9\ntrue\n',
    },
    {
        name: 'array-widening',
        source: 'fn main() { let ints = [1, 2]; let mut floats = [0.0]; floats = ints; floats[0] = 3; print(floats[0] / 2); print(ints[0] / 2); }',
        output: '1.5\n0\n',
    },
    {
        name: 'nominal-record-values',
        source: 'struct Point { x: Int, y: Int } fn main() { let original = Point { x: 2, y: 3 }; let copy = original; print(copy == Point { y: 3, x: 2 }); print(original.x); print(std::to_string(copy)); }',
        output: 'true\n2\nPoint { x: 2, y: 3 }\n',
    },
    {
        name: 'text-decoding',
        source: 'fn main() { let split = std::split_once("a|b|c", "|"); print(split.before); print(split.after); let parsed = std::parse_int("-42"); print(parsed.ok); print(parsed.value); }',
        output: 'a\nb|c\ntrue\n-42\n',
    },
    {
        name: 'utf8-byte-length',
        source: 'fn main() { print(std::len("é🐈")); print("a" + "é"); }',
        output: '6\naé\n',
    },
    {
        name: 'recursive-calls',
        source: 'fn factorial(n: Int) -> Int { if n <= 1 { return 1; } return n * factorial(n - 1); } fn main() { print(factorial(5)); }',
        output: '120\n',
    },
    {
        name: 'nested-nominal-equality',
        source: 'struct Point { x: Int } fn main() { let first = [Point { x: 1 }, Point { x: 2 }]; print(first == [Point { x: 1 }, Point { x: 2 }]); print(first != [Point { x: 1 }, Point { x: 3 }]); }',
        output: 'true\ntrue\n',
    },
    {
        name: 'simple-float-math',
        source: 'fn main() { print(1.0 / 2.0); print(1 + 2.5); }',
        output: '0.5\n3.5\n',
    },
    {
        name: 'float-negative-zero-display',
        source: 'struct Amount { value: Float } fn main() { let zero = 0.0 / (0.0 - 1.0); print(zero); print(std::to_string(zero)); print([zero]); print(Amount { value: zero }); }',
        output: '-0\n-0\n[-0]\nAmount { value: -0 }\n',
    },
    {
        name: 'float-exponent-thresholds',
        source: 'fn main() { print(0.0000001); print(1000000000000000000000.0); }',
        output: '0.0000001\n1000000000000000000000\n',
    },
    {
        name: 'float-nonfinite-display',
        source: `fn main() { let huge = ${'1' + '0'.repeat(308)}.0; let infinite = huge * 10.0; print(infinite); print(0.0 - infinite); print(infinite - infinite); }`,
        output: 'inf\n-inf\nNaN\n',
    },
    {
        name: 'overflow-after-output',
        source: 'fn main() { print("before"); print(9223372036854775807 + 1); }',
        output: 'before\n', error: 'integer overflow',
    },
    {
        name: 'division-by-zero',
        source: 'fn main() { print(1 / 0); }',
        output: '', error: 'division by zero',
    },
    {
        name: 'integer-underflow',
        source: 'fn main() { print((0 - 9223372036854775807) - 2); }',
        output: '', error: 'integer overflow',
    },
    {
        name: 'argument-out-of-bounds',
        source: 'fn main() { print(std::arg(0)); }',
        output: '', error: 'program argument index out of bounds',
    },
    {
        name: 'index-out-of-bounds',
        source: 'fn main() { let values = [1]; print(values[2]); }',
        output: '', error: 'array index out of bounds',
    },
];

try {
    for (const scenario of cases) {
        const source = join(directory, `${scenario.name}.svr`);
        const js = join(directory, `${scenario.name}.cjs`);
        writeFileSync(source, scenario.source + '\n');
        const checked = run(compiler, ['check', source]);
        assert.equal(checked.status, 0, `${scenario.name}: ${checked.stderr}`);
        const built = run(compiler, ['build', '--emit', 'js', source]);
        assert.equal(built.status, 0, `${scenario.name}: ${built.stderr}`);
        writeFileSync(js, built.stdout);
        for (const [engine, binary, command] of [
            ['interpreter', compiler, ['run', source]],
            ['javascript', process.execPath, [js]],
        ]) {
            const result = run(binary, command);
            assert.equal(result.stdout, scenario.output, `${scenario.name}/${engine} output`);
            if (scenario.error) {
                assert.notEqual(result.status, 0, `${scenario.name}/${engine} should fail`);
                assert.ok(result.stderr.includes(scenario.error),
                    `${scenario.name}/${engine}: ${result.stderr}`);
            } else {
                assert.equal(result.status, 0, `${scenario.name}/${engine}: ${result.stderr}`);
                assert.equal(result.stderr, '', `${scenario.name}/${engine} stderr`);
            }
        }
        console.log(`PASS runtime matrix/${scenario.name}`);
    }
    const literals = [
        '0.0', '1.0', '1.2345678901234567', '0.12345678901234567',
        '123456789012345.67', '999999999999999999999.0',
        '0.' + '0'.repeat(323) + '5',
    ];
    for (const power of [1, 2, 6, 7, 15, 20, 21, 22, 50, 100, 200, 308]) {
        literals.push('1' + '0'.repeat(power) + '.0');
        literals.push('0.' + '0'.repeat(power - 1) + '1');
    }
    let seed = 0x5eed1234n;
    for (let index = 0; index < 100; index++) {
        seed = (seed * 6364136223846793005n + 1n) & ((1n << 64n) - 1n);
        const digits = String(10000000000000000n + seed % 90000000000000000n);
        const exponent = (index * 37) % 580 - 290;
        if (exponent >= 0) {
            literals.push(digits + '0'.repeat(exponent) + '.0');
        } else {
            const point = digits.length + exponent;
            literals.push(point > 0 ? digits.slice(0, point) + '.' + digits.slice(point) :
                '0.' + '0'.repeat(-point) + digits);
        }
    }
    const source = join(directory, 'float-corpus.svr');
    const js = join(directory, 'float-corpus.cjs');
    writeFileSync(source, 'fn main() {\n' + literals.map(value => `    print(${value});`).join('\n') + '\n}\n');
    const built = run(compiler, ['build', '--emit', 'js', source]);
    assert.equal(built.status, 0, built.stderr);
    writeFileSync(js, built.stdout);
    const rust = run(compiler, ['run', source]);
    const javascript = run(process.execPath, [js]);
    assert.equal(rust.status, 0, rust.stderr);
    assert.equal(javascript.status, 0, javascript.stderr);
    assert.equal(javascript.stdout, rust.stdout, 'Float literal corpus differs by backend');
    assert.equal(rust.stdout.trimEnd().split('\n').length, literals.length);
    console.log(`PASS runtime matrix/float-corpus (${literals.length} values)`);
} finally {
    rmSync(directory, { recursive: true, force: true });
}
