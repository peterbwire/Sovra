// Checked links only: declaration indices never resolve by spelling or import text.
import { open } from 'node:fs/promises';
import { constants } from 'node:fs';
import { isAbsolute } from 'node:path';
import { pathToFileURL } from 'node:url';
import { createHash } from 'node:crypto';
import { hostPath } from './packages.mjs';
import { positionsFor } from './symbols.mjs';
const own = (o, key) => Object.hasOwn(o, key);
const string = s => typeof s === 'string' && s.isWellFormed() && s.length > 0 && Buffer.byteLength(s) <= 4096;
const integer = n => Number.isSafeInteger(n) && n >= 0;
async function closedSource(path, size) {
    // No symlink following or blocking special-file reads where these flags exist.
    const file = await open(path, constants.O_RDONLY | (constants.O_NOFOLLOW || 0) | (constants.O_NONBLOCK || 0));
    try {
        const stat = await file.stat();
        if (!stat.isFile() || stat.size !== size) throw new Error('Navigation source changed');
        const bytes = Buffer.alloc(size + 1); let count = 0;
        while (count < bytes.length) { const { bytesRead } = await file.read(bytes, count, bytes.length - count, null); if (!bytesRead) break; count += bytesRead; }
        if (count !== size) throw new Error('Navigation source changed');
        return bytes.subarray(0, size);
    } finally { await file.close(); }
}
export async function decodePackageNavigation(meta, entries, members) {
    if (meta === undefined) return undefined;
    if (meta?.schema_version !== 1 || typeof meta.unavailable !== 'boolean' || Buffer.byteLength(JSON.stringify(meta)) > 512 * 1024) throw new Error('Invalid package navigation envelope');
    if (meta.unavailable) { if (Object.keys(meta).length !== 2) throw new Error('Partial unavailable navigation'); return undefined; }
    if (!Array.isArray(meta.files) || !meta.files.length || meta.files.length > 32 || !Array.isArray(meta.declarations) || meta.declarations.length > 8192 || !Array.isArray(meta.references) || meta.references.length > 8192) throw new Error('Invalid navigation tables');
    let total = 0;
    const paths = new Set();
    for (const f of meta.files) {
        if (!string(f?.path) || !isAbsolute(f.path) || f.path.includes('\0') || !entries.has(hostPath(f.path)) || paths.has(hostPath(f.path)) || !integer(f.bytes) || f.bytes > 512 * 1024 || typeof f.sha256 !== 'string' || !/^[a-f0-9]{64}$/.test(f.sha256)) throw new Error('Invalid navigation source identity');
        paths.add(hostPath(f.path)); total += f.bytes;
        if (total > 4 * 1024 * 1024) throw new Error('Navigation source budget exceeded');
    }
    for (const path of members.keys()) if (!paths.has(path)) throw new Error('Incomplete navigation source table');
    const files = [];
    // Unavailable/changed closed files disable the entire optional projection.
    for (const f of meta.files) {
        const path = hostPath(f.path), doc = members.get(path);
        let bytes;
        try { bytes = doc ? Buffer.from(doc.text) : await closedSource(path, f.bytes); }
        catch { return undefined; }
        if (bytes.length !== f.bytes || createHash('sha256').update(bytes).digest('hex') !== f.sha256) return undefined;
        files.push({ path, doc, bytes, uri: doc?.uri || pathToFileURL(path).href, offsets: new Set([0, bytes.length]) });
    }
    function span(item, declaration) {
        if (!integer(item?.file) || item.file >= files.length || !integer(item.start) || !integer(item.end) || item.start >= item.end || item.end > files[item.file].bytes.length) throw new Error('Invalid navigation range');
        const offsets = files[item.file].offsets; offsets.add(item.start); offsets.add(item.end);
        if (declaration) {
            if (!integer(item.name_start) || !integer(item.name_end) || item.name_start < item.start || item.name_start >= item.name_end || item.name_end > item.end || !string(item.name) || !string(item.signature)) throw new Error('Invalid navigation declaration');
            offsets.add(item.name_start); offsets.add(item.name_end);
            const text = new TextDecoder('utf-8', { fatal: true }).decode(files[item.file].bytes.subarray(item.name_start, item.name_end));
            if (item.name.split('::').at(-1) !== text) throw new Error('Declaration name/source mismatch');
            const signature = Buffer.from(item.signature);
            if (signature.length > item.end - item.start || !files[item.file].bytes.subarray(item.start, item.start + signature.length).equals(signature)) throw new Error('Signature/source mismatch');
        }
    }
    for (const d of meta.declarations) span(d, true);
    for (const r of meta.references) {
        span(r, false);
        if (r.definition === null) { if (!string(r.signature)) throw new Error('Invalid builtin signature'); }
        else if (!integer(r.definition) || r.definition >= meta.declarations.length || own(r, 'signature')) throw new Error('Invalid checked target index');
    }
    for (const f of files) f.positions = positionsFor(f.bytes, f.offsets);
    const range = (file, start, end) => ({ start: file.positions.get(start), end: file.positions.get(end) });
    const declarations = meta.declarations.map(d => ({ ...d, uri: files[d.file].uri, range: range(files[d.file], d.start, d.end), selectionRange: range(files[d.file], d.name_start, d.name_end) }));
    const output = new Map([...members.values()].map(doc => [doc, { declarations: [], references: [], locals: [], localReferences: [] }]));
    for (const d of declarations) { const doc = files[d.file].doc; if (doc) output.get(doc).declarations.push(d); }
    for (const r of meta.references) {
        const file = files[r.file]; if (!file.doc) continue;
        output.get(file.doc).references.push({ ...r, declaration: r.definition === null ? undefined : declarations[r.definition], range: range(file, r.start, r.end) });
    }
    return output;
}
