import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, mkdir, writeFile, readFile, rm, realpath } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve, win32, posix } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { FrameReader, frame } from '../toolchain/ide/language-server/transport.mjs';
import { decodeContext, overlayFrame, packageDiagnostics, containsPath } from '../toolchain/ide/language-server/packages.mjs';

assert.equal(containsPath('/repo', '/repo2/file', posix), false);
assert.equal(containsPath('/repo', '/repo/../outside/file', posix), false);
const drive = 'C:' + String.fromCharCode(92), otherDrive = 'D:' + String.fromCharCode(92);
assert.equal(containsPath(win32.join(drive, 'repo'), win32.join(drive, 'repo2', 'file'), win32), false);
assert.equal(containsPath(win32.join(drive, 'repo'), win32.join(drive, 'repo', '..', 'outside', 'file'), win32), false);
assert.equal(containsPath(win32.join(drive, 'repo'), win32.join(otherDrive, 'file'), win32), false);
assert.equal(containsPath(win32.join(drive, 'repo'), win32.join(drive, 'repo', 'file'), win32), true);
const root = await mkdtemp(join(tmpdir(), 'sovra-lsp-packages '));
const app = join(root, 'app'), lib = join(root, 'lib');
await mkdir(app); await mkdir(lib);
await writeFile(join(app, 'sovra.toml'), '[project]\nname = "app"\nentry = "main.svr"\n[dependencies.a]\npath = "../lib"\n[dependencies.b]\npath = "../lib"\n');
await writeFile(join(lib, 'sovra.toml'), '[project]\nname = "lib"\nentry = "main.svr"\n');
const provider = 'mod api { export fn value(x: Int) -> Int { return x; } } fn main() {}';
const consumer = type => `use a::api; use b::api; fn main() { let x: ${type} = b::api::value(1); print(x); }`;
await writeFile(join(app, 'main.svr'), consumer('Int'));
await writeFile(join(lib, 'main.svr'), provider);
const appPath = join(app, 'main.svr'), libPath = join(lib, 'main.svr');
const appUri = pathToFileURL(appPath).href, libUri = pathToFileURL(libPath).href;
// Decoder must reject contradictory, oversized, unrelated or stale provenance.
const doc = { uri: appUri, text: 'fn main() {}' }, members = new Map([[appPath, doc]]), entries = new Set([appPath]);
const context = { schema_version: 1, target: app, kind: 'project', scope: 'package_graph', source_check: false, success: true, diagnostics: [], package_context: { schema_version: 1, root: app, entries: [appPath] } };
assert.deepEqual(decodeContext(context, app), entries);
for (const mutate of [c => { c.source_check = true; }, c => { c.package_context.entries.push(appPath); }, c => { c.package_context.entries = ['relative.svr']; }, c => { c.target = lib; }]) { const c = structuredClone(context); mutate(c); assert.throws(() => decodeContext(c, app)); }
assert.equal(overlayFrame(9, members).subarray(0, 8).toString(), 'SVROVL1\n');
assert.throws(() => overlayFrame(9, new Map([[appPath, { rejected: true }]])));
const valid = { schema_version: 1, target: app, kind: 'project', success: false, overlay_request: { schema_version: 1, request_id: 9 }, diagnostics: [{ severity: 'error', code: 'E3004', message: 'bad', location: null }] };
assert.equal(packageDiagnostics(valid, app, 9, entries, members, doc, () => {}).get(doc)[0].range.start.line, 0);
for (const mutate of [r => { r.success = true; }, r => { r.overlay_request.request_id = 10; }, r => { r.diagnostics[0].location = { file: libPath, start: 0, end: 1 }; }, r => { r.diagnostics[0].message = 42; }]) { const r = structuredClone(valid); mutate(r); assert.throws(() => packageDiagnostics(r, app, 9, entries, members, doc, () => {})); }
const repo = fileURLToPath(new URL('..', import.meta.url));
const args = process.env.SOVRA_LSP_LAUNCHER ? [process.env.SOVRA_LSP_LAUNCHER, 'lsp'] : [join(repo, 'toolchain/ide/language-server/server.mjs'), resolve(repo, 'target/debug', process.platform === 'win32' ? 'svr.exe' : 'svr')];
const worker = spawn(process.execPath, args, { stdio: ['pipe', 'pipe', 'pipe'] });
const messages = [], waiters = [], history = []; let stderr = '';
worker.stderr.on('data', b => { stderr += b; });
const closed = new Promise(accept => worker.on('close', accept));
worker.stdout.on('data', b => reader.push(b));
const reader = new FrameReader(message => {
    history.push(message);
    const i = waiters.findIndex(w => w.predicate(message));
    if (i < 0) messages.push(message); else { const w = waiters.splice(i, 1)[0]; clearTimeout(w.timer); w.accept(message); }
});
function wait(predicate) {
    const i = messages.findIndex(predicate); if (i >= 0) return Promise.resolve(messages.splice(i, 1)[0]);
    return new Promise((accept, reject) => { const w = { predicate, accept }; w.timer = setTimeout(() => { waiters.splice(waiters.indexOf(w), 1); reject(new Error(`Package LSP timeout for ${predicate.toString()}: ${stderr}; messages=${JSON.stringify(messages)}`)); }, 15000); waiters.push(w); });
}
const send = (method, params, id) => worker.stdin.write(frame({ jsonrpc: '2.0', method, params, ...(id === undefined ? {} : { id }) }));
const response = id => wait(m => m.id === id);
const diagnostic = (uri, code) => wait(m => m.method === 'textDocument/publishDiagnostics' && m.params.uri === uri && m.params.diagnostics.some(d => d.code === code));
const open = (uri, text, version = 1) => send('textDocument/didOpen', { textDocument: { uri, languageId: 'sovra', text, version } });
const change = (uri, text, version) => send('textDocument/didChange', { textDocument: { uri, version }, contentChanges: [{ text }] });
const latest = uri => history.filter(m => m.method === 'textDocument/publishDiagnostics' && m.params.uri === uri).at(-1)?.params.diagnostics;
async function checked(id) { send('textDocument/hover', { textDocument: { uri: appUri }, position: { line: 0, character: 0 } }, id); assert.equal((await response(id)).result, null); }
try {
    send('initialize', { workspaceFolders: [{ uri: pathToFileURL(root).href, name: 'fixture' }] }, 1); await response(1);
    open(appUri, consumer('Int')); await checked(2); assert.deepEqual(latest(appUri), []);
    send('sovra/taskTarget', { textDocument: { uri: appUri, version: 1 } }, 80);
    const plan = (await response(80)).result; assert.equal(plan.package, true); assert.equal(await realpath(plan.target), await realpath(app)); assert.equal(plan.cwd, plan.target); assert.equal(plan.sources.length, 2); assert.equal(plan.manifests.length, 2);
    send('sovra/taskTarget', { textDocument: { uri: appUri, version: 0 } }, 81); assert.equal((await response(81)).error.code, -32801);
    send('sovra/taskTarget', { textDocument: { uri: appUri, version: 1 } }, 82); send('$/cancelRequest', { id: 82 }); assert.equal((await response(82)).error.code, -32800);
    const floatProvider = provider.replaceAll('Int', 'Float');
    send('sovra/taskTarget', { textDocument: { uri: appUri, version: 1 } }, 83);
    open(libUri, floatProvider);
    assert.equal((await response(83)).error.code, -32801);
    const dependent = await diagnostic(appUri, 'E3002'); assert.equal(dependent.params.version, 1);
    change(appUri, consumer('Float'), 2); await checked(3); assert.deepEqual(latest(appUri), []);
    const broken = '// λ😀\r\nmod api { export fn value(x: Float) -> Float { missing(); return x; } } fn main() {}';
    change(libUri, broken, 2);
    const owned = await diagnostic(libUri, 'E3004'); assert.equal(owned.params.version, 2); assert.equal(owned.params.diagnostics[0].range.start.line, 1);
    assert.equal(owned.params.diagnostics[0].range.end.character - owned.params.diagnostics[0].range.start.character, 7);
    send('textDocument/didClose', { textDocument: { uri: libUri } }); await checked(4); assert.deepEqual(latest(appUri), []); assert.deepEqual(latest(libUri), []);
    open(libUri, 'x'.repeat(512 * 1024 + 1));
    const blocked = await diagnostic(appUri, 'SVR_LSP'); assert.match(blocked.params.diagnostics[0].message, /rejected/);
    send('textDocument/didClose', { textDocument: { uri: libUri } }); await checked(5); assert.deepEqual(latest(appUri), []);
    change(appUri, 'fn main() { missing(); }', 3);
    send('textDocument/hover', { textDocument: { uri: appUri }, position: { line: 0, character: 0 } }, 6);
    change(appUri, consumer('Float'), 4); assert.equal((await response(6)).error.code, -32801); await checked(7); assert.deepEqual(latest(appUri), []);
    // A standalone entry package keeps optional local hover after graph success.
    const solo = join(root, 'solo'); await mkdir(solo);
    await writeFile(join(solo, 'sovra.toml'), '[project]\nname = "solo"\nentry = "main.svr"\n');
    const soloText = 'fn main() { let x = 1; print(x); }';
    const soloPath = join(solo, 'main.svr'); await writeFile(soloPath, soloText);
    const soloUri = pathToFileURL(soloPath).href; open(soloUri, soloText);
    send('textDocument/hover', { textDocument: { uri: soloUri }, position: { line: 0, character: soloText.indexOf('x);') } }, 50);
    assert.match((await response(50)).result.contents.value, /Int/);
    send('textDocument/didClose', { textDocument: { uri: soloUri } }); await checked(51);
    // Closed dependencies are never assigned fabricated ranges in consumer text.
    await writeFile(libPath, 'fn main() { missing(); }');
    send('workspace/didChangeWatchedFiles', { changes: [{ uri: libUri, type: 2 }] });
    const diskError = await diagnostic(appUri, 'SVR_LSP'); assert.match(diskError.params.diagnostics[0].message, /closed source/);
    assert.deepEqual(diskError.params.diagnostics[0].range, { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } });
    await writeFile(libPath, provider); send('workspace/didChangeWatchedFiles', { changes: [{ uri: libUri, type: 2 }] }); await checked(8); assert.deepEqual(latest(appUri), []);
    assert.equal(await readFile(appPath, 'utf8'), consumer('Int')); assert.equal(await readFile(libPath, 'utf8'), provider);
    send('shutdown', null, 9); await response(9); send('exit'); assert.equal(await closed, 0);
    console.log('Package LSP: unsaved graph checks, dependency edits/close/rejection, UTF-16 ownership, generation cancellation, disk invalidation and no source writes passed. Imported navigation is verified separately.');
} finally { for (const w of waiters) clearTimeout(w.timer); worker.kill(); await rm(root, { recursive: true, force: true }); }
