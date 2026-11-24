const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const vscode = require('vscode');
async function until(check, label) {
    const end = Date.now() + 30000;
    while (Date.now() < end) { if (await check()) return; await new Promise(done => setTimeout(done, 75)); }
    throw new Error(`Generic host timeout: ${label}`);
}
async function run() {
    assert.equal(vscode.workspace.isTrusted, true);
    const config = vscode.workspace.getConfiguration('sovra');
    for (const [key, value] of Object.entries({ compilerPath: process.env.SOVRA_TEST_COMPILER, nodePath: process.env.SOVRA_TEST_NODE, experimentalV2: false, experimentalGenerics: true })) await config.update(key, value, vscode.ConfigurationTarget.Global);
    await vscode.workspace.getConfiguration('task').update('saveBeforeRun', 'never', vscode.ConfigurationTarget.Global);
    await vscode.extensions.getExtension('sovra-local.sovra-language').activate();
    const file = path.join(process.env.SOVRA_TEST_WORKSPACE, 'src/main.svr');
    const doc = await vscode.workspace.openTextDocument(vscode.Uri.file(file)); await vscode.window.showTextDocument(doc);
    async function replace(text) {
        const edit = new vscode.WorkspaceEdit(); edit.replace(doc.uri, new vscode.Range(doc.positionAt(0), doc.positionAt(doc.getText().length)), text);
        assert.equal(await vscode.workspace.applyEdit(edit), true);
    }
    await until(async () => {
        const result = await vscode.commands.executeCommand('vscode.executeCompletionItemProvider', doc.uri, doc.positionAt(0));
        const labels = result?.items.map(item => item.label) || [];
        return labels.includes('match') && !labels.includes('mod');
    }, 'generic initialization');
    const invalid = '// λ😀\r\nfn bad(x:std::Option<Int>)->Int{let label="λ😀";let n=x?;return 1;}fn main(){}';
    await replace(invalid);
    await until(() => vscode.languages.getDiagnostics(doc.uri).some(item => item.code === 'E3123'), 'generic diagnostic');
    const diagnostic = vscode.languages.getDiagnostics(doc.uri).find(item => item.code === 'E3123');
    assert.equal(doc.getText(diagnostic.range), '?');
    assert.equal(doc.offsetAt(diagnostic.range.start), doc.getText().indexOf('?'));
    const valid = 'type Maybe<T>=std::Option<T>;fn take<T>(x:Maybe<T>)->std::Option<T>{let n=x?;return std::Option::<T>::Some(n);}fn main(){let value=take::<Int>(std::Option::<Int>::None());}';
    await replace(valid); await until(() => vscode.languages.getDiagnostics(doc.uri).length === 0, 'generic clean buffer');
    const formatting = await vscode.commands.executeCommand('vscode.executeFormatDocumentProvider', doc.uri, { tabSize: 4, insertSpaces: true });
    assert.ok(formatting.length > 0);
    let formatted = doc.getText();
    const ordered = formatting.map(edit => ({ start: doc.offsetAt(edit.range.start), end: doc.offsetAt(edit.range.end), text: edit.newText })).sort((a, b) => b.start - a.start);
    for (const edit of ordered) formatted = formatted.slice(0, edit.start) + edit.text + formatted.slice(edit.end);
    assert.ok(formatted.includes('x?'));
    assert.notEqual(formatted, valid);
    assert.equal(doc.getText(), valid); // Asking for edits does not apply or save them.
    const outline=await vscode.commands.executeCommand('vscode.executeDocumentSymbolProvider',doc.uri);
    assert.deepEqual(outline.map(item=>item.name),['Maybe','take','main']);
    const hover=await vscode.commands.executeCommand('vscode.executeHoverProvider',doc.uri,doc.positionAt(valid.indexOf('take<T>')+1));
    const expected=new vscode.MarkdownString().appendText('fn take<T>(x:Maybe<T>)->std::Option<T>').value;
    assert.ok(hover.some(item=>item.contents.some(content=>content.value===expected)));
    const definitions=await vscode.commands.executeCommand('vscode.executeDefinitionProvider',doc.uri,doc.positionAt(valid.indexOf('take::<Int>')+1));assert.equal(definitions.length,1);assert.equal(doc.getText(definitions[0].range),'take');
    const callHover=await vscode.commands.executeCommand('vscode.executeHoverProvider',doc.uri,doc.positionAt(valid.indexOf('take::<Int>')+1));assert.ok(callHover.some(item=>item.contents.some(content=>content.value===expected)));
    for (const [use, name, label] of [['x?', 'x', 'parameter x: std::Option<T>'], ['Some(n)', 'n', 'let n: T']]) {
        const at=valid.indexOf(use)+(use==='Some(n)'?5:0);
        const defs=await vscode.commands.executeCommand('vscode.executeDefinitionProvider',doc.uri,doc.positionAt(at));
        assert.equal(defs.length,1);assert.equal(doc.getText(defs[0].range),name);
        const declaration=valid.indexOf(name==='x'?'(x:':'let n')+(name==='x'?1:4);
        assert.equal(doc.offsetAt(defs[0].range.start),declaration);
        const hovers=await vscode.commands.executeCommand('vscode.executeHoverProvider',doc.uri,doc.positionAt(at));
        const rendered=new vscode.MarkdownString().appendText(label).value;
        assert.ok(hovers.some(h=>h.contents.some(c=>c.value===rendered)));
    }
    const typePosition=doc.positionAt(valid.indexOf('x:Maybe')+2);
    const typeDefs=await vscode.commands.executeCommand('vscode.executeDefinitionProvider',doc.uri,typePosition);
    assert.equal(typeDefs.length,1);assert.equal(doc.getText(typeDefs[0].range),'Maybe');assert.equal(doc.offsetAt(typeDefs[0].range.start),5);
    const typeHover=await vscode.commands.executeCommand('vscode.executeHoverProvider',doc.uri,typePosition);
    const typeLabel=new vscode.MarkdownString().appendText('type Maybe<T>=std::Option<T>;').value;
    assert.ok(typeHover.some(h=>h.contents.some(c=>c.value===typeLabel)));
    const tasks = [];
    for (const action of ['check', 'run']) {
        const complete = new Promise((accept, reject) => {
            const timer = setTimeout(() => { listener.dispose(); reject(new Error(`${action} timeout`)); }, 30000);
            const listener = vscode.tasks.onDidEndTaskProcess(event => { if (event.execution.task.definition.action === action) { clearTimeout(timer); listener.dispose(); accept(event); } });
        });
        await vscode.commands.executeCommand(`sovra.${action}`);
        const event = await complete; assert.equal(event.exitCode, 0); assert.equal(event.execution.task.definition.package, false);
        assert.ok(event.execution.task.execution.args.includes('--experimental-generics'));
        tasks.push({ action, code: event.exitCode, args: event.execution.task.execution.args });
    }
    const launches = []; const listener = vscode.tasks.onDidStartTask(event => launches.push(event.execution.task));
    await vscode.commands.executeCommand('sovra.buildIR'); await vscode.commands.executeCommand('sovra.buildJS');
    await new Promise(done => setTimeout(done, 350)); listener.dispose(); assert.equal(launches.length, 0);
    assert.equal(await fs.readFile(file, 'utf8'), valid);
    await fs.writeFile(path.join(process.env.SOVRA_TEST_ARTIFACTS, 'vscode-generics-verification.json'), JSON.stringify({ success: true, host: vscode.version, checks: ['explicit generic diagnostics', 'UTF-16 original question range', 'unsaved buffer replacement', 'checked declaration outline/hover', 'bound call definition', 'checked alias annotation definition/hover', 'checked parameter and local definition/type hover', 'syntax-preserving generic formatting edits', 'standalone Check/Run under invalid manifest', 'generic task arguments and success', 'IR/JS task refusal'], tasks }, null, 2));
}
module.exports = { run };
