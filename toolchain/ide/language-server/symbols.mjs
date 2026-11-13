// Strict, bounded compiler evidence decoding. Invalid metadata never becomes hover data.
const MAX_BYTES = 512 * 1024;
const MAX_RECORDS = 8192;
const identifier = /^[A-Za-z_][A-Za-z_0-9]*(?:::[A-Za-z_][A-Za-z_0-9]*)*$/;
const own = (item, key) => Object.hasOwn(item, key);
const object = item => item !== null && typeof item === 'object' && !Array.isArray(item);
function string(value) {
    if (typeof value !== 'string' || !value.isWellFormed() || Buffer.byteLength(value) > 4096 || value.length === 0) throw new Error('Invalid symbol string');
}
function inferredType(value) {
    string(value);
    let depth = 0;
    // Inferred nesting may grow across shallow bindings beyond syntax depth.
    while (value.startsWith('Array<')) { value = value.slice(6); if (++depth > Math.floor(4096 / 7)) throw new Error('Invalid inferred type depth'); }
    if (depth) {
        if (!value.endsWith('>'.repeat(depth))) throw new Error('Invalid inferred type');
        value = value.slice(0, -depth);
    }
    if (!identifier.test(value)) throw new Error('Invalid inferred type');
}

export function decodeEvidence(symbols, bytes) {
    if (symbols === undefined) return undefined;
    if (!object(symbols) || typeof symbols.truncated !== 'boolean' || Buffer.byteLength(JSON.stringify(symbols)) > MAX_BYTES) throw new Error('Invalid symbol envelope');
    const declarations = symbols.declarations;
    const references = symbols.references;
    const locals = symbols.locals === undefined ? [] : symbols.locals;
    const localReferences = symbols.local_references === undefined ? [] : symbols.local_references;
    const tables = [declarations, references, locals, localReferences];
    if (tables.some(table => !Array.isArray(table) || table.length > MAX_RECORDS)) throw new Error('Invalid symbol table');
    if (own(symbols, 'unavailable') && typeof symbols.unavailable !== 'boolean') throw new Error('Invalid symbol availability');
    if (symbols.unavailable) {
        if (!symbols.truncated || tables.some(table => table.length !== 0)) throw new Error('Unavailable symbols contain evidence');
        return undefined;
    }
    const offsets = new Set([0]);
    const span = (item, declaration) => {
        if (!object(item) || !Number.isSafeInteger(item.start) || !Number.isSafeInteger(item.end) || item.start < 0 || item.start >= item.end || item.end > bytes.length) throw new Error('Invalid symbol range');
        offsets.add(item.start); offsets.add(item.end);
        if (declaration) {
            if (!Number.isSafeInteger(item.name_start) || !Number.isSafeInteger(item.name_end) || item.name_start < item.start || item.name_start >= item.name_end || item.name_end > item.end) throw new Error('Invalid symbol name range');
            offsets.add(item.name_start); offsets.add(item.name_end);
            string(item.name); string(item.signature);
            if (!identifier.test(item.name)) throw new Error('Invalid symbol name');
        }
    };
    for (const item of declarations) span(item, true);
    for (const item of locals) {
        span(item, true);
        if (!['local', 'parameter'].includes(item.kind) || item.name.includes('::')) throw new Error('Invalid local declaration');
        if (own(item, 'inferred_type')) {
            if (item.kind !== 'local') throw new Error('Inferred parameter type');
            if (item.inferred_type !== null) inferredType(item.inferred_type);
            const suffix = item.inferred_type === null ? ' (inferred type unavailable)' : `: ${item.inferred_type} (inferred)`;
            if (![ `let ${item.name}${suffix}`, `let mut ${item.name}${suffix}` ].includes(item.signature)) throw new Error('Inconsistent inferred signature');
        }
    }
    for (const item of references) {
        span(item, false);
        if (item.definition === null) string(item.signature);
        else if (!Number.isSafeInteger(item.definition) || item.definition < 0 || item.definition >= declarations.length) throw new Error('Invalid function definition index');
        if (own(item, 'signature')) string(item.signature);
    }
    for (const item of localReferences) {
        span(item, false);
        if (!Number.isSafeInteger(item.definition) || item.definition < 0 || item.definition >= locals.length) throw new Error('Invalid local definition index');
    }
    const positions = positionsFor(bytes, offsets);
    const range = (start, end) => ({ start: positions.get(start), end: positions.get(end) });
    return {
        declarations: declarations.map(item => ({ ...item, range: range(item.start, item.end), selectionRange: range(item.name_start, item.name_end) })),
        references: references.map(item => ({ ...item, range: range(item.start, item.end) })),
        locals: locals.map(item => ({ ...item, range: range(item.start, item.end), selectionRange: range(item.name_start, item.name_end) })),
        localReferences: localReferences.map(item => ({ ...item, range: range(item.start, item.end) })),
    };
}

// Shared UTF-8 boundary/UTF-16 coordinate conversion, without semantic inference.
export function positionsFor(bytes, offsets) {
    for (const offset of offsets) if (!Number.isInteger(offset) || offset < 0 || offset > bytes.length) throw new Error('Invalid source offset');
    const positions = new Map();
    let previous = 0, line = 0, character = 0;
    for (const offset of [...offsets].sort((a, b) => a - b)) {
        const fragment = new TextDecoder('utf-8', { fatal: true }).decode(bytes.subarray(previous, offset));
        for (const value of fragment) { if (value === '\n') { line++; character = 0; } else character += value.length; }
        positions.set(offset, { line, character }); previous = offset;
    }
    return positions;
}
