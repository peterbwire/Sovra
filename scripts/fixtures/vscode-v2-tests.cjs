const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const vscode = require('vscode');
async function until(check, label) {
    const end = Date.now() + 30000;
    while (Date.now() < end) { if (await check()) return; await new Promise(done => setTimeout(done, 75)); }
    throw new Error(`V2 host timeout: ${label}`);
}
async function run() {
    assert.equal(vscode.workspace.isTrusted, true);
    const config = vscode.workspace.getConfiguration('sovra');
    for (const [key, value] of Object.entries({ compilerPath: process.env.SOVRA_TEST_COMPILER, nodePath: process.env.SOVRA_TEST_NODE, experimentalV2: true })) await config.update(key, value, vscode.ConfigurationTarget.Global);
    await vscode.workspace.getConfiguration('task').update('saveBeforeRun', 'never', vscode.ConfigurationTarget.Global);
    await vscode.extensions.getExtension('sovra-local.sovra-language').activate();
    const file = path.join(process.env.SOVRA_TEST_WORKSPACE, 'src/main.svr');
    const doc = await vscode.workspace.openTextDocument(vscode.Uri.file(file)); await vscode.window.showTextDocument(doc);
    async function replace(text) {
        const edit = new vscode.WorkspaceEdit(); edit.replace(doc.uri, new vscode.Range(doc.positionAt(0), doc.positionAt(doc.getText().length)), text);
        assert.equal(await vscode.workspace.applyEdit(edit), true);
    }
    const words = async () => (await vscode.commands.executeCommand('vscode.executeCompletionItemProvider', doc.uri, doc.positionAt(0)))?.items.map(item => item.label);
    await until(async () => (await words())?.includes('enum'), 'v2 initialization');
    await replace('enum E{A(Int)} fn main(){let x:E=E::A(missing);}');
    await until(() => vscode.languages.getDiagnostics(doc.uri).some(d => d.code === 'E3001'), 'v2 diagnostic');
    await replace('mod schema{enum E{A(Int)}type Alias=E;}fn obtain(n:Int)->schema::Alias{return schema::Alias::A(n);}fn main(){let x:schema::Alias=obtain(1);match x{schema::Alias::A(n)=>{print(n);}}}');
    await until(() => vscode.languages.getDiagnostics(doc.uri).length === 0, 'clean enum');
    const edits = await vscode.commands.executeCommand('vscode.executeFormatDocumentProvider', doc.uri, { tabSize: 4, insertSpaces: true });
    assert.ok(edits.length > 0);
    const edit = new vscode.WorkspaceEdit(); edit.set(doc.uri, edits); assert.equal(await vscode.workspace.applyEdit(edit), true);
    assert.ok(doc.getText().includes('\n')); assert.ok(doc.getText().includes('enum E'));
    assert.equal(await fs.readFile(file, 'utf8'), 'fn main(){}');
    assert.deepEqual(await vscode.commands.executeCommand('vscode.executeFormatDocumentProvider', doc.uri, { tabSize: 4, insertSpaces: true }) || [], []);
    let outline;
    await until(async () => { outline = await vscode.commands.executeCommand('vscode.executeDocumentSymbolProvider', doc.uri); return outline?.length === 6; }, 'v2 outline');
    assert.deepEqual(outline.map(item => item.name), ['schema', 'schema::E', 'schema::E::A', 'schema::Alias', 'obtain', 'main']);
    const callPoint = doc.positionAt(doc.getText().lastIndexOf('obtain(1)') + 1);
    const definition = await vscode.commands.executeCommand('vscode.executeDefinitionProvider', doc.uri, callPoint);
    assert.equal(definition[0].uri.toString(), doc.uri.toString());
    assert.equal(doc.getText(definition[0].range), 'obtain');
    const hover = await vscode.commands.executeCommand('vscode.executeHoverProvider', doc.uri, callPoint);
    const expectedHover = new vscode.MarkdownString().appendText(outline.find(item => item.name === 'obtain').detail).value;
    assert.ok(hover.some(item => item.contents.some(content => content.value === expectedHover)), JSON.stringify(hover));
    const parameterUse = doc.positionAt(doc.getText().indexOf('schema::Alias::A(n)') + 17);
    const parameterDefinition = await vscode.commands.executeCommand('vscode.executeDefinitionProvider', doc.uri, parameterUse);
    assert.equal(doc.getText(parameterDefinition[0].range), 'n');
    assert.ok(parameterDefinition[0].range.start.isBefore(parameterUse));
    const payloadUse = doc.positionAt(doc.getText().lastIndexOf('print(n)') + 6);
    const payloadDefinition = await vscode.commands.executeCommand('vscode.executeDefinitionProvider', doc.uri, payloadUse);
    assert.equal(doc.getText(payloadDefinition[0].range), 'n');
    assert.notDeepEqual(payloadDefinition[0].range, parameterDefinition[0].range);
    const payloadHover = await vscode.commands.executeCommand('vscode.executeHoverProvider', doc.uri, payloadUse);
    const expectedPayload = new vscode.MarkdownString().appendText('match payload n: Int').value;
    assert.ok(payloadHover.some(item => item.contents.some(content => content.value === expectedPayload)));
    assert.equal(outline.find(item => item.name === 'schema::E').kind, vscode.SymbolKind.Enum);
    assert.equal(outline.find(item => item.name === 'schema::E::A').kind, vscode.SymbolKind.EnumMember);
    const variantPoint = doc.positionAt(doc.getText().indexOf('schema::Alias::A(n)') + 15);
    const variantDefinition = await vscode.commands.executeCommand('vscode.executeDefinitionProvider', doc.uri, variantPoint);
    assert.equal(doc.getText(variantDefinition[0].range), 'A');
    assert.deepEqual(variantDefinition[0].range, outline.find(item => item.name === 'schema::E::A').selectionRange);
    const variantHover = await vscode.commands.executeCommand('vscode.executeHoverProvider', doc.uri, variantPoint);
    const expectedVariant = new vscode.MarkdownString().appendText('schema::E::A(Int) -> schema::E').value;
    assert.ok(variantHover.some(item => item.contents.some(content => content.value === expectedVariant)));
    const tasks = [];
    for (const action of ['check', 'buildIR', 'run', 'buildJS']) {
        const complete = new Promise((accept, reject) => {
            const timer = setTimeout(() => { listener.dispose(); reject(new Error(`${action} timeout`)); }, 30000);
            const listener = vscode.tasks.onDidEndTaskProcess(event => { if (event.execution.task.definition.action === action) { clearTimeout(timer); listener.dispose(); accept(event); } });
        });
        await vscode.commands.executeCommand(`sovra.${action}`);
        const event = await complete; assert.equal(event.exitCode, action === 'buildJS' ? 2 : 0);
        assert.equal(event.execution.task.definition.package, false);
        assert.equal(event.execution.task.definition.target, await fs.realpath(file));
        assert.ok(event.execution.task.execution.args.includes('--experimental-v2'));
        tasks.push({ action, exit_code: event.exitCode });
    }
    await config.update('experimentalV2', false, vscode.ConfigurationTarget.Global); await vscode.commands.executeCommand('sovra.restartServer');
    await until(async () => (await words())?.includes('mod') && !(await words())?.includes('enum'), 'legacy restart');
    await config.update('experimentalV2', true, vscode.ConfigurationTarget.Global); await vscode.commands.executeCommand('sovra.restartServer');
    await until(async () => (await words())?.includes('enum'), 'v2 restart');
    await replace('mod methods{struct Point{x:Int}type Coordinate=Int;fn hidden(n:Int)->Int{return n;}export fn value()->Int{return methods::hidden(7);}}fn main(){print(methods::value());}');
    let moduleOutline;
    await until(async () => { moduleOutline = await vscode.commands.executeCommand('vscode.executeDocumentSymbolProvider', doc.uri); return moduleOutline?.some(item => item.name === 'methods::value') && moduleOutline?.some(item => item.name === 'methods' && item.kind === vscode.SymbolKind.Module); }, 'checked module functions');
    assert.equal(outline.find(item => item.name === 'schema::Alias').kind, vscode.SymbolKind.TypeParameter);
    assert.equal(moduleOutline.find(item => item.name === 'methods::Point').kind, vscode.SymbolKind.Struct);
    assert.equal(moduleOutline.find(item => item.name === 'methods::Coordinate').kind, vscode.SymbolKind.TypeParameter);
    const container = moduleOutline.find(item => item.name === 'methods');
    assert.equal(doc.getText(container.selectionRange), 'methods');
    assert.equal(doc.getText(container.range), doc.getText().slice(0, doc.getText().indexOf('fn main')));
    const modulePoint = doc.positionAt(doc.getText().lastIndexOf('methods::value()') + 10);
    const moduleDefinition = await vscode.commands.executeCommand('vscode.executeDefinitionProvider', doc.uri, modulePoint);
    assert.equal(doc.getText(moduleDefinition[0].range), 'value');
    await replace('mod typed{enum E{End}export fn pass(x:[E])->[E]{return x;}}fn main(){}');
    await until(async () => (await vscode.commands.executeCommand('vscode.executeDocumentSymbolProvider', doc.uri))?.some(item => item.name === 'typed::E'), 'enum header navigation');
    for (const fragment of ['x:[E]', '->[E]']) {
        const offset = doc.getText().indexOf(fragment) + fragment.length - 2;
        const definitions = await vscode.commands.executeCommand('vscode.executeDefinitionProvider', doc.uri, doc.positionAt(offset));
        assert.equal(doc.getText(definitions[0].range), 'E');
        assert.equal(doc.offsetAt(definitions[0].range.start), doc.getText().indexOf('enum E') + 5);
    }
    await replace('mod methods{fn hidden(){}}fn main(){methods::hidden();}');
    await until(() => vscode.languages.getDiagnostics(doc.uri).some(item => item.code === 'E3004'), 'private member refused');
    await replace('mod edits{fn pending(){missing();}}fn main(){}');
    await until(() => vscode.languages.getDiagnostics(doc.uri).some(item => item.code === 'E3004'), 'invalid module body diagnosed');
    const draftEdits = await vscode.commands.executeCommand('vscode.executeFormatDocumentProvider', doc.uri, { tabSize: 4, insertSpaces: true });
    assert.ok(draftEdits?.length > 0);
    const draftEdit = new vscode.WorkspaceEdit(); draftEdit.set(doc.uri, draftEdits); assert.equal(await vscode.workspace.applyEdit(draftEdit), true);
    await until(() => vscode.languages.getDiagnostics(doc.uri).some(item => item.code === 'E3004'), 'formatted module body remains invalid');
    assert.deepEqual(await vscode.commands.executeCommand('vscode.executeDocumentSymbolProvider', doc.uri) || [], []);
    await fs.writeFile(path.join(process.env.SOVRA_TEST_ARTIFACTS, 'vscode-v2-verification.json'), JSON.stringify({ success: true, host: vscode.version, checks: ['opt-in enum diagnostics', 'pure formatting and fixed point', 'checked root function outline/hover/definition', 'checked parameter/payload definitions and types', 'enum/variant outline kinds and constructor definition/hover', 'alias-qualified constructors/patterns retain nominal targets', 'type-only inline modules retain qualified ownership', 'module body formatting preserves semantic failure', 'qualified exported module functions and private refusal', 'checked module container outline', 'checked record and alias declarations', 'checked enum parameter and return annotation definitions', 'standalone tasks under invalid package', 'explicit JS refusal', 'pipeline restart'], tasks }, null, 2));
}
module.exports = { run };
