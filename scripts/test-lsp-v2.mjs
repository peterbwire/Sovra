// Actual compiler/LSP integration: explicit dialect, UTF-16, pure formatting and retirement.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, mkdir, writeFile, readFile, readdir, rm, realpath } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { FrameReader, frame } from '../toolchain/ide/language-server/transport.mjs';
const compiler = resolve(process.env.SOVRA_TEST_COMPILER || `target/debug/svr${process.platform === 'win32' ? '.exe' : ''}`);
const scratch = await mkdtemp(join(tmpdir(), 'sovra-lsp-v2-proof-'));
const before = new Set((await readdir(tmpdir())).filter(name => name.startsWith('sovra-lsp-format-')));
const root = join(scratch, 'broken package'); await mkdir(join(root, 'src'), { recursive: true });
await writeFile(join(root, 'sovra.toml'), '[project]\nname="bad"\nentry="src/main.svr"\n[dependencies.missing]\npath="../absent"\n');
const file = join(root, 'src/main.svr'); const disk = 'fn main() {}'; await writeFile(file, disk);
const uri = pathToFileURL(file).href;
const marker = join(scratch, 'never-created');
function client() {
    const child = spawn(process.execPath, ['toolchain/ide/language-server/server.mjs', compiler], { stdio: ['pipe', 'pipe', 'pipe'] });
    const queued = [], pending = []; let stderr = '', ended = false, exitCode;
    child.stderr.on('data', chunk => { stderr += chunk; });
    const closed = new Promise(done => child.on('close', code => { ended = true; exitCode = code; done(code); }));
    child.stdin.on('error', () => {});
    const reader = new FrameReader(message => {
        const index = pending.findIndex(waiter => waiter.predicate(message));
        if (index < 0) queued.push(message);
        else { const waiter = pending.splice(index, 1)[0]; clearTimeout(waiter.timer); waiter.accept(message); }
    });
    child.stdout.on('data', chunk => reader.push(chunk));
    const wait = (predicate, label) => new Promise((accept, reject) => {
        const index = queued.findIndex(predicate); if (index >= 0) return accept(queued.splice(index, 1)[0]);
        const record = { predicate, accept };
        record.timer = setTimeout(() => { pending.splice(pending.indexOf(record), 1); reject(new Error(`LSP v2 timeout ${label}; exited=${ended}/${exitCode}: ${stderr}; ${JSON.stringify(queued)}`)); }, 20000); pending.push(record);
    });
    const message = (method, params, id) => ({ jsonrpc: '2.0', method, params, ...(id === undefined ? {} : { id }) });
    return { child, queued, closed, wait,
        send(method, params, id) { child.stdin.write(frame(message(method, params, id))); },
        batch(items) { child.stdin.write(Buffer.concat(items.map(([method, params, id]) => frame(message(method, params, id))))); },
        response(id) { return wait(value => value.id === id, `response ${id}`); },
        diagnostic(version) { return wait(value => value.method === 'textDocument/publishDiagnostics' && value.params.uri === uri && value.params.version === version, `diagnostic ${version}`); },
        async stop() { if (!ended) { this.send('shutdown', null, 990); assert.equal((await this.response(990)).result, null); this.send('exit'); assert.equal(await closed, 0); } },
        kill() { if (!ended) child.kill(); for (const item of pending) clearTimeout(item.timer); },
    };
}
let c;
try {
    c = client();
    c.send('initialize', { initializationOptions: { experimentalV2: 'true' } }, 1);
    assert.equal((await c.response(1)).error.code, -32602);
    c.send('initialize', { rootUri: pathToFileURL(root).href, initializationOptions: { experimentalV2: true } }, 2);
    const caps = (await c.response(2)).result.capabilities;
    assert.equal(caps.experimental.sovraPipeline, 'v2-standalone'); assert.equal(caps.documentFormattingProvider, true);
    assert.equal(caps.hoverProvider, true); assert.equal(caps.definitionProvider, true); assert.equal(caps.documentSymbolProvider, true);
    c.send('initialized', {});
    const invalid = '// λ😀\r\nenum E{A(Int),End}\r\nfn main(){std::write_text(' + JSON.stringify(marker) + ',"bad");let e=E::A(missing);}';
    c.send('textDocument/didOpen', { textDocument: { uri, languageId: 'sovra', version: 1, text: invalid } });
    const d = (await c.diagnostic(1)).params.diagnostics.find(item => item.code === 'E3001'); assert.ok(d);
    assert.deepEqual(d.range, { start: { line: 2, character: invalid.split('\n')[2].indexOf('missing') }, end: { line: 2, character: invalid.split('\n')[2].indexOf('missing') + 7 } });
    c.send('textDocument/formatting', { textDocument: { uri } }, 100);
    assert.ok((await c.response(100)).result[0].newText.includes('missing'));
    c.send('textDocument/hover', { textDocument: { uri }, position: { line: 2, character: 3 } }, 3); assert.equal((await c.response(3)).result, null);
    c.send('textDocument/documentSymbol', { textDocument: { uri } }, 4); assert.deepEqual((await c.response(4)).result, []);
    const raw = 'mod schema{enum E{A(Int),End}type Alias=E;}fn main(){print("λ😀");let e=helper(1);match e{schema::Alias::A(n)=>{print(n);},schema::E::End()=>{}}}fn helper(n:Int)->schema::E{return schema::Alias::A(n);}';
    c.send('textDocument/didChange', { textDocument: { uri, version: 2 }, contentChanges: [{ text: raw }] }); assert.deepEqual((await c.diagnostic(2)).params.diagnostics, []);
    const point = { line: 0, character: raw.indexOf('helper(1)') + 1 };
    c.send('textDocument/hover', { textDocument: { uri }, position: point }, 101);
    assert.equal((await c.response(101)).result.contents.value, 'fn helper(n:Int)->schema::E');
    c.send('textDocument/definition', { textDocument: { uri }, position: point }, 102);
    assert.deepEqual((await c.response(102)).result, { uri, range: { start: { line: 0, character: raw.indexOf('helper(n:') }, end: { line: 0, character: raw.indexOf('helper(n:') + 6 } } });
    c.send('textDocument/documentSymbol', { textDocument: { uri } }, 103);
    const outline = (await c.response(103)).result;
    assert.deepEqual(outline.map(item => item.name), ['main', 'helper', 'schema::E', 'schema::E::A', 'schema::E::End', 'schema', 'schema::Alias']);
    assert.equal(outline.find(item => item.name === 'schema').kind, 2);
    assert.deepEqual(outline.find(item => item.name === 'schema').selectionRange, { start: { line: 0, character: 4 }, end: { line: 0, character: 10 } });
    c.send('textDocument/hover', { textDocument: { uri }, position: { line: 0, character: raw.lastIndexOf('schema::Alias::A(n)') } }, 104);
    assert.equal((await c.response(104)).result, null); // Type-prefix uses have no retained type-use evidence.
    for (const [id, start] of [[170, raw.lastIndexOf('schema::Alias::A(n)') + 15], [171, raw.indexOf('schema::Alias::A(n)=>') + 15]]) {
        c.send('textDocument/definition', { textDocument: { uri }, position: { line: 0, character: start } }, id);
        assert.deepEqual((await c.response(id)).result.range, { start: { line: 0, character: raw.indexOf('A(Int)') }, end: { line: 0, character: raw.indexOf('A(Int)') + 1 } });
    }
    c.send('textDocument/hover', { textDocument: { uri }, position: { line: 0, character: raw.lastIndexOf('schema::Alias::A(n)') + 15 } }, 172);
    assert.equal((await c.response(172)).result.contents.value, 'schema::E::A(Int) -> schema::E');
    c.send('textDocument/definition', { textDocument: { uri }, position: { line: 0, character: raw.indexOf('->schema::E') + 10 } }, 173);
    assert.deepEqual((await c.response(173)).result.range, { start: { line: 0, character: raw.indexOf('enum E') + 5 }, end: { line: 0, character: raw.indexOf('enum E') + 6 } });
    c.send('textDocument/formatting', { textDocument: { uri }, options: { tabSize: 2, insertSpaces: false } }, 5);
    const edits = (await c.response(5)).result; assert.equal(edits.length, 1); assert.equal(edits[0].range.end.character, raw.length); assert.ok(edits[0].newText.includes('    A(Int),'));
    assert.equal(await readFile(file, 'utf8'), disk);
    c.send('textDocument/didChange', { textDocument: { uri, version: 3 }, contentChanges: [{ text: edits[0].newText }] }); await c.diagnostic(3);
    c.send('textDocument/formatting', { textDocument: { uri } }, 6); assert.deepEqual((await c.response(6)).result, []);
    c.send('sovra/taskTarget', { textDocument: { uri, version: 3 } }, 7); const task = (await c.response(7)).result;
    assert.equal(task.package, false); assert.equal(task.target, await realpath(file)); assert.deepEqual(task.sources, [await realpath(file)]); assert.deepEqual(task.manifests, []);
    c.send('textDocument/completion', { textDocument: { uri } }, 8); const words = (await c.response(8)).result.map(value => value.label); assert.ok(words.includes('enum') && words.includes('match')); assert.ok(words.includes('mod') && !words.includes('use'));
    c.batch([['textDocument/formatting', { textDocument: { uri } }, 9], ['textDocument/didChange', { textDocument: { uri, version: 4 }, contentChanges: [{ text: raw }] }]]);
    assert.equal((await c.response(9)).error.code, -32801); await c.diagnostic(4);
    c.batch([['textDocument/formatting', { textDocument: { uri } }, 10], ['$/cancelRequest', { id: 10 }]]); assert.equal((await c.response(10)).error.code, -32800);
    const cr = 'fn main(){\rprint(1);\r}';
    c.send('textDocument/didChange', { textDocument: { uri, version: 5 }, contentChanges: [{ text: cr }] }); await c.diagnostic(5);
    c.send('textDocument/formatting', { textDocument: { uri } }, 11);
    assert.deepEqual((await c.response(11)).result[0].range.end, { line: 2, character: 1 });
    const large = 'fn main(){' + 'if true{'.repeat(120) + 'return;'.repeat(2300) + '}'.repeat(121);
    c.send('textDocument/didChange', { textDocument: { uri, version: 6 }, contentChanges: [{ text: large }] }); await c.diagnostic(6);
    c.send('textDocument/formatting', { textDocument: { uri } }, 12);
    assert.match((await c.response(12)).error.message, /exceeded|limit/i);
    c.batch([...Array.from({ length: 5 }, (_, i) => ['textDocument/formatting', { textDocument: { uri } }, 30 + i]),
        ...Array.from({ length: 4 }, (_, i) => ['$/cancelRequest', { id: 30 + i }])]);
    assert.equal((await c.response(34)).error.code, -32000);
    for (let i = 0; i < 4; i++) assert.equal((await c.response(30 + i)).error.code, -32800);
    const bindings = 'enum E{A(Int),End}fn helper(n:Int)->Int{let mut total=n;if true{let n=total;total=n;}return n;}fn main(){print("λ😀");let mut rows=[[1]];let i=0;rows[i]=[2];let e=E::A(helper(7));match e{E::A(n)=>{print(n);},E::End()=>{}}}';
    c.send('textDocument/didChange', { textDocument: { uri, version: 7 }, contentChanges: [{ text: bindings }] }); assert.deepEqual((await c.diagnostic(7)).params.diagnostics, []);
    const targets = [
        [bindings.indexOf('return n') + 7, bindings.indexOf('n:Int')],
        [bindings.lastIndexOf('total=n;') + 6, bindings.indexOf('let n=total') + 4],
        [bindings.indexOf('rows[i]='), bindings.indexOf('let mut rows') + 8],
        [bindings.indexOf('rows[i]=') + 5, bindings.indexOf('let i=') + 4],
        [bindings.lastIndexOf('print(n)') + 6, bindings.indexOf('E::A(n)=>') + 5],
        [bindings.indexOf('match e') + 6, bindings.indexOf('let e=') + 4],
    ];
    for (let index = 0; index < targets.length; index++) {
        const [use, declaration] = targets[index];
        c.send('textDocument/definition', { textDocument: { uri }, position: { line: 0, character: use } }, 150 + index);
        const result = (await c.response(150 + index)).result;
        assert.equal(result.uri, uri); assert.deepEqual(result.range.start, { line: 0, character: declaration });
    }
    c.send('textDocument/hover', { textDocument: { uri }, position: { line: 0, character: bindings.lastIndexOf('print(n)') + 6 } }, 160);
    assert.equal((await c.response(160)).result.contents.value, 'match payload n: Int');
    const draft = 'mod edits{enum E{End}fn pending(x:E)->E{missing();match x{E::End()=>{return x;}}}}fn main(){}';
    c.send('textDocument/didChange', { textDocument: { uri, version: 8 }, contentChanges: [{ text: draft }] });
    assert.ok((await c.diagnostic(8)).params.diagnostics.some(item => item.code === 'E3004'));
    c.send('textDocument/formatting', { textDocument: { uri } }, 180);
    assert.ok((await c.response(180)).result[0].newText.includes('    fn pending'));
    c.send('textDocument/documentSymbol', { textDocument: { uri } }, 181); assert.deepEqual((await c.response(181)).result, []);
    c.send('textDocument/didChange', { textDocument: { uri, version: 9 }, contentChanges: [{ text: invalid }] }); await c.diagnostic(9);
    c.send('textDocument/documentSymbol', { textDocument: { uri } }, 105); assert.deepEqual((await c.response(105)).result, []);
    const unavailable = 'fn main(' + ' '.repeat(4100) + '){}';
    c.send('textDocument/didChange', { textDocument: { uri, version: 10 }, contentChanges: [{ text: unavailable }] }); assert.deepEqual((await c.diagnostic(10)).params.diagnostics, []);
    c.send('textDocument/documentSymbol', { textDocument: { uri } }, 106); assert.deepEqual((await c.response(106)).result, []);
    await c.stop(); c = undefined;
    c = client(); c.send('initialize', { capabilities: {} }, 20); assert.equal((await c.response(20)).result.capabilities.experimental.sovraPipeline, 'legacy');
    c.send('textDocument/didOpen', { textDocument: { uri, languageId: 'sovra', version: 1, text: raw } }); assert.ok((await c.diagnostic(1)).params.diagnostics.length > 0);
    c.send('textDocument/formatting', { textDocument: { uri } }, 21); assert.ok((await c.response(21)).error);
    c.send('textDocument/didChange', { textDocument: { uri, version: 2 }, contentChanges: [{ text: 'fn main(){print(1);}' }] }); assert.deepEqual((await c.diagnostic(2)).params.diagnostics, []);
    c.send('textDocument/formatting', { textDocument: { uri } }, 22); assert.ok((await c.response(22)).result[0].newText.includes('    print(1);'));
    await c.stop(); c = undefined;
    assert.deepEqual((await readdir(tmpdir())).filter(name => name.startsWith('sovra-lsp-format-') && !before.has(name)), []);
    assert.equal(await readFile(file, 'utf8'), disk);
    await assert.rejects(readFile(marker));
    console.log('LSP v2: explicit initialization, standalone workspace isolation, Unicode diagnostics, checked function/scope/payload navigation and unavailable facts, pure formatting/fixed points, task scope, cancellation/stale retirement, cleanup and legacy mode passed.');
} finally { c?.kill(); await rm(scratch, { recursive: true, force: true }); }
