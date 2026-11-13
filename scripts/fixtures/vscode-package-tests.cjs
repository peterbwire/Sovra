const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const vscode = require('vscode');
async function until(check, label) {
    const end = Date.now() + 30000;
    while (Date.now() < end) { const result = await check(); if (result) return result; await new Promise(done => setTimeout(done, 75)); }
    throw new Error(`Package host condition timed out: ${label}`);
}
async function replace(doc, text) {
    const edit = new vscode.WorkspaceEdit(); edit.replace(doc.uri, new vscode.Range(doc.positionAt(0), doc.positionAt(doc.getText().length)), text);
    assert.equal(await vscode.workspace.applyEdit(edit), true);
}
async function run() {
    assert.equal(vscode.workspace.isTrusted, true);
    const root = process.env.SOVRA_TEST_WORKSPACE;
    const app = path.join(root, 'app'), libPath = path.join(root, 'lib/main.svr'), manifestPath = path.join(app, 'sovra.toml');
    const appPath = path.join(app, 'src/main.svr'), notesPath = path.join(root, 'notes.svr');
    await vscode.workspace.getConfiguration('task').update('saveBeforeRun', 'never', vscode.ConfigurationTarget.Global);
    assert.equal(vscode.workspace.getConfiguration('task').get('saveBeforeRun'), 'never');
    const config = vscode.workspace.getConfiguration('sovra');
    await config.update('compilerPath', process.env.SOVRA_TEST_COMPILER, vscode.ConfigurationTarget.Global);
    await config.update('nodePath', process.env.SOVRA_TEST_NODE, vscode.ConfigurationTarget.Global);
    const extension = vscode.extensions.getExtension('sovra-local.sovra-language'); assert.ok(extension); await extension.activate();
    const appDoc = await vscode.workspace.openTextDocument(vscode.Uri.file(appPath)); await vscode.window.showTextDocument(appDoc);
    const point = () => appDoc.positionAt(appDoc.getText().indexOf('api::exports::value'));
    const definitions = () => vscode.commands.executeCommand('vscode.executeDefinitionProvider', appDoc.uri, point());
    const definition = await until(async () => (await definitions())?.[0], 'closed dependency navigation');
    assert.equal(await fs.realpath(definition.uri.fsPath), await fs.realpath(libPath)); assert.equal(definition.range.start.line, 1);
    const libDoc = await vscode.workspace.openTextDocument(vscode.Uri.file(libPath));
    const originalLib = await fs.readFile(libPath, 'utf8'); const floating = originalLib.replace('-> Int', '-> Float').replace('return 1;', 'return 2;');
    await replace(libDoc, floating.replace('return 2;', 'missing(); return 2;'));
    await until(() => vscode.languages.getDiagnostics(libDoc.uri).some(d => d.code === 'E3004'), 'unsaved dependency diagnostic');
    const broken = vscode.languages.getDiagnostics(libDoc.uri).find(d => d.code === 'E3004');
    assert.equal(broken.range.start.line, 1); assert.equal(broken.range.start.character, libDoc.positionAt(libDoc.getText().indexOf('missing')).character);
    await replace(libDoc, floating);
    const hoverFloat = async () => (await vscode.commands.executeCommand('vscode.executeHoverProvider', appDoc.uri, point()))?.some(h => h.contents.some(c => c.value?.includes('Float')));
    await until(hoverFloat, 'unsaved dependency hover refresh'); assert.equal(await fs.readFile(libPath, 'utf8'), originalLib);
    const consumer = 'use api::exports; fn main() { let n: Float = api::exports::value(); print(n); }'; await replace(appDoc, consumer);
    const notes = await vscode.workspace.openTextDocument(vscode.Uri.file(notesPath)); const originalNotes = await fs.readFile(notesPath, 'utf8'); await replace(notes, originalNotes + '\n// unrelated draft');
    const manifest = await vscode.workspace.openTextDocument(vscode.Uri.file(manifestPath)); const originalManifest = await fs.readFile(manifestPath, 'utf8'); await replace(manifest, originalManifest + '\n# unsaved configuration');
    let starts = 0;
    const startListener = vscode.tasks.onDidStartTaskProcess(event => { if (event.execution.task.definition.type === 'sovra') starts++; });
    await vscode.window.showTextDocument(appDoc);
    await vscode.commands.executeCommand('sovra.check');
    await new Promise(done => setTimeout(done, 250));
    assert.equal(starts, 0, 'dirty manifest must block task launch'); assert.equal(manifest.isDirty, true); assert.equal(libDoc.isDirty, true); assert.equal(await fs.readFile(manifestPath, 'utf8'), originalManifest);
    await replace(manifest, originalManifest); assert.equal(await manifest.save(), true);
    const tasks = [];
    for (const action of ['check', 'buildIR', 'buildJS', 'run']) {
        await vscode.window.showTextDocument(appDoc);
        const complete = new Promise((accept, reject) => {
            const timer = setTimeout(() => { listener.dispose(); reject(new Error(`Package ${action} task timed out`)); }, 30000);
            const listener = vscode.tasks.onDidEndTaskProcess(event => { if (event.execution.task.definition.action === action) { clearTimeout(timer); listener.dispose(); accept(event); } });
        });
        await vscode.commands.executeCommand(`sovra.${action}`);
        const event = await complete; assert.equal(event.exitCode, 0);
        const task = event.execution.task; assert.equal(task.definition.package, true); assert.equal(await fs.realpath(task.definition.target), await fs.realpath(app));
        if (action === 'check') assert.ok(task.execution.args.includes('--executable-package'));
        assert.equal(task.execution.args.at(-1), task.definition.target); assert.equal(task.execution.options.cwd, task.definition.target);
        tasks.push({ action, exit_code: event.exitCode, target: task.definition.target, cwd: task.execution.options.cwd });
    }
    startListener.dispose(); assert.equal(starts, 4);
    assert.equal(await fs.readFile(libPath, 'utf8'), floating); assert.equal(await fs.readFile(appPath, 'utf8'), consumer);
    assert.equal(notes.isDirty, true); assert.equal(await fs.readFile(notesPath, 'utf8'), originalNotes);
    assert.equal(await fs.readFile(manifestPath, 'utf8'), originalManifest);
    await vscode.commands.executeCommand('sovra.restartServer'); await until(hoverFloat, 'package restart');
    const afterRestart = await until(async () => (await definitions())?.[0], 'package definition after restart'); assert.equal(await fs.realpath(afterRestart.uri.fsPath), await fs.realpath(libPath));
    await fs.writeFile(path.join(process.env.SOVRA_TEST_ARTIFACTS, 'vscode-package-verification.json'), JSON.stringify({ success: true, host: vscode.version, checks: ['workspace roots', 'nested entry package target', 'closed dependency definition', 'unsaved dependency diagnostics/hover', 'dirty manifest refusal', 'related-source saves', 'unrelated draft preservation with task.saveBeforeRun=never', 'check/IR/JS/run root arguments and cwd', 'restart'], tasks }, null, 2));
}
module.exports = { run };
