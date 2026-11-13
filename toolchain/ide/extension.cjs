const vscode = require('vscode');
const { spawn } = require('node:child_process');
const path = require('node:path');
const { pathToFileURL } = require('node:url');

let stopCurrent;
async function activate(context) {
    const { prepareTaskTarget } = await import(pathToFileURL(path.join(__dirname, 'task-targets.mjs')).href);
    const { canonicalFile } = await import(pathToFileURL(path.join(__dirname, 'language-server/packages.mjs')).href);
    const { FrameReader, frame } = await import(pathToFileURL(path.join(__dirname, 'language-server/transport.mjs')).href);
    const diagnostics = vscode.languages.createDiagnosticCollection('sovra');
    const output = vscode.window.createOutputChannel('Sovra Language Server');
    context.subscriptions.push(diagnostics, output);
    let child, ready = false, sequence = 0;
    const pending = new Map();
    const opened = new Set();
    const selector = [{ language: 'sovra', scheme: 'file' }, { language: 'sovra', scheme: 'untitled' }];
    function stop() {
        ready = false;
        const previous = child; child = undefined;
        opened.clear(); diagnostics.clear();
        for (const value of pending.values()) { clearTimeout(value.timer); value.reject(new Error('Language server stopped')); }
        pending.clear();
        if (!previous || previous.exitCode !== null || previous.stdin.destroyed) return Promise.resolve();
        return new Promise(done => {
            const timer = setTimeout(() => previous.kill(), 2000);
            previous.once('close', () => { clearTimeout(timer); done(); });
            const retiring = new FrameReader(message => {
                if (message?.id === 'sovra-shutdown') {
                    try { previous.stdin.write(frame({ jsonrpc: '2.0', method: 'exit' })); } catch { previous.kill(); }
                }
            });
            previous.stdout.on('data', chunk => { try { retiring.push(chunk); } catch { previous.kill(); } });
            try { previous.stdin.write(frame({ jsonrpc: '2.0', id: 'sovra-shutdown', method: 'shutdown', params: null })); }
            catch { previous.kill(); }
        });
    }
    stopCurrent = stop;
    context.subscriptions.push({ dispose: stop });
    function send(message) {
        if (!child || child.stdin.destroyed) throw new Error('Sovra language server is unavailable');
        if (child.stdin.writableLength > 4 * 1024 * 1024) { stop(); throw new Error('Language server input backlog exceeded'); }
        child.stdin.write(frame({ jsonrpc: '2.0', ...message }));
    }
    function request(method, params) {
        return new Promise((accept, reject) => {
            if (pending.size >= 64) return reject(new Error('Too many language-server requests'));
            const id = ++sequence;
            const timer = setTimeout(() => { pending.delete(id); reject(new Error('Language server request timed out')); }, 10000);
            pending.set(id, { accept, reject, timer });
            try { send({ id, method, params }); }
            catch (error) { clearTimeout(timer); pending.delete(id); reject(error); }
        });
    }
    const eligible = doc => doc.languageId === 'sovra' && ['file', 'untitled'].includes(doc.uri.scheme);
    function open(doc) {
        if (!ready || !eligible(doc) || opened.has(doc.uri.toString())) return;
        try {
            send({ method: 'textDocument/didOpen', params: { textDocument: { uri: doc.uri.toString(), languageId: 'sovra', version: doc.version, text: doc.getText() } } });
            opened.add(doc.uri.toString());
        } catch (error) { output.appendLine(error.message); }
    }
    async function start() {
        stop();
        if (!vscode.workspace.isTrusted) { output.appendLine('Trust the workspace to enable compiler diagnostics.'); return; }
        const config = vscode.workspace.getConfiguration('sovra');
        const worker = spawn(config.get('nodePath', 'node'), [path.join(__dirname, 'language-server/server.mjs'), config.get('compilerPath', 'svr')], { windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'] });
        child = worker;
        const reader = new FrameReader(message => {
            if (child !== worker || !message) return;
            if (message.id !== undefined) {
                const value = pending.get(message.id);
                if (value) { clearTimeout(value.timer); pending.delete(message.id); message.error ? value.reject(Object.assign(new Error(message.error.message), { code: message.error.code })) : value.accept(message.result); }
            } else if (message.method === 'textDocument/publishDiagnostics') {
                const { uri, version, diagnostics: items } = message.params;
                const doc = vscode.workspace.textDocuments.find(value => value.uri.toString() === uri);
                if (!doc || doc.version !== version) return;
                diagnostics.set(doc.uri, items.map(item => {
                    const range = new vscode.Range(item.range.start.line, item.range.start.character, item.range.end.line, item.range.end.character);
                    const diagnostic = new vscode.Diagnostic(range, item.message, item.severity === 2 ? vscode.DiagnosticSeverity.Warning : vscode.DiagnosticSeverity.Error);
                    diagnostic.source = item.source; diagnostic.code = item.code;
                    return diagnostic;
                }));
            } else if (message.method === 'window/logMessage') output.appendLine(message.params.message);
        });
        worker.stdout.on('data', chunk => { try { reader.push(chunk); } catch (error) { output.appendLine(error.message); if (child === worker) stop(); } });
        worker.stderr.on('data', chunk => output.appendLine(chunk.toString('utf8').slice(0, 4096)));
        worker.stdin.on('error', () => {});
        worker.on('error', error => { output.appendLine(`Cannot start server: ${error.message}`); if (child === worker) stop(); });
        worker.on('close', () => { if (child === worker) { output.appendLine('Language server stopped. Use Sovra: Restart Language Server to reconnect.'); stop(); } });
        try {
            await request('initialize', { processId: process.pid, capabilities: { general: { positionEncodings: ['utf-16'] } }, rootUri: vscode.workspace.workspaceFolders?.[0]?.uri.toString() || null, workspaceFolders: vscode.workspace.workspaceFolders?.map(f => ({ uri: f.uri.toString(), name: f.name })) || [] });
            if (child !== worker) return;
            send({ method: 'initialized', params: {} }); ready = true;
            for (const doc of vscode.workspace.textDocuments) open(doc);
        } catch (error) { output.appendLine(error.message); if (child === worker) stop(); }
    }
    context.subscriptions.push(vscode.workspace.onDidOpenTextDocument(open));
    context.subscriptions.push(vscode.workspace.onDidChangeTextDocument(event => {
        const doc = event.document;
        if (!ready || !eligible(doc)) return;
        diagnostics.delete(doc.uri);
        if (!opened.has(doc.uri.toString())) return open(doc);
        try { send({ method: 'textDocument/didChange', params: { textDocument: { uri: doc.uri.toString(), version: doc.version }, contentChanges: [{ text: doc.getText() }] } }); }
        catch (error) { output.appendLine(error.message); }
    }));
    context.subscriptions.push(vscode.workspace.onDidCloseTextDocument(doc => {
        if (!ready || !opened.delete(doc.uri.toString())) return;
        diagnostics.delete(doc.uri);
        try { send({ method: 'textDocument/didClose', params: { textDocument: { uri: doc.uri.toString() } } }); } catch {}
    }));
    for (const glob of ['**/*.svr', '**/sovra.toml']) {
        const watcher = vscode.workspace.createFileSystemWatcher(glob);
        context.subscriptions.push(watcher);
        for (const [event, type] of [['onDidCreate', 1], ['onDidChange', 2], ['onDidDelete', 3]]) {
            context.subscriptions.push(watcher[event](uri => {
                if (!ready) return;
                try { send({ method: 'workspace/didChangeWatchedFiles', params: { changes: [{ uri: uri.toString(), type }] } }); }
                catch (error) { output.appendLine(error.message); }
            }));
        }
    }
    context.subscriptions.push(vscode.workspace.onDidGrantWorkspaceTrust(start));
    context.subscriptions.push(vscode.workspace.onDidChangeWorkspaceFolders(start));
    context.subscriptions.push(vscode.commands.registerCommand('sovra.restartServer', start));
    context.subscriptions.push(vscode.languages.registerCompletionItemProvider(selector, {
        async provideCompletionItems(doc, position, cancellation) {
            if (!ready || cancellation.isCancellationRequested) return [];
            try {
                const items = await request('textDocument/completion', { textDocument: { uri: doc.uri.toString() }, position: { line: position.line, character: position.character } });
                if (cancellation.isCancellationRequested) return [];
                return items.map(value => { const item = new vscode.CompletionItem(value.label, vscode.CompletionItemKind.Keyword); item.detail = value.detail; return item; });
            } catch { return []; }
        },
    }));
    const rangeOf = range => new vscode.Range(range.start.line, range.start.character, range.end.line, range.end.character);
    const query = async (method, doc, position) => {
        if (!ready) return null;
        const version = doc.version;
        try {
            const result = await request(method, { textDocument: { uri: doc.uri.toString() }, ...(position ? { position: { line: position.line, character: position.character } } : {}) });
            return version === doc.version ? result : null;
        } catch (error) { output.appendLine(`${method}: ${error.message}`); return null; }
    };
    context.subscriptions.push(vscode.languages.registerHoverProvider(selector, {
        async provideHover(doc, position) {
            const result = await query('textDocument/hover', doc, position);
            if (!result) return undefined;
            const content = new vscode.MarkdownString(); content.appendText(result.contents.value);
            return new vscode.Hover(content, rangeOf(result.range));
        },
    }));
    context.subscriptions.push(vscode.languages.registerDefinitionProvider(selector, {
        async provideDefinition(doc, position) {
            const result = await query('textDocument/definition', doc, position);
            return result ? new vscode.Location(vscode.Uri.parse(result.uri), rangeOf(result.range)) : undefined;
        },
    }));
    context.subscriptions.push(vscode.languages.registerDocumentSymbolProvider(selector, {
        async provideDocumentSymbols(doc) {
            const result = await query('textDocument/documentSymbol', doc);
            return (result || []).map(item => new vscode.DocumentSymbol(item.name, item.detail, vscode.SymbolKind.Function, rangeOf(item.range), rangeOf(item.selectionRange)));
        },
    }));
    for (const [name, args] of Object.entries({ check: ['check'], buildIR: ['build', '--emit', 'ir'], buildJS: ['build', '--emit', 'js'], run: ['run'] })) {
        context.subscriptions.push(vscode.commands.registerCommand(`sovra.${name}`, async () => {
            if (!vscode.workspace.isTrusted) return vscode.window.showWarningMessage('Trust this workspace before invoking the Sovra compiler.');
            const doc = vscode.window.activeTextEditor?.document;
            if (!doc || !eligible(doc) || doc.uri.scheme !== 'file') return vscode.window.showWarningMessage('Save a Sovra source file before running this command.');
            if (!ready) { void vscode.window.showWarningMessage('Restart the Sovra language server before launching this task.'); return; }
            try {
                const target = await prepareTaskTarget(doc, () => vscode.workspace.textDocuments,
                    source => request('sovra/taskTarget', { textDocument: { uri: source.uri.toString(), version: source.version } }), canonicalFile);
                const scope = vscode.workspace.getWorkspaceFolder(doc.uri) || vscode.TaskScope.Global;
                const commandArgs = name === 'check' && target.package ? ['check', '--executable-package'] : args;
                const task = new vscode.Task({ type: 'sovra', action: name, package: target.package, target: target.target }, scope, name, 'Sovra', new vscode.ProcessExecution(vscode.workspace.getConfiguration('sovra').get('compilerPath', 'svr'), [...commandArgs, target.target], { cwd: target.cwd }));
                await vscode.tasks.executeTask(task);
            } catch (error) { void vscode.window.showErrorMessage(error.message); }
        }));
    }
    await start();
}
function deactivate() { return stopCurrent?.(); }
module.exports = { activate, deactivate };
