import { inflateRawSync } from 'node:zlib';

export function crc32(bytes) {
    let value = 0xffffffff;
    for (const byte of bytes) {
        value ^= byte;
        for (let bit = 0; bit < 8; bit++) value = (value >>> 1) ^ ((value & 1) ? 0xedb88320 : 0);
    }
    return (value ^ 0xffffffff) >>> 0;
}

// Read bounded ZIP members for comparison only; never extract archive paths.
export function verifyIdeVsix(bytes, expected, ide) {
    if (bytes.length < 22 || bytes.length > 8 * 1024 * 1024) throw new Error('Invalid VSIX size');
    let end = bytes.length - 22;
    const minimum = Math.max(0, bytes.length - 65557);
    while (end >= minimum && bytes.readUInt32LE(end) !== 0x06054b50) end--;
    if (end < minimum || end + 22 + bytes.readUInt16LE(end + 20) !== bytes.length) throw new Error('Invalid VSIX directory');
    const count = bytes.readUInt16LE(end + 10);
    let cursor = bytes.readUInt32LE(end + 16);
    if (count > 128 || bytes.readUInt16LE(end + 4) || bytes.readUInt16LE(end + 6) || bytes.readUInt16LE(end + 8) !== count || cursor + bytes.readUInt32LE(end + 12) !== end) throw new Error('Unsupported VSIX directory');
    const found = new Set();
    for (let index = 0; index < count; index++) {
        if (cursor + 46 > end || bytes.readUInt32LE(cursor) !== 0x02014b50) throw new Error('Invalid VSIX entry');
        const nameLength = bytes.readUInt16LE(cursor + 28);
        const next = cursor + 46 + nameLength + bytes.readUInt16LE(cursor + 30) + bytes.readUInt16LE(cursor + 32);
        if (next > end || bytes.readUInt16LE(cursor + 8) & 1) throw new Error('Invalid VSIX entry');
        const name = new TextDecoder('utf-8', { fatal: true }).decode(bytes.subarray(cursor + 46, cursor + 46 + nameLength)).toLowerCase();
        if (found.has(name) || (!expected.has(name) && !['[content_types].xml', 'extension.vsixmanifest'].includes(name))) throw new Error(`Unexpected VSIX member: ${name}`);
        found.add(name);
        const offset = bytes.readUInt32LE(cursor + 42);
        const compressed = bytes.readUInt32LE(cursor + 20), size = bytes.readUInt32LE(cursor + 24), method = bytes.readUInt16LE(cursor + 10);
        if (size > 1024 * 1024 || offset + 30 > end || bytes.readUInt32LE(offset) !== 0x04034b50) throw new Error('Invalid VSIX payload');
        const start = offset + 30 + bytes.readUInt16LE(offset + 26) + bytes.readUInt16LE(offset + 28);
        if (start + compressed > end || ![0, 8].includes(method)) throw new Error('Unsupported VSIX payload');
        const data = method === 0 ? bytes.subarray(start, start + compressed) : inflateRawSync(bytes.subarray(start, start + compressed), { maxOutputLength: 1024 * 1024 });
        if (data.length !== size || crc32(data) !== bytes.readUInt32LE(cursor + 16) || (expected.has(name) && !data.equals(expected.get(name)))) throw new Error(`VSIX differs from current source: ${name}`);
        if (name === 'extension.vsixmanifest') {
            const xml = data.toString('utf8');
            for (const [key, value] of Object.entries({ Id: ide.name, Version: ide.version, Publisher: ide.publisher })) if (!xml.includes(`${key}="${value}"`)) throw new Error('VSIX identity differs from source');
        }
        cursor = next;
    }
    if (cursor !== end || !found.has('extension.vsixmanifest') || !found.has('[content_types].xml') || [...expected.keys()].some(name => !found.has(name))) throw new Error('VSIX source files missing');
}
