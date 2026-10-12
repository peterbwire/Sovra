// Private local browser host for named-file operations implemented in Sovra.
import { randomBytes } from 'node:crypto';
import { lstatSync, readdirSync, realpathSync } from 'node:fs';
import { createServer } from 'node:http';
import { basename, dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createSovraProgram } from '../../01-task-manager/gui/bridge.mjs';
import { decodeUiView, renderUiView } from '../../01-task-manager/gui/ui-view.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const options = {
    root: '', port: '8788', engine: 'javascript',
    compiler: resolve(here, '../../../target/debug', process.platform === 'win32' ? 'svr.exe' : 'svr'),
    source: resolve(here, '../main.svr'),
};
for (let index = 2; index < process.argv.length; index += 2) {
    const key = process.argv[index]?.slice(2);
    if (!Object.hasOwn(options, key) || !process.argv[index + 1]) {
        throw new Error('Usage: server.mjs --root DIRECTORY [--compiler PATH] [--source PATH] [--engine javascript|interpreter] [--port PORT]');
    }
    options[key] = process.argv[index + 1];
}
if (!options.root) throw new Error('--root is required');
options.root = realpathSync(resolve(options.root));
if (!lstatSync(options.root).isDirectory()) throw new Error('Workspace must be a directory');
options.compiler = resolve(options.compiler);
options.source = resolve(options.source);
const port = Number(options.port);
if (!Number.isInteger(port) || port < 0 || port > 65535) throw new Error('Invalid port');
const program = createSovraProgram(options);
const snapshots = new Map();
const escapeHtml = value => String(value).replace(/[&<>"']/g, character => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
})[character]);
const validName = name => typeof name === 'string' && name.length > 0 &&
    name !== '.' && name !== '..' && basename(name) === name &&
    !/[\\/\u0000-\u001f\u007f-\u009f]/u.test(name);

function files() {
    return readdirSync(options.root, { withFileTypes: true })
        .filter(entry => entry.isFile() && validName(entry.name))
        .map(entry => entry.name).sort((left, right) => left < right ? -1 : left > right ? 1 : 0);
}

function regularPath(name, mustExist) {
    if (!validName(name)) return null;
    const path = join(options.root, name);
    try {
        if (!lstatSync(path).isFile()) return null;
    } catch (error) {
        if (error.code !== 'ENOENT' || mustExist) return null;
    }
    return path;
}

function page(notice = '', preview = null, requestedPage = 0) {
    const allNames = files();
    const pageCount = Math.max(1, Math.ceil(allNames.length / 80));
    const pageIndex = Math.max(0, Math.min(requestedPage, pageCount - 1));
    const names = allNames.slice(pageIndex * 80, (pageIndex + 1) * 80);
    const result = program.run(['ui-view', String(pageIndex + 1), String(pageCount), ...names]);
    if (result.status !== 0) throw new Error(result.stdout || result.stderr);
    const nodes = decodeUiView(result.stdout.replaceAll('\r\n', '\n'));
    const token = randomBytes(32).toString('hex');
    snapshots.set(token, { names, nodes, pageIndex });
    if (snapshots.size > 32) snapshots.delete(snapshots.keys().next().value);
    const view = renderUiView(nodes, token, '/action', notice);
    return `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Sovra File Manager</title><style>body{font:1rem system-ui;max-width:56rem;margin:2rem auto;padding:0 1rem}form{display:inline;margin-left:.5rem}pre{white-space:pre-wrap;overflow-wrap:anywhere;border:1px solid #ccc;padding:1rem}input{max-width:14rem}</style></head><body><main>${view}${preview !== null ? `<h2>Preview</h2><pre>${escapeHtml(preview)}</pre>` : ''}</main></body></html>`;
}

function respond(response, status, body) {
    response.writeHead(status, {
        'Content-Type': 'text/html; charset=utf-8', 'Cache-Control': 'no-store',
        'Content-Security-Policy': "default-src 'none'; style-src 'unsafe-inline'; form-action 'self'; base-uri 'none'; frame-ancestors 'none'",
        'X-Content-Type-Options': 'nosniff',
    }).end(body);
}

const server = createServer(async (request, response) => {
    try {
        const host = request.headers.host;
        if (![ `127.0.0.1:${server.address().port}`, `localhost:${server.address().port}` ].includes(host)) {
            return respond(response, 403, 'Forbidden');
        }
        if (request.method === 'GET' && request.url === '/') return respond(response, 200, page());
        if (request.method !== 'POST' || request.url !== '/action') return respond(response, 404, 'Not found');
        if (!request.headers['content-type']?.startsWith('application/x-www-form-urlencoded')) {
            return respond(response, 415, 'Unsupported media type');
        }
        if (request.headers.origin && request.headers.origin !== `http://${host}`) {
            return respond(response, 403, 'Forbidden');
        }
        const chunks = [];
        let length = 0;
        for await (const chunk of request) {
            length += chunk.length;
            if (length > 65536) return respond(response, 413, 'Request too large');
            chunks.push(chunk);
        }
        let body;
        try { body = new TextDecoder('utf-8', { fatal: true }).decode(Buffer.concat(chunks, length)); }
        catch { return respond(response, 400, 'Invalid UTF-8'); }
        const form = new URLSearchParams(body);
        if (form.getAll('token').length !== 1) return respond(response, 403, 'Forbidden');
        const snapshot = snapshots.get(form.get('token'));
        if (!snapshot) return respond(response, 403, 'Forbidden');
        if (form.getAll('action').length !== 1 || form.getAll('command').length !== 0 ||
            form.getAll('source').length !== 0 || form.getAll('destination').length !== 0) {
            return respond(response, 400, 'Bad request');
        }
        const action = form.get('action');
        const node = snapshot.nodes.find(item => ['button', 'input'].includes(item.kind) && item.action === action);
        if ((action === 'page-next' || action === 'page-previous') && node?.kind === 'button' &&
            form.getAll('value').length === 0) {
            return respond(response, 200, page('', null, snapshot.pageIndex + (action === 'page-next' ? 1 : -1)));
        }
        if (action === 'create' && node?.kind === 'input' && form.getAll('value').length === 1) {
            const name = form.get('value');
            if (!validName(name) || files().includes(name)) return respond(response, 400, 'Invalid destination');
            const destinationPath = regularPath(name, false);
            if (!destinationPath) return respond(response, 400, 'Invalid destination');
            const result = program.run(['create', destinationPath]);
            return respond(response, result.status === 0 ? 200 : 400,
                page(result.status === 0 ? `Created ${name}` : result.stdout.trimEnd(), null, snapshot.pageIndex));
        }
        const match = /^(preview|copy)-([0-9]+)$/.exec(action);
        if (!node || !match || (match[1] === 'preview' && node.kind !== 'button') ||
            (match[1] === 'copy' && node.kind !== 'input')) return respond(response, 400, 'Unknown action');
        const source = snapshot.names[Number(match[2])];
        if (!source) return respond(response, 400, 'Unknown file');
        if (!files().includes(source)) return respond(response, 400, 'File unavailable');
        const sourcePath = regularPath(source, true);
        if (!sourcePath) return respond(response, 400, 'File unavailable');
        if (match[1] === 'preview' && form.getAll('value').length === 0) {
            const result = program.run(['preview', sourcePath]);
            const content = result.stdout.endsWith('\n') ? result.stdout.slice(0, -1) : result.stdout;
            return respond(response, result.status === 0 ? 200 : 400,
                page(result.status === 0 ? `Previewing ${source}` : content,
                    result.status === 0 ? content : null, snapshot.pageIndex));
        }
        if (match[1] === 'copy' && form.getAll('value').length === 1) {
            const destination = form.get('value');
            if (!validName(destination) || destination === source) return respond(response, 400, 'Invalid destination');
            const destinationPath = regularPath(destination, false);
            if (!destinationPath) return respond(response, 400, 'Invalid destination');
            const result = program.run(['copy', sourcePath, destinationPath]);
            return respond(response, result.status === 0 ? 200 : 400,
                page(result.stdout.trimEnd(), null, snapshot.pageIndex));
        }
        return respond(response, 400, 'Bad request');
    } catch (error) {
        console.error(error);
        return respond(response, 500, 'Unable to load workspace');
    }
});
server.listen(port, '127.0.0.1', () => {
    console.log(`Sovra File Manager (${options.engine}): http://127.0.0.1:${server.address().port}/`);
});
