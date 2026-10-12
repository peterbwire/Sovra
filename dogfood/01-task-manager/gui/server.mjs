// Local browser front end for the Sovra Task Manager CLI. Node 22+; no packages.
import { randomBytes } from 'node:crypto';
import { createServer } from 'node:http';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createSovraProgram } from './bridge.mjs';
import { decodeUiView, renderUiView } from './ui-view.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const defaults = {
    compiler: resolve(here, '../../../target/debug', process.platform === 'win32' ? 'svr.exe' : 'svr'),
    source: resolve(here, '../main.svr'),
    state: resolve(process.cwd(), 'tasks.txt'),
    port: '8787',
    engine: 'javascript',
};
const options = { ...defaults };
for (let i = 2; i < process.argv.length; i += 2) {
    const key = process.argv[i]?.slice(2);
    if (!['compiler', 'source', 'state', 'port', 'engine'].includes(key) || !process.argv[i + 1]) {
        throw new Error('Usage: node server.mjs [--compiler PATH] [--source PATH] [--state PATH] [--port 0..65535] [--engine javascript|interpreter]');
    }
    options[key] = process.argv[i + 1];
}
for (const key of ['compiler', 'source', 'state']) options[key] = resolve(options[key]);
const port = Number(options.port);
if (!Number.isInteger(port) || port < 0 || port > 65535) throw new Error('Invalid port');
const token = randomBytes(32).toString('hex');
const program = createSovraProgram(options);
const escapeHtml = value => String(value).replace(/[&<>"']/g, character => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
})[character]);

function sovra(command, title, value) {
    const args = [command, options.state];
    if (title !== undefined) args.push(title);
    if (value !== undefined) args.push(value);
    const result = program.run(args);
    return {
        ok: result.status === 0,
        stdout: result.stdout,
        message: (result.stdout || result.stderr || 'Task Manager failed').trim(),
    };
}

function loadTasks() {
    const checked = sovra('gui-list');
    if (!checked.ok) throw new Error(checked.message);
    const output = checked.stdout.replaceAll('\r\n', '\n');
    if (!output.endsWith('\n')) throw new Error('Invalid GUI view response');
    const [header, ...rows] = output.slice(0, -1).split('\n');
    if (header !== 'SVR-GUI-1') throw new Error('Unsupported GUI view response');
    return rows.map(row => {
        const match = row.match(/^([1-9][0-9]*)\|([01])\|([\s\S]*)$/);
        if (!match || !match[3]) throw new Error('Invalid GUI view response');
        return { id: match[1], done: match[2] === '1', title: match[3] };
    });
}

function uiPage(notice = '') {
    const result = sovra('ui-view');
    if (!result.ok) throw new Error(result.message);
    const nodes = decodeUiView(result.stdout.replaceAll('\r\n', '\n'));
    return `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Sovra UI</title></head><body><main>${renderUiView(nodes, token, '/ui-action', notice)}</main></body></html>`;
}

const hasControls = text => /[\u0000-\u001f\u007f-\u009f]/u.test(text);
const displayTitle = text => text.replace(/[\u0000-\u001f\u007f-\u009f]/gu,
    character => `\\u{${character.codePointAt(0).toString(16).toUpperCase().padStart(4, '0')}}`);

function page(tasks, notice = '', isError = false) {
    const rows = tasks.map(task => `<li class="task ${task.done ? 'done' : ''}">
      <span class="task-title">${escapeHtml(displayTitle(task.title))}</span>
      <span class="task-status">${task.done ? 'Done' : 'Open'} · #${escapeHtml(task.id)}</span>
      <span class="actions">
        ${hasControls(task.title) ? '<span class="task-status">Edit in CLI</span>' : `
        ${task.done ? '' : `<form method="post" action="/action"><input type="hidden" name="token" value="${token}"><input type="hidden" name="command" value="complete"><input type="hidden" name="title" value="${escapeHtml(task.title)}"><button>Complete</button></form>`}
        <form method="post" action="/action"><input type="hidden" name="token" value="${token}"><input type="hidden" name="command" value="delete"><input type="hidden" name="title" value="${escapeHtml(task.title)}"><button class="quiet">Delete</button></form>`}
      </span></li>`).join('');
    return `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Sovra Task Manager</title><style>
      :root{font-family:system-ui,sans-serif;color:#183047;background:#f2f5f6}*{box-sizing:border-box}
      body{margin:0}.wrap{max-width:760px;margin:4rem auto;padding:0 1rem}h1{margin-bottom:.2rem}
      .sub{color:#526474;margin-top:0}.card{background:white;border:1px solid #dbe3e8;border-radius:14px;padding:1.3rem;margin-top:1.2rem;box-shadow:0 3px 16px #142e4010}
      .add{display:flex;gap:.5rem}.add input{flex:1;min-width:0;padding:.7rem;border:1px solid #aab9c2;border-radius:7px;font:inherit}
      button{font:inherit;background:#075b72;color:white;border:0;border-radius:7px;padding:.65rem .85rem;cursor:pointer}
      button:hover{background:#06495b}button.quiet{background:#e9eef1;color:#183047}button.quiet:hover{background:#dbe4e9}
      ul{padding:0;list-style:none}.task{display:flex;align-items:center;gap:.6rem;padding:.9rem 0;border-top:1px solid #e5eaed;flex-wrap:wrap}
      .task-title{flex:1;min-width:12rem;overflow-wrap:anywhere;white-space:pre-wrap}.done .task-title{text-decoration:line-through;color:#61727c}
      .task-status{color:#526474;font-size:.85rem}.actions{display:flex;gap:.4rem}.actions form{margin:0}
      .notice{padding:.7rem;border-radius:7px;background:#e5f4ef}.notice.error{background:#ffe9e8;color:#8a2825}
      </style></head><body><main class="wrap"><h1>Sovra Task Manager</h1><p class="sub">${tasks.length} task${tasks.length === 1 ? '' : 's'} · saved locally</p>
      ${notice ? `<p role="status" class="notice${isError ? ' error' : ''}">${escapeHtml(notice)}</p>` : ''}
      <section class="card"><h2>Add a task</h2><form class="add" method="post" action="/action"><input type="hidden" name="token" value="${token}"><input type="hidden" name="command" value="add"><input name="title" aria-label="Task title" required autofocus><button>Add</button></form></section>
      <section class="card"><h2>Tasks</h2>${rows ? `<ul>${rows}</ul>` : '<p>No tasks yet.</p>'}</section></main></body></html>`;
}

function respond(response, status, body) {
    response.writeHead(status, {
        'Content-Type': 'text/html; charset=utf-8',
        'Cache-Control': 'no-store',
        'Content-Security-Policy': "default-src 'none'; style-src 'unsafe-inline'; form-action 'self'; base-uri 'none'; frame-ancestors 'none'",
        'X-Content-Type-Options': 'nosniff',
    });
    response.end(body);
}
const server = createServer(async (request, response) => {
    try {
        const address = server.address();
        const allowedHosts = [`127.0.0.1:${address.port}`, `localhost:${address.port}`];
        if (!allowedHosts.includes(request.headers.host)) return respond(response, 403, 'Forbidden');
        if (request.method === 'GET' && request.url === '/') {
            return respond(response, 200, page(loadTasks()));
        }
        if (request.method === 'GET' && request.url === '/ui') {
            return respond(response, 200, uiPage());
        }
        if (request.method !== 'POST' || !['/action', '/ui-action'].includes(request.url)) {
            return respond(response, 404, 'Not found');
        }
        if (!request.headers['content-type']?.startsWith('application/x-www-form-urlencoded')) {
            return respond(response, 415, 'Unsupported media type');
        }
        const origin = request.headers.origin;
        if (origin && origin !== `http://${request.headers.host}`) return respond(response, 403, 'Forbidden');
        const chunks = [];
        let bodyBytes = 0;
        for await (const chunk of request) {
            bodyBytes += chunk.length;
            if (bodyBytes > 65536) return respond(response, 413, 'Request too large');
            chunks.push(chunk);
        }
        let body;
        try {
            body = new TextDecoder('utf-8', { fatal: true }).decode(Buffer.concat(chunks, bodyBytes));
        } catch {
            return respond(response, 400, 'Invalid UTF-8');
        }
        const form = new URLSearchParams(body);
        if (form.getAll('token').length !== 1 || form.get('token') !== token) return respond(response, 403, 'Forbidden');
        if (request.url === '/ui-action') {
            if (form.getAll('action').length !== 1 || form.getAll('command').length !== 0 ||
                form.getAll('title').length !== 0) return respond(response, 400, 'Bad request');
            const view = sovra('ui-view');
            if (!view.ok) throw new Error(view.message);
            const nodes = decodeUiView(view.stdout.replaceAll('\r\n', '\n'));
            const action = form.get('action');
            const node = nodes.find(item => ['button', 'input'].includes(item.kind) && item.action === action);
            if (!node) {
                return respond(response, 400, 'Unknown UI action');
            }
            if ((node.kind === 'input' && form.getAll('value').length !== 1) ||
                (node.kind === 'button' && form.getAll('value').length !== 0)) {
                return respond(response, 400, 'Bad request');
            }
            const result = sovra('ui-event', action, node.kind === 'input' ? form.get('value') : undefined);
            return respond(response, result.ok ? 200 : 400, uiPage(result.message));
        }
        if (form.getAll('command').length !== 1 || form.getAll('title').length !== 1) return respond(response, 400, 'Bad request');
        const command = form.get('command');
        if (!['add', 'complete', 'delete'].includes(command)) return respond(response, 400, 'Bad request');
        const result = sovra(command, form.get('title'));
        return respond(response, result.ok ? 200 : 400, page(loadTasks(), result.message, !result.ok));
    } catch (error) {
        console.error(error);
        return respond(response, 500, page([], 'Unable to load the task file. See server output.', true));
    }
});
server.listen(port, '127.0.0.1', () => {
    console.log(`Sovra Task Manager GUI (${options.engine}): http://127.0.0.1:${server.address().port}/`);
});
