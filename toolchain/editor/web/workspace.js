// Shared browser/Node workspace admission. Names are identities, never host paths.
export const MAX_FILES = 64;
export const MAX_BYTES = 512 * 1024;
const encoder = new TextEncoder();
export function validateWorkspace(value) {
    if (!value || value.schema_version !== 1 || typeof value.id !== 'string' || !/^[a-zA-Z0-9-]{1,80}$/.test(value.id)
        || !Array.isArray(value.files) || !value.files.length || value.files.length > MAX_FILES) throw new Error('Invalid workspace: expected version 1 and 1–64 files');
    const names = new Set();
    let bytes = 0;
    const files = value.files.map(file => {
        if (!file || typeof file.name !== 'string' || file.name.length > 160
            || !/^[a-zA-Z0-9_-]+\.svr$/.test(file.name) || names.has(file.name.toLowerCase())
            || typeof file.source !== 'string' || /[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/u.test(file.source)) {
            throw new Error('Invalid workspace file: use unique simple .svr names and valid Unicode');
        }
        names.add(file.name.toLowerCase());
        bytes += encoder.encode(file.source).length;
        if (bytes > MAX_BYTES) throw new Error('Workspace source exceeds 512 KiB');
        return { name: file.name, source: file.source };
    });
    if (!files.some(file => file.name === value.active)) throw new Error('Workspace active file is missing');
    return { schema_version: 1, id: value.id, active: value.active, files };
}
