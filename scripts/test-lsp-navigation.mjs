import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, mkdir, writeFile, rm, realpath } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { FrameReader, frame } from '../toolchain/ide/language-server/transport.mjs';
import { decodePackageNavigation } from '../toolchain/ide/language-server/package-navigation.mjs';
const root = await mkdtemp(join(tmpdir(), 'sovra-navigation '));
const app = join(root, 'app'), lib = join(root, 'lib'), other = join(root, 'other');
for (const dir of [app, lib, other]) { await mkdir(dir); await writeFile(join(dir, 'sovra.toml'), `[project]\nname = "${dir === app ? 'app' : 'lib'}"\nentry = "main.svr"\n${dir === app ? '[dependencies.a]\npath = "../lib"\n[dependencies.b]\npath = "../lib"\n[dependencies.c]\npath = "../other"\n' : ''}`); }
const provider = '// title\r\nfn before() { print("λ😀"); } mod api { export fn value(x: Int) -> Int { return x; } } fn main() {}';
const consumer = 'use a::api; use b::api; use c::api; fn main() { print(a::api::value(1)); print(b::api::value(2)); print(c::api::value(3)); }';
const appPath = join(app, 'main.svr'), libPath = join(lib, 'main.svr'), otherPath = join(other, 'main.svr');
await writeFile(appPath, consumer); await writeFile(libPath, provider); await writeFile(otherPath, 'mod api { export fn value(x: Int) -> Int { return x; } fn secret() {} } fn main() {}');
const appUri = pathToFileURL(appPath).href, libUri = pathToFileURL(libPath).href;
// Strict metadata and source verification are independent of the LSP session.
const doc = { uri: appUri, text: 'fn main() {}' }, members = new Map([[appPath, doc]]), entries = new Set([appPath]);
const meta = { schema_version: 1, unavailable: false, files: [{ path: appPath, bytes: Buffer.byteLength(doc.text), sha256: createHash('sha256').update(doc.text).digest('hex') }], declarations: [{ file: 0, name: 'main', signature: 'fn main()', start: 0, end: Buffer.byteLength(doc.text), name_start: 3, name_end: 7 }], references: [] };
assert.equal((await decodePackageNavigation(meta, entries, members)).get(doc).declarations[0].name, 'main');
for (const mutate of [m => { m.files[0].path = libPath; }, m => { m.declarations[0].name_end = 1000; }, m => { m.declarations[0].name = 'wrong'; }, m => { m.declarations[0].signature = 'fn wrong()'; }, m => { m.references.push({ file: 0, start: 0, end: 2, definition: 99 }); }, m => { m.unavailable = true; }, m => { m.files.push(structuredClone(m.files[0])); }]) { const m = structuredClone(meta); mutate(m); await assert.rejects(() => decodePackageNavigation(m, entries, members)); }
const mismatch = structuredClone(meta); mismatch.files[0].sha256 = '0'.repeat(64); assert.equal(await decodePackageNavigation(mismatch, entries, members), undefined);
assert.equal(await decodePackageNavigation({ schema_version: 1, unavailable: true }, entries, members), undefined);
const closedMismatch = structuredClone(meta); closedMismatch.files[0].path = libPath; assert.equal(await decodePackageNavigation(closedMismatch, new Set([libPath]), new Map()), undefined);
const repo = fileURLToPath(new URL('..', import.meta.url));
const args = process.env.SOVRA_LSP_LAUNCHER ? [process.env.SOVRA_LSP_LAUNCHER, 'lsp'] : [join(repo, 'toolchain/ide/language-server/server.mjs'), join(repo, 'target/debug', process.platform === 'win32' ? 'svr.exe' : 'svr')];
const child = spawn(process.execPath, args, { stdio: ['pipe', 'pipe', 'pipe'] });
const messages = [], waiters = []; let stderr = '', sequence = 0;
child.stderr.on('data', b => { stderr += b; });
const closed = new Promise(accept => child.on('close', accept));
const reader = new FrameReader(message => { const i = waiters.findIndex(w => w.test(message)); if (i < 0) messages.push(message); else { const w = waiters.splice(i, 1)[0]; clearTimeout(w.timer); w.accept(message); } });
child.stdout.on('data', b => reader.push(b));
function wait(test) { const i = messages.findIndex(test); if (i >= 0) return Promise.resolve(messages.splice(i, 1)[0]); return new Promise((accept, reject) => { const w = { test, accept }; w.timer = setTimeout(() => { waiters.splice(waiters.indexOf(w), 1); reject(new Error(`Navigation timeout: ${stderr}; ${JSON.stringify(messages)}`)); }, 15000); waiters.push(w); }); }
const send = (method, params, id) => child.stdin.write(frame({ jsonrpc: '2.0', method, params, ...(id === undefined ? {} : { id }) }));
async function request(method, params) { const id = ++sequence; send(method, params, id); const response = await wait(m => m.id === id); assert.equal(response.error, undefined, JSON.stringify(response)); return response.result; }
const point = (text, spelling) => ({ line: 0, character: text.indexOf(spelling) });
const query = (method, text, spelling) => request(`textDocument/${method}`, { textDocument: { uri: appUri }, position: point(text, spelling) });
const change = (uri, text, version) => send('textDocument/didChange', { textDocument: { uri, version }, contentChanges: [{ text }] });
try {
    await request('initialize', { workspaceFolders: [{ uri: pathToFileURL(root).href, name: 'fixture' }] });
    send('textDocument/didOpen', { textDocument: { uri: appUri, version: 1, languageId: 'sovra', text: consumer } });
    const a = await query('definition', consumer, 'a::api::value');
    const b = await query('definition', consumer, 'b::api::value');
    const c = await query('definition', consumer, 'c::api::value');
    assert.ok(a, JSON.stringify(messages)); assert.deepEqual(a, b);
    assert.equal(await realpath(fileURLToPath(a.uri)), await realpath(libPath));
    assert.equal(await realpath(fileURLToPath(c.uri)), await realpath(otherPath));
    assert.equal(a.range.start.line, 1); assert.equal(a.range.start.character, provider.split('\n').at(-1).indexOf('value('));
    assert.match((await query('hover', consumer, 'b::api::value')).contents.value, /fn value\(x: Int\) -> Int/);
    const outline = await request('textDocument/documentSymbol', { textDocument: { uri: appUri } }); assert.deepEqual(outline.map(d => d.name), ['main']);
    const floating = provider.replaceAll('Int', 'Float');
    send('textDocument/didOpen', { textDocument: { uri: libUri, version: 1, languageId: 'sovra', text: floating } });
    assert.match((await query('hover', consumer, 'b::api::value')).contents.value, /Float/);
    assert.equal((await query('definition', consumer, 'b::api::value')).uri, libUri);
    const renamed = floating.replaceAll('value', 'renamed'); change(libUri, renamed, 2);
    assert.equal(await query('definition', consumer, 'b::api::value'), null);
    const updated = consumer.replaceAll('a::api::value', 'a::api::renamed').replaceAll('b::api::value', 'b::api::renamed');
    change(appUri, updated, 2);
    const target = await query('definition', updated, 'b::api::renamed'); assert.equal(target.uri, libUri); assert.equal(target.range.start.character, renamed.split('\n').at(-1).indexOf('renamed('));
    send('textDocument/didClose', { textDocument: { uri: libUri } });
    assert.equal(await query('definition', updated, 'b::api::renamed'), null);
    change(appUri, consumer, 3); assert.deepEqual(await query('definition', consumer, 'b::api::value'), b);
    const privateCall = 'use c::api; fn main() { c::api::secret(); }'; change(appUri, privateCall, 4);
    assert.equal(await query('definition', privateCall, 'c::api::secret'), null);
    await request('shutdown', null); send('exit'); assert.equal(await closed, 0);
    console.log('Checked package navigation: aliases share targets, same-named providers stay distinct, closed/unsaved UTF-16 declarations, hover/outline, rename/close refresh, privacy and strict unavailable metadata passed.');
} finally { for (const w of waiters) clearTimeout(w.timer); child.kill(); await rm(root, { recursive: true, force: true }); }
