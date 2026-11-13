const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const vscode = require('vscode');
async function until(check) {
    const end = Date.now() + 20000;
    while (Date.now() < end) { if (check()) return; await new Promise(done => setTimeout(done, 50)); }
    throw new Error('VS Code diagnostics did not reach the expected state');
}
async function run() {
    const untrusted = process.env.SOVRA_TEST_UNTRUSTED === '1';
    assert.equal(vscode.workspace.isTrusted, !untrusted);
    const config = vscode.workspace.getConfiguration('sovra');
    await config.update('compilerPath', process.env.SOVRA_TEST_COMPILER, vscode.ConfigurationTarget.Global);
    await config.update('nodePath', process.env.SOVRA_TEST_NODE, vscode.ConfigurationTarget.Global);
    const extension = vscode.extensions.getExtension('sovra-local.sovra-language');
    assert.ok(extension);
    await extension.activate();
    const text = 'fn main() { print("λ😀"); print(missing) }';
    const doc = await vscode.workspace.openTextDocument({ language: 'sovra', content: text });
    await vscode.window.showTextDocument(doc);
    if (untrusted) {
        await vscode.commands.executeCommand('sovra.restartServer');
        await new Promise(done => setTimeout(done, 1500));
        assert.deepEqual(vscode.languages.getDiagnostics(doc.uri), []);
        await fs.writeFile(path.join(process.env.SOVRA_TEST_ARTIFACTS, 'vscode-untrusted-verification.json'), JSON.stringify({ success: true, host: vscode.version, checks: ['untrusted workspace activation', 'diagnostics disabled', 'restart remains gated'] }, null, 2));
        return;
    }
    await until(() => vscode.languages.getDiagnostics(doc.uri).some(value => value.source === 'sovra'));
    const diagnostic = vscode.languages.getDiagnostics(doc.uri)[0];
    assert.match(diagnostic.message, /missing/);
    assert.equal(diagnostic.range.start.character, text.indexOf('missing'));
    const items = await vscode.commands.executeCommand('vscode.executeCompletionItemProvider', doc.uri, new vscode.Position(0, 0));
    assert.ok(items.items.some(value => value.label === 'fn'));
    const edit = new vscode.WorkspaceEdit();
    edit.replace(doc.uri, new vscode.Range(doc.positionAt(0), doc.positionAt(doc.getText().length)), 'fn main() {}');
    assert.equal(await vscode.workspace.applyEdit(edit), true);
    await until(() => vscode.languages.getDiagnostics(doc.uri).length === 0);
    const symbolSource = 'fn helper(x: Int) -> Int { return x }\nfn main() { print("λ😀"); print(helper(2)) }';
    const symbolEdit = new vscode.WorkspaceEdit();
    symbolEdit.replace(doc.uri, new vscode.Range(doc.positionAt(0), doc.positionAt(doc.getText().length)), symbolSource);
    await vscode.workspace.applyEdit(symbolEdit);
    // Wait for the checked version's outline, not just eagerly cleared errors.
    let outline;
    for (let retry = 0; retry < 100; retry++) {
        outline = await vscode.commands.executeCommand('vscode.executeDocumentSymbolProvider', doc.uri);
        if (outline?.some(value => value.name === 'helper')) break;
        await new Promise(done => setTimeout(done, 50));
    }
    assert.ok(outline?.some(value => value.name === 'helper'), `Outline missing; diagnostics: ${JSON.stringify(vscode.languages.getDiagnostics(doc.uri))}`);
    const point = new vscode.Position(1, symbolSource.split('\n')[1].indexOf('helper(2)') + 2);
    const hover = await vscode.commands.executeCommand('vscode.executeHoverProvider', doc.uri, point);
    assert.ok(hover.some(value => value.contents.some(content => content.value?.includes('helper') && content.value?.includes('Int'))), JSON.stringify(hover));
    const definitions = await vscode.commands.executeCommand('vscode.executeDefinitionProvider', doc.uri, point);
    assert.equal(definitions[0].uri.toString(), doc.uri.toString());
    assert.equal(definitions[0].range.start.line, 0);
    assert.equal(definitions[0].range.start.character, 3);
    const localSource = 'fn keep(value: Int) -> Int {\n let x: Int = value;\n if true { let x = x + 1; print(x); }\n return x;\n}';
    const localEdit = new vscode.WorkspaceEdit();
    localEdit.replace(doc.uri, new vscode.Range(doc.positionAt(0), doc.positionAt(doc.getText().length)), localSource);
    await vscode.workspace.applyEdit(localEdit);
    await vscode.commands.executeCommand('vscode.executeDocumentSymbolProvider', doc.uri);
    const lines = localSource.split('\n');
    const parameterPoint = new vscode.Position(1, lines[1].indexOf('value') + 1);
    const parameterHover = await vscode.commands.executeCommand('vscode.executeHoverProvider', doc.uri, parameterPoint);
    assert.ok(parameterHover.some(value => value.contents.some(content => content.value?.includes('parameter') && content.value?.includes('value'))));
    const inner = await vscode.commands.executeCommand('vscode.executeDefinitionProvider', doc.uri, new vscode.Position(2, lines[2].indexOf('print(x)') + 6));
    assert.equal(inner[0].range.start.line, 2);
    assert.equal(inner[0].range.start.character, lines[2].indexOf('let x') + 4);
    const inferredHover = await vscode.commands.executeCommand('vscode.executeHoverProvider', doc.uri, new vscode.Position(2, lines[2].indexOf('print(x)') + 6));
    const expectedInferred = new vscode.MarkdownString().appendText('let x: Int (inferred)').value;
    assert.ok(inferredHover.some(value => value.contents.some(content => content.value === expectedInferred)), JSON.stringify(inferredHover.map(value => value.contents.map(content => content.value))));
    const outer = await vscode.commands.executeCommand('vscode.executeDefinitionProvider', doc.uri, new vscode.Position(3, lines[3].indexOf('x;')));
    assert.equal(outer[0].range.start.line, 1);
    const commands = await vscode.commands.getCommands(true);
    for (const name of ['sovra.restartServer', 'sovra.check', 'sovra.buildIR', 'sovra.buildJS', 'sovra.run']) assert.ok(commands.includes(name));
    await vscode.commands.executeCommand('sovra.restartServer');
    const invalid = new vscode.WorkspaceEdit();
    invalid.insert(doc.uri, new vscode.Position(0, 0), '@');
    await vscode.workspace.applyEdit(invalid);
    await until(() => vscode.languages.getDiagnostics(doc.uri).some(value => value.source === 'sovra'));
    const result = path.join(process.env.SOVRA_TEST_ARTIFACTS, 'vscode-verification.json');
    const diskPath = path.join(process.env.SOVRA_TEST_ARTIFACTS, 'source with spaces.svr');
    await fs.writeFile(diskPath, 'fn main() { print("IDE task verification") }');
    const diskDoc = await vscode.workspace.openTextDocument(vscode.Uri.file(diskPath));
    await vscode.window.showTextDocument(diskDoc);
    for (const action of ['check', 'buildIR', 'buildJS', 'run']) {
        const complete = new Promise((accept, reject) => {
            const timer = setTimeout(() => { listener.dispose(); reject(new Error(`Sovra ${action} task timed out`)); }, 20000);
            const listener = vscode.tasks.onDidEndTaskProcess(event => {
                if (event.execution.task.definition.action === action) { listener.dispose(); clearTimeout(timer); accept(event.exitCode); }
            });
        });
        await vscode.commands.executeCommand(`sovra.${action}`);
        assert.equal(await complete, 0);
    }
    await fs.writeFile(result, JSON.stringify({ success: true, host: vscode.version, checks: ['activation', 'unsaved diagnostics', 'Unicode range', 'basic completion', 'function hover/definition/outline', 'parameter/local inferred hover and shadowed definitions', 'edit refresh', 'check/IR/JS/run tasks with spaces in path', 'restart'] }, null, 2));
}
module.exports = { run };
