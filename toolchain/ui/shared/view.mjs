// Shared private Sovra UI boundary; applications own views and event behavior.
import { TextDecoder } from 'node:util';

const bytesLimit = 65536;
const controls = /[\u0000-\u001f\u007f-\u009f]/u;
const keyPattern = /^[a-z][a-z0-9-]{0,63}$/;
const utf8 = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true });
const escapeHtml = value => value.replace(/[&<>"']/g, character => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
})[character]);

function scalarString(value, limit = bytesLimit) {
    if (typeof value !== 'string') throw new TypeError('UI text must be a string');
    if (value.length > limit) throw new Error('UI text exceeds its byte limit');
    for (const character of value) {
        const point = character.codePointAt(0);
        if (point >= 0xd800 && point <= 0xdfff) throw new Error('Invalid UI Unicode');
    }
    if (Buffer.byteLength(value, 'utf8') > limit) throw new Error('UI text exceeds its byte limit');
    return value;
}

const validKey = value => typeof value === 'string' && keyPattern.test(value) && !controls.test(value);
const validText = value => scalarString(value).length !== 0 && !controls.test(value);

// v2 must be selected explicitly; existing v1 consumers never probe/fall back.
export function decodeUiView(output, { version = 1 } = {}) {
    if (version !== 1 && version !== 2) throw new Error('Unsupported UI view version');
    if (output instanceof Uint8Array) {
        if (output.byteLength > bytesLimit) throw new Error('UI view exceeds 65536 bytes');
        try { output = utf8.decode(output); }
        catch { throw new Error('Invalid UI UTF-8'); }
    }
    scalarString(output);
    if (!output.endsWith('\n')) throw new Error('Incomplete UI view');
    const lines = output.slice(0, -1).split('\n');
    if (lines.shift() !== `SVR-UI-${version}`) throw new Error('Unsupported UI view version');
    if (lines.length > (version === 2 ? 768 : 256)) throw new Error('UI view has too many nodes');
    const actions = new Set();
    const groups = new Set();
    const stack = [];
    let leaves = 0;
    const nodes = lines.map(line => {
        const separator = line.indexOf('|');
        if (separator < 0) throw new Error('Invalid UI node');
        const kind = line.slice(0, separator);
        const value = line.slice(separator + 1);
        if (version === 2 && kind === 'group') {
            const parts = value.split('|');
            if (parts.length !== 3 || !validKey(parts[0]) ||
                !['vertical', 'horizontal'].includes(parts[1]) || !['card', 'plain'].includes(parts[2])) {
                throw new Error('Invalid UI group');
            }
            if (groups.size >= 256 || stack.length >= 8 || groups.has(parts[0])) throw new Error('UI group identity/depth limit exceeded');
            groups.add(parts[0]); stack.push(parts[0]);
            return { kind, key: parts[0], orientation: parts[1], presentation: parts[2] };
        }
        if (version === 2 && kind === 'end') {
            if (stack.length === 0 || stack.pop() !== value) throw new Error('Unbalanced UI group');
            return { kind, key: value };
        }
        if (++leaves > 256) throw new Error('UI view has too many leaf nodes');
        if (kind === 'heading' || kind === 'text') {
            if (!validText(value)) throw new Error('Invalid UI text');
            return { kind, text: value };
        }
        if (kind === 'button' || kind === 'input') {
            const first = value.indexOf('|');
            if (first < 0) throw new Error('Invalid UI action');
            const action = value.slice(0, first);
            if (!validKey(action) || actions.has(action)) throw new Error('Invalid or duplicate UI action key');
            actions.add(action);
            if (kind === 'button') {
                const label = value.slice(first + 1);
                if (!validText(label)) throw new Error('Invalid UI button');
                return { kind, action, label };
            }
            const last = value.lastIndexOf('|');
            if (last === first) throw new Error('Invalid UI input');
            const label = value.slice(first + 1, last);
            const submit = value.slice(last + 1);
            if (!validText(label) || !validText(submit)) throw new Error('Invalid UI input');
            return { kind, action, label, submit };
        }
        throw new Error('Unsupported UI node');
    });
    if (nodes.length === 0 || nodes[0].kind !== 'heading') throw new Error('Missing UI heading');
    if (stack.length) throw new Error('Unclosed UI group');
    return nodes;
}

// Revalidate caller-built/mutated models before interpolating HTML attributes.
function checkedNodes(nodes) {
    if (!Array.isArray(nodes) || nodes.length > 768) throw new Error('Invalid UI node collection');
    const version = nodes.some(node => node?.kind === 'group' || node?.kind === 'end') ? 2 : 1;
    const lines = [`SVR-UI-${version}`];
    let bytes = Buffer.byteLength(lines[0]) + 1;
    for (const node of nodes) {
        if (!node || typeof node !== 'object') throw new Error('Invalid UI node');
        if (['button', 'input'].includes(node.kind) && !validKey(node.action)) throw new Error('Invalid UI action key');
        if (['group', 'end'].includes(node.kind) && !validKey(node.key)) throw new Error('Invalid UI group key');
        let fields;
        switch (node.kind) {
            case 'heading': case 'text': fields = [node.kind, node.text]; break;
            case 'button': fields = [node.kind, node.action, node.label]; break;
            case 'input':
                if (typeof node.submit !== 'string' || node.submit.includes('|')) throw new Error('Invalid UI submit label');
                fields = [node.kind, node.action, node.label, node.submit]; break;
            case 'group': fields = [node.kind, node.key, node.orientation, node.presentation]; break;
            case 'end': fields = [node.kind, node.key]; break;
            default: throw new Error('Unsupported UI node');
        }
        for (const field of fields) {
            scalarString(field);
            if (controls.test(field)) throw new Error('Invalid UI node text');
        }
        bytes += fields.reduce((size, field) => size + Buffer.byteLength(field), fields.length);
        if (bytes > bytesLimit) throw new Error('UI view exceeds 65536 bytes');
        lines.push(fields.join('|'));
    }
    return decodeUiView(lines.join('\n') + '\n', { version });
}

const groupStyles = `<style data-sovra-ui>
.sovra-ui-group{display:flex;gap:1rem;min-width:0}.sovra-ui-vertical{flex-direction:column}
.sovra-ui-horizontal{flex-flow:row wrap;align-items:start}.sovra-ui-card{border:1px solid var(--sovra-ui-border,#cbd5e1);border-radius:14px;padding:1.25rem;background:var(--sovra-ui-surface,transparent)}
.sovra-ui-group>p{margin:0;white-space:pre-wrap;overflow-wrap:anywhere}.sovra-ui-group>form{display:flex;flex-wrap:wrap;gap:.65rem;align-items:center}.sovra-ui-group input{min-width:0;flex:1;font:inherit}.sovra-ui-group button{font:inherit;min-height:44px}
@media(forced-colors:active){.sovra-ui-card{border-color:CanvasText}}
</style>`;

export function renderUiView(nodes, token, actionPath = '/action', notice = '') {
    if (!/^[0-9a-f]{64}$/.test(token) || token.length !== 64) throw new Error('Invalid UI session token');
    if (!['/action', '/ui-action'].includes(actionPath)) throw new Error('Invalid UI action path');
    scalarString(notice, 2 * bytesLimit); // Two bounded native output streams.
    const checked = checkedNodes(nodes);
    const grouped = checked.some(node => node.kind === 'group');
    const content = checked.map(node => {
        if (node.kind === 'group') return `<section data-sovra-group="${node.key}" class="sovra-ui-group sovra-ui-${node.orientation} sovra-ui-${node.presentation}">`;
        if (node.kind === 'end') return '</section>';
        if (node.kind === 'heading') return `<h1>${escapeHtml(node.text)}</h1>`;
        if (node.kind === 'text') return `<p>${escapeHtml(node.text)}</p>`;
        if (node.kind === 'button') return `<form method="post" action="${actionPath}"><input type="hidden" name="token" value="${token}"><input type="hidden" name="action" value="${node.action}"><button>${escapeHtml(node.label)}</button></form>`;
        return `<form method="post" action="${actionPath}"><input type="hidden" name="token" value="${token}"><input type="hidden" name="action" value="${node.action}"><input name="value" aria-label="${escapeHtml(node.label)}" required><button>${escapeHtml(node.submit)}</button></form>`;
    }).join('');
    return (grouped ? groupStyles : '') + (notice ? `<p role="status">${escapeHtml(notice)}</p>` : '') + content;
}
