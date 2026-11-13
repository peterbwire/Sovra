// Package graph membership comes from the compiler; this module never parses manifests.
import { realpath, stat } from 'node:fs/promises';
import { dirname, join, relative, isAbsolute, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
const scalar = s => typeof s === 'string' && !/[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/u.test(s);
const pathString = s => scalar(s) && Buffer.byteLength(s) <= 4096 && !s.includes('\0') && isAbsolute(hostPath(s));
// Rust canonical Windows paths may carry a verbatim prefix; match Node's
// canonical spelling without changing drive/UNC identity.
export function hostPath(path) {
    if (process.platform !== 'win32') return path;
    const slash = String.fromCharCode(92), prefix = slash + slash + '?' + slash;
    if (path.startsWith(prefix + 'UNC' + slash)) return slash + slash + path.slice(8);
    if (path.startsWith(prefix) && /^[A-Za-z]:/.test(path.slice(4)) && path[6] === slash) return path.slice(4);
    return path;
}
export async function canonicalFile(uri) {
    if (!scalar(uri)) return undefined;
    try { const u = new URL(uri); if (u.protocol !== 'file:' || u.search || u.hash) return undefined; return hostPath(await realpath(fileURLToPath(u))); } catch { return undefined; }
}
export async function workspaceRoots(params) {
    const folders = Array.isArray(params?.workspaceFolders) ? params.workspaceFolders : params?.rootUri ? [{ uri: params.rootUri }] : [];
    return [...new Set((await Promise.all(folders.slice(0, 16).map(f => canonicalFile(f?.uri)))).filter(Boolean))];
}
export function containsPath(root, file, paths = { relative, isAbsolute, sep }) {
    const r = paths.relative(root, file);
    return r === '' || (r !== '..' && !r.startsWith('..' + paths.sep) && !paths.isAbsolute(r));
}
export async function nearestPackage(file, roots) {
    const root = roots.filter(r => containsPath(r, file)).sort((a, b) => b.length - a.length)[0];
    if (!root) return undefined;
    let directory = dirname(file);
    for (let n = 0; n < 64 && containsPath(root, directory); n++) {
        try { if ((await stat(join(directory, 'sovra.toml'))).isFile()) return directory; } catch (e) { if (!['ENOENT', 'ENOTDIR'].includes(e.code)) throw new Error('Unable to inspect package manifest'); }
        if (directory === root) return undefined;
        directory = dirname(directory);
    }
    throw new Error('Package discovery depth limit exceeded');
}
export function decodeContext(report, root) {
    const c = report?.package_context;
    if (report?.schema_version !== 1 || report.kind !== 'project' || report.scope !== 'package_graph' || report.source_check !== false || report.target !== root || report.success !== true || !Array.isArray(report.diagnostics) || report.diagnostics.length || c?.schema_version !== 1 || (!pathString(c.root) || hostPath(c.root) !== root) || !Array.isArray(c.entries) || !c.entries.length || c.entries.length > 1024 || !c.entries.every(pathString) || new Set(c.entries.map(hostPath)).size !== c.entries.length) throw new Error('Compiler package context unavailable');
    return new Set(c.entries.map(hostPath));
}
export function overlayFrame(id, members) {
    if (members.size > 32) throw new Error('Package overlay buffer limit exceeded');
    const word = n => { const b = Buffer.alloc(4); b.writeUInt32BE(n); return b; };
    const pieces = [Buffer.from('SVROVL1\n'), word(id), word(members.size)];
    let total = 0;
    for (const [path, doc] of members) {
        if (doc.rejected) throw new Error('An open package buffer was rejected; package evidence unavailable');
        if (!pathString(path) || !scalar(doc.text)) throw new Error('Invalid package buffer');
        const p = Buffer.from(path), s = Buffer.from(doc.text);
        total += s.length;
        if (s.length > 512 * 1024 || total > 4 * 1024 * 1024) throw new Error('Package overlay source limit exceeded');
        pieces.push(word(p.length), word(s.length), p, s);
    }
    return Buffer.concat(pieces);
}
export function packageDiagnostics(report, root, id, entries, members, owner, position) {
    if (report?.schema_version !== 1 || report.kind !== 'project' || report.target !== root || typeof report.success !== 'boolean' || report.overlay_request?.schema_version !== 1 || report.overlay_request.request_id !== id || !Array.isArray(report.diagnostics) || report.diagnostics.length > 256) throw new Error('Unsupported package check report');
    const grouped = new Map([...members.values()].map(doc => [doc, []]));
    const zero = { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } };
    let errors = 0, closed;
    for (const item of report.diagnostics) {
        if (!['error', 'warning'].includes(item?.severity) || !/^E\d{4}$/.test(item.code) || !scalar(item.message) || Buffer.byteLength(item.message) > 4096) throw new Error('Invalid package diagnostic');
        if (item.severity === 'error') errors++;
        const location = item.location;
        let doc = owner, range = zero;
        if (location !== null) {
            if (!pathString(location?.file) || !entries.has(hostPath(location.file)) || !Number.isInteger(location.start) || !Number.isInteger(location.end) || location.start < 0 || location.end < location.start) throw new Error('Invalid package diagnostic location');
            doc = members.get(hostPath(location.file));
            if (!doc) { closed ||= `Package diagnostic in closed source ${location.file}: ${item.code}: ${item.message}`; continue; }
            const bytes = Buffer.from(doc.text);
            range = { start: position(bytes, location.start), end: position(bytes, location.end) };
        }
        grouped.get(doc).push({ range, severity: item.severity === 'warning' ? 2 : 1, source: 'sovra', code: item.code, message: item.message });
    }
    if (report.success === (errors > 0)) throw new Error('Inconsistent package check result');
    if (closed) grouped.get(owner).push({ range: zero, severity: 1, source: 'sovra-lsp', code: 'SVR_LSP', message: Array.from(closed).slice(0, 4096).join('') });
    return grouped;
}
