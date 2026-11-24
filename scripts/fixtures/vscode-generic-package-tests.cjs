const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const crypto = require('node:crypto');
const vscode = require('vscode');
async function until(check, label) {
    const end = Date.now() + 30000;
    while (Date.now() < end) {
        const result = await check(); if (result) return result;
        await new Promise(done => setTimeout(done, 75));
    }
    throw new Error(`Generic package host timeout: ${label}`);
}
async function replace(doc, text) {
    const edit = new vscode.WorkspaceEdit();
    edit.replace(doc.uri, new vscode.Range(doc.positionAt(0), doc.positionAt(doc.getText().length)), text);
    assert.equal(await vscode.workspace.applyEdit(edit), true);
}
async function run() {
    assert.equal(vscode.workspace.isTrusted, true);
    const root = process.env.SOVRA_TEST_WORKSPACE, app = path.join(root, 'app');
    const appPath = path.join(app, 'src/main.svr'), providerPath = path.join(root, 'lib/values.svr');
    const manifestPath = path.join(app, 'sovra.toml'), notesPath = path.join(root, 'notes.svr');
    const config = vscode.workspace.getConfiguration('sovra');
    for (const [key, value] of Object.entries({ compilerPath: process.env.SOVRA_TEST_COMPILER, nodePath: process.env.SOVRA_TEST_NODE,
        experimentalV2: false, experimentalGenerics: true, experimentalGenericPackageDiagnostics: true, experimentalGenericPackageTasks: true })) {
        await config.update(key, value, vscode.ConfigurationTarget.Global);
    }
    await vscode.workspace.getConfiguration('task').update('saveBeforeRun', 'never', vscode.ConfigurationTarget.Global);
    const extension = vscode.extensions.getExtension('sovra-local.sovra-language');
    assert.ok(extension); await extension.activate();
    const doc = await vscode.workspace.openTextDocument(vscode.Uri.file(appPath)); await vscode.window.showTextDocument(doc);
    const originalApp = await fs.readFile(appPath, 'utf8'), originalProvider = await fs.readFile(providerPath, 'utf8');
    const point = (marker, offset = 0) => {
        const index = doc.getText().indexOf(marker); assert.ok(index >= 0, marker);
        return doc.positionAt(index + offset);
    };
    const definitions = (marker, offset) => vscode.commands.executeCommand('vscode.executeDefinitionProvider', doc.uri, point(marker, offset));
    const closed = await until(async () => (await definitions('Box::<Int>'))?.[0], 'closed dependency type');
    assert.equal(await fs.realpath(closed.uri.fsPath), await fs.realpath(providerPath));
    assert.equal(closed.range.start.line, 1);
    const provider = await vscode.workspace.openTextDocument(vscode.Uri.file(providerPath));
    await replace(provider, originalProvider.replace('return x;', 'return true;'));
    await until(() => vscode.languages.getDiagnostics(provider.uri).some(d => d.code === 'E3123'), 'unsaved dependency rejection');
    await until(async () => !(await definitions('Box::<Int>'))?.length, 'invalid graph retires navigation');
    assert.equal(await fs.readFile(providerPath, 'utf8'), originalProvider);
    const fixedProvider = originalProvider.replace('\r\n', '\r\n\r\n');
    await replace(provider, fixedProvider);
    await until(async () => (await definitions('Box::<Int>'))?.[0]?.range.start.line === 2, 'unsaved dependency relocation');
    assert.equal(vscode.languages.getDiagnostics(provider.uri).length, 0);
    assert.equal(vscode.languages.getDiagnostics(doc.uri).length, 0);
    for (const [marker, offset, targetDoc, declaration, label] of [
        ['id::<Alias', 0, provider, 'id<T>', 'export fn id<T>(x:T)->T'],
        ['x:Alias', 2, doc, 'Alias<T>=', 'type Alias<T>=api :: values :: Box<T>;'],
        ['saved:Alias', 6, doc, 'Alias<T>=', 'type Alias<T>=api :: values :: Box<T>;'],
        ['id::<Alias', 5, doc, 'Alias<T>=', 'type Alias<T>=api :: values :: Box<T>;'],
        ['Box::<Int>', 0, provider, 'Box<T>', 'export struct Box<T>'],
        ['{value:1}', 1, provider, 'value:T', 'value:T'],
        ['item.value', 5, provider, 'value:T', 'value:T'],
        ['(saved)', 1, doc, 'saved:Alias', 'let saved: Box<T>'],
    ]) {
        const result = await definitions(marker, offset); assert.equal(result.length, 1, marker);
        assert.equal(await fs.realpath(result[0].uri.fsPath), await fs.realpath(targetDoc.uri.fsPath));
        const expected = targetDoc.getText().indexOf(declaration); assert.ok(expected >= 0, declaration);
        assert.equal(targetDoc.offsetAt(result[0].range.start), expected, marker);
        const hovers = await vscode.commands.executeCommand('vscode.executeHoverProvider', doc.uri, point(marker, offset));
        // Compiler headers exclude package export markers; compare their meaningful suffix.
        const rendered = new vscode.MarkdownString().appendText(label.replace(/^export /, '')).value;
        assert.ok(hovers.some(h => h.contents.some(c => c.value.includes(rendered))), `${marker}: ${JSON.stringify(hovers)}`);
    }
    const outline = await vscode.commands.executeCommand('vscode.executeDocumentSymbolProvider', provider.uri);
    assert.ok(outline.some(item => item.name === 'value' && item.kind === vscode.SymbolKind.Field));
    const edits = await vscode.commands.executeCommand('vscode.executeFormatDocumentProvider', doc.uri, { tabSize: 4, insertSpaces: true });
    assert.ok(edits.length > 0); assert.equal(doc.getText(), originalApp);
    assert.equal(await fs.readFile(appPath, 'utf8'), originalApp);
    assert.equal(await fs.readFile(providerPath, 'utf8'), originalProvider);
    const notes = await vscode.workspace.openTextDocument(vscode.Uri.file(notesPath)), originalNotes = notes.getText();
    await replace(notes, originalNotes + '\n// unrelated draft');
    const manifest = await vscode.workspace.openTextDocument(vscode.Uri.file(manifestPath)), originalManifest = manifest.getText();
    await replace(manifest, originalManifest + '\n# unsaved configuration');
    let starts = 0; const launched = vscode.tasks.onDidStartTaskProcess(event => { if (event.execution.task.definition.type === 'sovra') starts++; });
    const tasks = [];
    try {
        await vscode.window.showTextDocument(doc); await vscode.commands.executeCommand('sovra.check');
        await new Promise(done => setTimeout(done, 250)); assert.equal(starts, 0);
        assert.equal(manifest.isDirty, true); assert.equal(provider.isDirty, true);
        assert.equal(await fs.readFile(manifestPath, 'utf8'), originalManifest);
        await replace(manifest, originalManifest); assert.equal(await manifest.save(), true);
        await replace(doc, originalApp + '\n// related draft');
        for (const action of ['check', 'run']) {
            await vscode.window.showTextDocument(doc);
            const complete = new Promise((accept, reject) => {
                const timer = setTimeout(() => { listener.dispose(); reject(new Error(`${action} timeout`)); }, 30000);
                const listener = vscode.tasks.onDidEndTaskProcess(event => {
                    if (event.execution.task.definition.action === action) { clearTimeout(timer); listener.dispose(); accept(event); }
                });
            });
            await vscode.commands.executeCommand(`sovra.${action}`);
            const event = await complete, task = event.execution.task;
            assert.equal(event.exitCode, 0); assert.equal(task.definition.package, true);
            assert.equal(await fs.realpath(task.definition.target), await fs.realpath(app));
            assert.equal(task.execution.options.cwd, task.definition.target);
            assert.ok(task.execution.args.includes('--experimental-generics'));
            assert.equal(task.execution.args.at(-1), task.definition.target);
            tasks.push({ action, exit_code: event.exitCode, args: task.execution.args, cwd: task.execution.options.cwd });
        }
        assert.equal(starts, 2);
    } finally { launched.dispose(); }
    assert.equal(notes.isDirty, true); assert.equal(await fs.readFile(notesPath, 'utf8'), originalNotes);
    assert.equal(await fs.readFile(providerPath, 'utf8'), fixedProvider);
    assert.equal(await fs.readFile(appPath, 'utf8'), doc.getText());
    await vscode.commands.executeCommand('sovra.restartServer');
    await until(async () => (await definitions('item.value', 5))?.[0]?.range.start.line === 2, 'restart package navigation');
    const compilerSha256 = crypto.createHash('sha256').update(await fs.readFile(process.env.SOVRA_TEST_COMPILER)).digest('hex');
    await fs.writeFile(path.join(process.env.SOVRA_TEST_ARTIFACTS, 'vscode-generic-package-verification.json'), JSON.stringify({ success: true, host: vscode.version, platform: process.platform, compiler_sha256: compilerSha256,
        checks: ['closed dependency type navigation', 'unsaved dependency diagnostics', 'invalid graph clears navigation', 'UTF-16/CRLF relocated dependency definitions', 'function/alias/local/field navigation and hover', 'field outlines', 'format edits without saving', 'dirty manifest task refusal', 'related-only source saves', 'package Check/Run target and cwd', 'unrelated draft preservation', 'server restart'], tasks }, null, 2));
}
module.exports = { run };
