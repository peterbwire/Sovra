import { spawn } from 'node:child_process';
import { mkdtemp, writeFile, rm } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { tmpdir } from 'node:os';
import { join, resolve, dirname, isAbsolute } from 'node:path';
import { FrameReader, frame } from './transport.mjs';
import { decodeEvidence } from './symbols.mjs';
import { decodePackageNavigation } from './package-navigation.mjs';
import { canonicalFile, hostPath, workspaceRoots, nearestPackage, decodeContext, overlayFrame, packageDiagnostics } from './packages.mjs';

const configuredCompiler = process.argv[2] || process.env.SOVRA_COMPILER || 'svr';
const compiler = /[/\\]/.test(configuredCompiler) ? resolve(configuredCompiler) : configuredCompiler;
const documents = new Map();
const symbolRequests = new Map();
const taskRequests = new Map(), activeTaskRequests = new Set();
const LIMIT = 512 * 1024;
const TOTAL = 4 * 1024 * 1024;
const zero = { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } };
let initialized = false, shutdown = false, active, pumping = false, exiting = false;
let roots = Promise.resolve([]), workspaceMode = false, packageSequence = 0;
const send = value => {
    if (exiting) return;
    if (process.stdout.writableLength > 4 * 1024 * 1024) return stop(1);
    process.stdout.write(frame({ jsonrpc: '2.0', ...value }));
};
const error = (id, code, message) => send({ id, error: { code, message } });
const publish = (doc, diagnostics) => send({ method: 'textDocument/publishDiagnostics', params: { uri: doc.uri, version: doc.version, diagnostics } });
const operational = message => [{ range: zero, severity: 1, source: 'sovra-lsp', code: 'SVR_LSP', message }];
const scalar = text => !/[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/u.test(text);
function completeSymbols(doc) {
    doc.checked = true;
    for (const [id, request] of symbolRequests) {
        if (request.doc !== doc) continue;
        clearTimeout(request.timer); symbolRequests.delete(id); receive(request.message);
    }
}

function position(bytes, offset) {
    if (!Number.isInteger(offset) || offset < 0 || offset > bytes.length) throw new Error('Invalid compiler location');
    const prefix = new TextDecoder('utf-8', { fatal: true }).decode(bytes.subarray(0, offset));
    const lines = prefix.split('\n');
    return { line: lines.length - 1, character: lines.at(-1).length };
}

function offsetAt(text, point) {
    if (!Number.isInteger(point?.line) || !Number.isInteger(point?.character) || point.line < 0 || point.character < 0) return undefined;
    let start = 0;
    for (let line = 0; line < point.line; line++) { const end = text.indexOf('\n', start); if (end < 0) return undefined; start = end + 1; }
    const newline = text.indexOf('\n', start);
    const end = newline < 0 ? text.length : newline;
    if (start + point.character > end) return undefined;
    const index = start + point.character;
    if (index > start && /[\uD800-\uDBFF]/.test(text[index - 1]) && /[\uDC00-\uDFFF]/.test(text[index])) return undefined;
    return Buffer.byteLength(text.slice(0, index), 'utf8');
}
function symbolAt(doc, point) {
    if (!doc?.evidence) return undefined;
    const offset = offsetAt(doc.text, point);
    const local = doc.evidence.locals.find(item => item.name_start <= offset && offset < item.name_end);
    if (local) return { declaration: local, range: local.selectionRange };
    const localReference = doc.evidence.localReferences.find(item => item.start <= offset && offset < item.end);
    if (localReference) return { declaration: doc.evidence.locals[localReference.definition], range: localReference.range };
    const declaration = doc.evidence.declarations.find(item => item.name_start <= offset && offset < item.name_end);
    if (declaration) return { declaration, range: declaration.selectionRange };
    const reference = doc.evidence.references.find(item => item.start <= offset && offset < item.end);
    if (!reference) return undefined;
    return { declaration: reference.declaration || doc.evidence.declarations[reference.definition], signature: reference.signature, range: reference.range };
}

async function compilerReport(doc, args, cwd, input) {
    if (doc.cancelled || shutdown) throw new Error('Document changed');
    return new Promise((accept, reject) => {
        const child = spawn(compiler, args, { cwd, windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'] });
        doc.child = child;
        child.stdin?.on('error', () => {});
        child.stdin?.end(input);
        const output = [];
        let count = 0, failure;
        const timer = setTimeout(() => { failure = 'Compiler check timed out'; child.kill(); }, 10000);
        child.stdout.on('data', chunk => { count += chunk.length; if (count <= 1024 * 1024) output.push(chunk); else { failure = 'Compiler output exceeded 1 MiB'; child.kill(); } });
        child.stderr.on('data', chunk => { count += chunk.length; if (count > 1024 * 1024) { failure = 'Compiler output exceeded 1 MiB'; child.kill(); } });
        child.on('error', () => { failure = 'Unable to start Sovra compiler; check your compiler path'; });
        child.on('close', code => {
            if (doc.child === child) doc.child = undefined;
            clearTimeout(timer);
            if (failure) return reject(new Error(failure));
            if (code !== 0 && code !== 1) return reject(new Error('Compiler check did not complete'));
            try { accept(JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(Buffer.concat(output)))); }
            catch { reject(new Error('Compiler returned an invalid JSON report')); }
        });
    });
}

async function checkPackage(doc) {
    const file = await canonicalFile(doc.uri);
    if (!file) return false;
    const root = await nearestPackage(file, await roots);
    if (!root) return false;
    const context = await compilerReport(doc, ['check', '--format', 'json', '--package-context', root], root);
    const entries = decodeContext(context, root);
    if (!entries.has(file)) return false;
    const snapshot = [...documents.values()];
    const members = new Map();
    for (const member of snapshot) {
        const path = await canonicalFile(member.uri);
        if (!entries.has(path)) continue;
        if (members.has(path)) throw new Error('Duplicate open buffers target one canonical source');
        members.set(path, member);
    }
    const id = packageSequence = (packageSequence + 1) >>> 0;
    const input = overlayFrame(id, members);
    const report = await compilerReport(doc, ['check', '--format', 'json', '--overlays-stdin', root], root, input);
    const grouped = packageDiagnostics(report, root, id, entries, members, doc, position);
    const navigation = report.success ? await decodePackageNavigation(report.package_symbols, entries, members) : undefined;
    const local = report.success && entries.size === 1 && members.size === 1;
    if (!doc.cancelled && documents.get(doc.uri) === doc && !shutdown && snapshot.every(d => documents.get(d.uri) === d)) {
        for (const [member, diagnostics] of grouped) { clearTimeout(member.timer); member.ready = false; publish(member, diagnostics); member.evidence = navigation?.get(member); if (!local || member !== doc) completeSymbols(member); }
    }
    return local ? 'local' : true;
}
function refreshWorkspace() {
    for (const previous of documents.values()) {
        cancel(previous);
        const doc = { uri: previous.uri, version: previous.version, text: previous.text, size: previous.size, rejected: previous.rejected };
        documents.set(doc.uri, doc);
        if (doc.rejected) { doc.checked = true; continue; }
        publish(doc, []);
        doc.timer = setTimeout(() => { doc.ready = true; void pump(); }, 150);
    }
}

async function check(doc) {
    let directory, localOnly = false;
    try {
        if (workspaceMode) { const result = await checkPackage(doc); if (result === true) return; localOnly = result === 'local'; }
        if (doc.cancelled || documents.get(doc.uri) !== doc || shutdown) return;
        directory = await mkdtemp(join(tmpdir(), 'sovra-lsp-'));
        const source = join(directory, 'main.svr');
        await writeFile(source, doc.text, { mode: 0o600 });
        if (doc.cancelled || shutdown) return;
        const report = await compilerReport(doc, ['check', '--format', 'json', '--symbols', source], directory);
        if (report.schema_version !== 1 || report.kind !== 'source' || !Array.isArray(report.diagnostics)) throw new Error('Unsupported compiler report');
        const bytes = Buffer.from(doc.text, 'utf8');
        const diagnostics = report.diagnostics.map(item => ({
            range: item.location ? { start: position(bytes, item.location.start), end: position(bytes, item.location.end) } : zero,
            severity: item.severity === 'warning' ? 2 : 1, source: 'sovra', code: item.code, message: item.message,
        }));
        if (!doc.cancelled && documents.get(doc.uri) === doc && !shutdown) {
            const evidence = report.success ? decodeEvidence(report.symbols, bytes) : undefined;
            if (evidence || !localOnly) doc.evidence = evidence;
            if (!localOnly) publish(doc, diagnostics);
            completeSymbols(doc);
        }
    } catch (cause) {
        if (!doc.cancelled && documents.get(doc.uri) === doc && !shutdown) { if (!localOnly) publish(doc, operational(cause.message)); completeSymbols(doc); }
    } finally { if (directory) await rm(directory, { recursive: true, force: true }); }
}

async function pump() {
    if (pumping || shutdown) return;
    pumping = true;
    try {
        while (!shutdown) {
            const doc = [...documents.values()].find(value => value.ready && !value.cancelled);
            if (!doc) break;
            doc.ready = false; active = doc;
            await check(doc);
            active = undefined;
        }
    } catch { send({ method: 'window/logMessage', params: { type: 1, message: 'Language server temporary-file cleanup failed' } }); }
    finally { active = undefined; pumping = false; }
}

async function taskTarget(doc, control) {
    const file = await canonicalFile(doc.uri);
    if (!file) throw new Error('Task source must exist on disk');
    const standalone = { schema_version: 1, package: false, target: file, cwd: dirname(file), sources: [file], manifests: [] };
    const root = await nearestPackage(file, await roots);
    if (!root) return standalone;
    const report = await compilerReport(control, ['check', '--format', 'json', '--package-context', root], root);
    const entries = decodeContext(report, root);
    if (!entries.has(file)) return standalone;
    const paths = report.package_context.manifests;
    if (!Array.isArray(paths) || !paths.length || paths.length > 1024) throw new Error('Compiler package task context unavailable');
    const manifests = new Set();
    for (const path of paths) {
        if (control.cancelled) throw new Error('Task target cancelled');
        if (typeof path !== 'string' || !scalar(path) || !isAbsolute(path) || Buffer.byteLength(path) > 4096) throw new Error('Invalid package manifest context');
        const canonical = await canonicalFile(pathToFileURL(hostPath(path)).href);
        if (!canonical) throw new Error('Package manifest changed during task preparation');
        manifests.add(canonical);
    }
    return { schema_version: 1, package: true, target: root, cwd: root, sources: [...entries], manifests: [...manifests] };
}
function retireTask(id, record, code, message) {
    if (taskRequests.get(id) !== record) return;
    taskRequests.delete(id); clearTimeout(record.timer); record.cancelled = true; record.child?.kill();
    error(id, code, message);
}
function requestTaskTarget(id, params) {
    const doc = documents.get(params?.textDocument?.uri);
    if (!doc || params?.textDocument?.version !== doc.version) return error(id, -32801, 'Document changed or unavailable');
    if (activeTaskRequests.size >= 4 || taskRequests.has(id)) return error(id, -32000, 'Too many task target requests');
    const record = { doc, cancelled: false };
    record.timer = setTimeout(() => retireTask(id, record, -32800, 'Task target request timed out'), 11000);
    taskRequests.set(id, record); activeTaskRequests.add(record);
    void taskTarget(doc, record).then(result => {
        if (taskRequests.get(id) === record && !record.cancelled && documents.get(doc.uri) === doc && !shutdown) send({ id, result });
    }).catch(cause => {
        if (taskRequests.get(id) === record && !record.cancelled && !shutdown) error(id, -32000, cause.message);
    }).finally(() => {
        clearTimeout(record.timer); activeTaskRequests.delete(record);
        if (taskRequests.get(id) === record) taskRequests.delete(id);
    });
}

function cancel(doc) {
    if (!doc) return;
    doc.cancelled = true; clearTimeout(doc.timer); doc.child?.kill();
    for (const [id, request] of taskRequests) if (request.doc === doc) retireTask(id, request, -32801, 'Document changed');
    for (const [id, request] of symbolRequests) {
        if (request.doc !== doc) continue;
        clearTimeout(request.timer); symbolRequests.delete(id); error(id, -32801, 'Document changed');
    }
}
function update(uri, version, text) {
    if (typeof uri !== 'string' || !scalar(uri) || uri.length > 4096 || !/^(file|untitled):/.test(uri) || !Number.isSafeInteger(version) || typeof text !== 'string') return;
    const previous = documents.get(uri);
    if (previous && version <= previous.version) return;
    if (!previous && documents.size >= 32) {
        return publish({ uri, version }, operational('Language server allows at most 32 open documents'));
    }
    cancel(previous);
    const size = Buffer.byteLength(text, 'utf8');
    const total = [...documents.values()].reduce((sum, doc) => sum + (doc === previous ? 0 : doc.size), 0);
    const rejected = !scalar(text) || size > LIMIT || total + size > TOTAL;
    const doc = { uri, version, text: rejected ? '' : text, size: rejected ? 0 : size, rejected };
    documents.set(uri, doc);
    if (rejected) { doc.checked = true; publish(doc, operational('Document rejected: valid Unicode, 512 KiB/document and 4 MiB total required')); if (workspaceMode) refreshWorkspace(); return; }
    if (workspaceMode) { refreshWorkspace(); return; }
    doc.timer = setTimeout(() => { doc.ready = true; void pump(); }, 150);
}

function receive(message) {
    if (message === undefined) return error(null, -32700, 'Parse error');
    if (!message || message.jsonrpc !== '2.0' || typeof message.method !== 'string' || (message.id !== undefined && typeof message.id !== 'string' && !Number.isSafeInteger(message.id))) return error(null, -32600, 'Invalid request');
    const { id, method, params } = message;
    if (method === 'exit') return stop(shutdown ? 0 : 1);
    if (method === 'initialize' && !initialized && id !== undefined) {
        if (Array.isArray(params?.workspaceFolders) && params.workspaceFolders.length > 16) return error(id, -32602, 'At most 16 workspace folders supported');
        workspaceMode = (Array.isArray(params?.workspaceFolders) && params.workspaceFolders.length > 0) || typeof params?.rootUri === 'string';
        roots = workspaceRoots(params).catch(() => []);
        initialized = true;
        return send({ id, result: { capabilities: { positionEncoding: 'utf-16', textDocumentSync: { openClose: true, change: 1 }, hoverProvider: true, definitionProvider: true, documentSymbolProvider: true, completionProvider: { resolveProvider: false }, experimental: { sovraTaskTarget: true } }, serverInfo: { name: 'sovra-lsp', version: '0.3.0' } } });
    }
    if (!initialized) { if (id !== undefined) error(id, -32002, 'Server not initialized'); return; }
    if (shutdown) { if (id !== undefined) error(id, -32600, 'Server has shut down'); return; }
    if (method === 'shutdown' && id !== undefined) {
        shutdown = true; for (const doc of documents.values()) cancel(doc);
        return send({ id, result: null });
    }
    if (method === '$/cancelRequest') {
        const task = taskRequests.get(params?.id);
        if (task) { retireTask(params.id, task, -32800, 'Request cancelled'); return; }
        const request = symbolRequests.get(params?.id);
        if (request) { clearTimeout(request.timer); symbolRequests.delete(params.id); error(params.id, -32800, 'Request cancelled'); }
        return;
    }
    if (method === 'initialized') return;
    if (method === 'textDocument/didOpen') {
        const doc = params?.textDocument;
        if (doc?.languageId === 'sovra') update(doc.uri, doc.version, doc.text);
        return;
    }
    if (method === 'textDocument/didChange') {
        const doc = params?.textDocument;
        if (documents.has(doc?.uri) && params?.contentChanges?.length === 1 && params.contentChanges[0].range === undefined) update(doc.uri, doc.version, params.contentChanges[0].text);
        return;
    }
    if (method === 'textDocument/didClose') {
        const doc = documents.get(params?.textDocument?.uri);
        if (doc) { cancel(doc); documents.delete(doc.uri); publish(doc, []); if (workspaceMode) refreshWorkspace(); }
        return;
    }
    if (method === 'workspace/didChangeWatchedFiles' || method === 'textDocument/didSave') { if (workspaceMode) refreshWorkspace(); return; }
    if (method === 'sovra/taskTarget' && id !== undefined) return requestTaskTarget(id, params);
    if (method === 'textDocument/completion' && id !== undefined) {
        const words = ['fn', 'let', 'mut', 'return', 'if', 'else', 'while', 'mod', 'export', 'use', 'true', 'false', 'Int', 'Float', 'Bool', 'String', 'Unit'];
        return send({ id, result: documents.has(params?.textDocument?.uri) ? words.map(label => ({ label, kind: 14, detail: 'Sovra keyword/type (basic suggestion)' })) : [] });
    }
    if (id !== undefined && ['textDocument/hover', 'textDocument/definition', 'textDocument/documentSymbol'].includes(method)) {
        const doc = documents.get(params?.textDocument?.uri);
        if (doc && !doc.checked) {
            if (symbolRequests.size >= 64 || symbolRequests.has(id)) return error(id, -32000, 'Too many pending symbol requests');
            const timer = setTimeout(() => { symbolRequests.delete(id); error(id, -32800, 'Symbol request timed out'); }, 11000);
            symbolRequests.set(id, { doc, message, timer });
            return;
        }
        if (method === 'textDocument/documentSymbol') return send({ id, result: doc?.evidence?.declarations.map(item => ({ name: item.name, detail: item.signature, kind: 12, range: item.range, selectionRange: item.selectionRange })) || [] });
        const symbol = symbolAt(doc, params?.position);
        if (method === 'textDocument/definition') return send({ id, result: symbol?.declaration ? { uri: symbol.declaration.uri || doc.uri, range: symbol.declaration.selectionRange } : null });
        return send({ id, result: symbol ? { contents: { kind: 'plaintext', value: symbol.declaration?.signature || symbol.signature }, range: symbol.range } : null });
    }
    if (id !== undefined) error(id, -32601, 'Method not found');
}

function stop(code) {
    if (exiting) return;
    exiting = true;
    for (const doc of documents.values()) cancel(doc);
    cancel(active);
    // Allow pending cleanup and final protocol writes to finish; no new checks.
    shutdown = true;
    process.stdin.pause();
    const finish = () => {
        if (pumping) return setTimeout(finish, 10);
        process.stdout.end(() => process.exit(code));
    };
    finish();
}
const reader = new FrameReader(receive);
process.stdin.on('data', chunk => { try { reader.push(chunk); } catch { stop(1); } });
process.stdin.on('end', () => stop(shutdown ? 0 : 1));
process.stdin.on('error', () => stop(1));
process.stdout.on('error', () => stop(1));
process.once('SIGTERM', () => stop(1));
process.once('SIGINT', () => stop(1));
