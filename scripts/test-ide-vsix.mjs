import assert from 'node:assert/strict';
import { verifyIdeVsix, crc32 } from './verify-ide-vsix.mjs';

assert.equal(crc32(Buffer.from('123456789')), 0xcbf43926);
function zip(entries) {
    const local = [], central = []; let offset = 0;
    for (const [name, text] of entries) {
        const file = Buffer.from(name), data = Buffer.from(text), crc = crc32(data);
        const header = Buffer.alloc(30); header.writeUInt32LE(0x04034b50); header.writeUInt16LE(20, 4);
        header.writeUInt32LE(crc, 14); header.writeUInt32LE(data.length, 18); header.writeUInt32LE(data.length, 22); header.writeUInt16LE(file.length, 26);
        local.push(header, file, data);
        const directory = Buffer.alloc(46); directory.writeUInt32LE(0x02014b50); directory.writeUInt16LE(20, 4); directory.writeUInt16LE(20, 6);
        directory.writeUInt32LE(crc, 16); directory.writeUInt32LE(data.length, 20); directory.writeUInt32LE(data.length, 24); directory.writeUInt16LE(file.length, 28); directory.writeUInt32LE(offset, 42);
        central.push(directory, file); offset += header.length + file.length + data.length;
    }
    const directory = Buffer.concat(central), end = Buffer.alloc(22); end.writeUInt32LE(0x06054b50);
    end.writeUInt16LE(entries.length, 8); end.writeUInt16LE(entries.length, 10); end.writeUInt32LE(directory.length, 12); end.writeUInt32LE(offset, 16);
    return Buffer.concat([...local, directory, end]);
}
const ide = { name: 'sovra-language', version: '0.3.0', publisher: 'sovra-local' };
const entries = [['[Content_Types].xml', '<Types/>'], ['extension.vsixmanifest', '<Identity Id="sovra-language" Version="0.3.0" Publisher="sovra-local"/>'], ['extension/package.json', '{"name":"sovra-language"}']];
const expected = new Map([['extension/package.json', Buffer.from(entries[2][1])]]);
verifyIdeVsix(zip(entries), expected, ide);
for (const invalid of [
    zip([...entries, entries[2]]), zip([...entries, ['../outside', 'bad']]), zip(entries.slice(0, 2)),
    zip([...entries.slice(0, 2), ['extension/package.json', 'stale source']]),
    zip([entries[0], [entries[1][0], entries[1][1].replace('0.3.0', '0.2.0')], entries[2]]),
    zip([...entries.slice(0, 2), ['extension/package.json', 'x'.repeat(1024 * 1024 + 1)]]),
    Buffer.from('invalid ZIP'), zip(entries).subarray(0, 100),
]) assert.throws(() => verifyIdeVsix(invalid, expected, ide));
const damaged = zip(entries); damaged[damaged.indexOf(Buffer.from('<Types/>'))] ^= 1;
assert.throws(() => verifyIdeVsix(damaged, expected, ide));
console.log('VSIX admission: exact source, identity, duplicate/path/missing members, corruption, size limits and malformed directory passed.');
