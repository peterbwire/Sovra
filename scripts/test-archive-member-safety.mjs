// Reject archive links before either release verifier extracts downloaded bytes.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { gzipSync } from 'node:zlib';

function header(name, type, link = '') {
    const bytes = Buffer.alloc(512);
    bytes.write(name, 0, 100, 'utf8');
    bytes.write('0000755\0', 100, 'ascii');
    bytes.write('0000000\0', 108, 'ascii');
    bytes.write('0000000\0', 116, 'ascii');
    bytes.write('00000000000\0', 124, 'ascii');
    bytes.write('00000000000\0', 136, 'ascii');
    bytes.fill(0x20, 148, 156);
    bytes.write(type, 156, 'ascii');
    bytes.write(link, 157, 100, 'utf8');
    bytes.write('ustar\0', 257, 'ascii');
    bytes.write('00', 263, 'ascii');
    const checksum = bytes.reduce((sum, byte) => sum + byte, 0);
    bytes.write(`${checksum.toString(8).padStart(6, '0')}\0 `, 148, 'ascii');
    return bytes;
}

const scratch = mkdtempSync(join(tmpdir(), 'sovra-archive-members-'));
try {
    for (const item of [
        { name: 'sovra-1.0.0-test-target', file: 'verify-candidate-archive.mjs', option: '--archive',
            extension: '.tar.gz', args: ['--name', 'sovra-1.0.0-test-target', '--version', '1.0.0'] },
        { name: 'sovra-1.0.0', file: 'verify-source-package.mjs', option: '--crate',
            extension: '.crate', args: ['--version', '1.0.0'] },
    ]) {
        for (const [kind, type] of [['symlink', '2'], ['hardlink', '1']]) {
            const work = join(scratch, kind);
            mkdirSync(work, { recursive: true });
            const archive = join(work, `${item.name}${item.extension}`);
            const bytes = gzipSync(Buffer.concat([
                header(`${item.name}/`, '5'),
                header(`${item.name}/escape`, type, 'README.md'),
                Buffer.alloc(1024),
            ]));
            writeFileSync(archive, bytes);
            const digest = createHash('sha256').update(bytes).digest('hex');
            writeFileSync(`${archive}.sha256`, `${digest}  ${item.name}${item.extension}\n`);
            const result = spawnSync(process.execPath,
                [join(import.meta.dirname, item.file), item.option, archive, ...item.args],
                { cwd: scratch, encoding: 'utf8', timeout: 10000, windowsHide: true });
            assert.ifError(result.error);
            assert.notEqual(result.status, 0, `${item.file} accepted a ${kind} member`);
            assert.match(result.stderr, /unsupported .*archive member type/,
                `${item.file} did not reject the ${kind} before extraction:\n${result.stderr}`);
        }
    }
    console.log('PASS candidate and source archives reject symlinks and hardlinks before extraction');
} finally {
    rmSync(scratch, { recursive: true, force: true });
}
