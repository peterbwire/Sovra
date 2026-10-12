// Private, bounded view protocol for the first Sovra-owned UI component.
const escapeHtml = value => value.replace(/[&<>"']/g, character => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
})[character]);
const keyPattern = /^[a-z][a-z0-9-]{0,63}$/;

export function decodeUiView(output) {
    if (Buffer.byteLength(output, 'utf8') > 65536) throw new Error('UI view exceeds 65536 bytes');
    if (!output.endsWith('\n')) throw new Error('Incomplete UI view');
    const lines = output.slice(0, -1).split('\n');
    if (lines.shift() !== 'SVR-UI-1') throw new Error('Unsupported UI view version');
    if (lines.length > 256) throw new Error('UI view has too many nodes');
    const nodes = lines.map(line => {
        const separator = line.indexOf('|');
        if (separator < 0) throw new Error('Invalid UI node');
        const kind = line.slice(0, separator);
        const value = line.slice(separator + 1);
        if (kind === 'heading' || kind === 'text') {
            if (!value || /[\u0000-\u001f\u007f-\u009f]/u.test(value)) throw new Error('Invalid UI text');
            return { kind, text: value };
        }
        if (kind === 'button') {
            const split = value.indexOf('|');
            if (split < 0) throw new Error('Invalid UI button');
            const action = value.slice(0, split);
            const label = value.slice(split + 1);
            if (!keyPattern.test(action) || !label || /[\u0000-\u001f\u007f-\u009f]/u.test(label)) {
                throw new Error('Invalid UI button');
            }
            return { kind, action, label };
        }
        if (kind === 'input') {
            const split = value.indexOf('|');
            const last = value.lastIndexOf('|');
            if (split < 0 || split === last) throw new Error('Invalid UI input');
            const action = value.slice(0, split);
            const label = value.slice(split + 1, last);
            const submit = value.slice(last + 1);
            if (!keyPattern.test(action) || !label || !submit ||
                /[\u0000-\u001f\u007f-\u009f]/u.test(label + submit)) {
                throw new Error('Invalid UI input');
            }
            return { kind, action, label, submit };
        }
        throw new Error('Unsupported UI node');
    });
    if (nodes.length === 0 || nodes[0].kind !== 'heading') throw new Error('Missing UI heading');
    return nodes;
}

export function renderUiView(nodes, token, actionPath = '/action', notice = '') {
    if (!/^[0-9a-f]{64}$/.test(token)) throw new Error('Invalid UI session token');
    if (!['/action', '/ui-action'].includes(actionPath)) throw new Error('Invalid UI action path');
    if (typeof notice !== 'string') throw new Error('Invalid UI notice');
    const content = nodes.map(node => {
        if (node.kind === 'heading') return `<h1>${escapeHtml(node.text)}</h1>`;
        if (node.kind === 'text') return `<p>${escapeHtml(node.text)}</p>`;
        if (node.kind === 'button') return `<form method="post" action="${actionPath}"><input type="hidden" name="token" value="${token}"><input type="hidden" name="action" value="${node.action}"><button>${escapeHtml(node.label)}</button></form>`;
        if (node.kind === 'input') return `<form method="post" action="${actionPath}"><input type="hidden" name="token" value="${token}"><input type="hidden" name="action" value="${node.action}"><input name="value" aria-label="${escapeHtml(node.label)}" required><button>${escapeHtml(node.submit)}</button></form>`;
        throw new Error('Unsupported UI node');
    }).join('');
    return notice ? `<p role="status">${escapeHtml(notice)}</p>${content}` : content;
}
