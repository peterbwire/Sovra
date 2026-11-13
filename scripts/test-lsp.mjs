import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { resolve } from 'node:path';
import { mkdtemp, access, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { FrameReader, frame, MAX_MESSAGE } from '../toolchain/ide/language-server/transport.mjs';
import { decodeEvidence } from '../toolchain/ide/language-server/symbols.mjs';

// Malformed compiler metadata must fail closed before becoming hover/navigation.
const metadataSource = '// λ😀\r\nfn main() { let x = 1; print(x); }';
const byteAt = index => Buffer.byteLength(metadataSource.slice(0, index));
const metadataBytes = Buffer.from(metadataSource);
const localStart = byteAt(metadataSource.indexOf('let x'));
const localName = byteAt(metadataSource.indexOf('let x') + 4);
const call = byteAt(metadataSource.indexOf('print(x)'));
const metadata = {
    declarations: [{ name: 'main', signature: 'fn main()', start: byteAt(metadataSource.indexOf('fn main')), end: metadataBytes.length, name_start: byteAt(metadataSource.indexOf('main')), name_end: byteAt(metadataSource.indexOf('main') + 4) }],
    references: [{ start: call, end: call + 5, definition: null, signature: 'fn std::print(Any) -> Unit' }],
    locals: [{ name: 'x', kind: 'local', signature: 'let x: Int (inferred)', inferred_type: 'Int', start: localStart, end: byteAt(metadataSource.indexOf('1;') + 2), name_start: localName, name_end: localName + 1 }],
    local_references: [{ start: call + 6, end: call + 7, definition: 0 }], truncated: false,
};
assert.equal(decodeEvidence(metadata, metadataBytes).locals[0].selectionRange.start.line, 1);
assert.equal(decodeEvidence(undefined, metadataBytes), undefined);
assert.equal(decodeEvidence({ declarations: [], references: [], locals: [], local_references: [], truncated: true, unavailable: true }, metadataBytes), undefined);
const legacyMetadata = structuredClone(metadata);
delete legacyMetadata.locals[0].inferred_type;
legacyMetadata.locals[0].signature = 'let x (inferred type unavailable)';
assert.ok(decodeEvidence(legacyMetadata, metadataBytes));
const deepMetadata = structuredClone(metadata);
deepMetadata.locals[0].inferred_type = 'Array<'.repeat(129) + 'Int' + '>'.repeat(129);
deepMetadata.locals[0].signature = `let x: ${deepMetadata.locals[0].inferred_type} (inferred)`;
assert.doesNotThrow(() => decodeEvidence(deepMetadata, metadataBytes));
for (const mutate of [
    item => { item.declarations = {}; },
    item => { item.locals[0] = null; },
    item => { item.locals = null; },
    item => { item.locals[0].signature = {}; },
    item => { item.locals[0].inferred_type = 42; },
    item => { item.locals[0].inferred_type = 'Array<Int'; },
    item => { item.locals[0].inferred_type = null; },
    item => { item.locals[0].kind = 'parameter'; },
    item => { item.locals[0].name_end = metadataBytes.length + 1; },
    item => { item.locals[0].signature = '\ud800'; },
    item => { item.locals[0].signature = 'x'.repeat(4097); },
    item => { item.local_references[0].definition = 1; },
    item => { item.references[0].definition = -1; },
    item => { item.references[0].start = byteAt(metadataSource.indexOf('λ')) + 1; item.references[0].end = item.references[0].start + 1; },
    item => { item.locals = Array(8193).fill(metadata.locals[0]); },
    item => { item.padding = 'x'.repeat(512 * 1024); },
    item => { item.unavailable = true; },
    item => { item.unavailable = 'true'; },
]) {
    const item = structuredClone(metadata); mutate(item);
    assert.throws(() => decodeEvidence(item, metadataBytes));
}

// The transport accepts arbitrary byte boundaries, including inside Unicode.
const received = [];
const parser = new FrameReader(value => received.push(value));
const encoded = frame({ text: 'λ😀' });
for (const byte of encoded) parser.push(Buffer.from([byte]));
assert.deepEqual(received, [{ text: 'λ😀' }]);
parser.push(Buffer.concat([frame({ one: 1 }), frame({ two: 2 })]));
assert.equal(received.length, 3);
for (const header of ['Content-Length: -1\r\n\r\n', 'Content-Length: 1\r\nContent-Length: 1\r\n\r\n', `Content-Length: ${MAX_MESSAGE + 1}\r\n\r\n`, 'x'.repeat(4097)]) {
    assert.throws(() => new FrameReader(() => {}).push(Buffer.from(header)));
}
const serverArgs = process.env.SOVRA_LSP_LAUNCHER ? [process.env.SOVRA_LSP_LAUNCHER, 'lsp'] : ['toolchain/ide/language-server/server.mjs', resolve('target/debug', process.platform === 'win32' ? 'svr.exe' : 'svr')];
const worker = spawn(process.execPath, serverArgs, { stdio: ['pipe', 'pipe', 'pipe'] });
const messages = [], waiters = [];
let stderr = '';
worker.stderr.on('data', data => { stderr += data; });
const closed = new Promise(accept => worker.once('close', code => accept(code)));
const reader = new FrameReader(message => {
    const index = waiters.findIndex(value => value.predicate(message));
    if (index >= 0) { const [waiter] = waiters.splice(index, 1); clearTimeout(waiter.timer); waiter.accept(message); }
    else messages.push(message);
});
worker.stdout.on('data', data => reader.push(data));
const wait = predicate => new Promise((accept, reject) => {
    const index = messages.findIndex(predicate);
    if (index >= 0) return accept(messages.splice(index, 1)[0]);
    const value = { predicate, accept };
    value.timer = setTimeout(() => { waiters.splice(waiters.indexOf(value), 1); reject(new Error(`LSP response timed out; pending messages: ${JSON.stringify(messages)}; stderr: ${stderr}`)); }, 15000);
    waiters.push(value);
});
const send = (method, params, id) => worker.stdin.write(frame({ jsonrpc: '2.0', method, params, ...(id === undefined ? {} : { id }) }));
const response = id => wait(message => message.id === id);
const uri = 'file:///test/unwritten-source.svr';
const diagnostic = version => wait(message => message.method === 'textDocument/publishDiagnostics' && message.params.uri === uri && message.params.version === version);
const scratch = await mkdtemp(resolve(tmpdir(), 'sovra-lsp-proof-'));
try {
    send('textDocument/completion', {}, 0);
    assert.equal((await response(0)).error.code, -32002);
    send('initialize', { capabilities: {} }, 1);
    const initialized = (await response(1)).result;
    assert.equal(initialized.capabilities.positionEncoding, 'utf-16');
    assert.equal(initialized.capabilities.textDocumentSync.change, 1);
    assert.equal(initialized.capabilities.definitionProvider, true);
    assert.equal(initialized.capabilities.hoverProvider, true);
    send('initialized', {});
    const text = 'fn main() { print("λ😀"); print(missing) }';
    send('textDocument/didOpen', { textDocument: { uri, languageId: 'sovra', version: 1, text } });
    const first = (await diagnostic(1)).params.diagnostics[0];
    assert.equal(first.source, 'sovra');
    assert.match(first.message, /missing/);
    assert.deepEqual(first.range, { start: { line: 0, character: text.indexOf('missing') }, end: { line: 0, character: text.indexOf('missing') + 7 } });
    send('textDocument/completion', { textDocument: { uri }, position: { line: 0, character: 0 } }, 2);
    assert.ok((await response(2)).result.some(value => value.label === 'fn'));
    send('textDocument/didChange', { textDocument: { uri, version: 2 }, contentChanges: [{ text: 'fn main() {}' }] });
    assert.deepEqual((await diagnostic(2)).params.diagnostics, []);
    // Ignore stale document versions and coalesce newer full-document changes.
    send('textDocument/didChange', { textDocument: { uri, version: 1 }, contentChanges: [{ text }] });
    for (let version = 3; version <= 12; version++) send('textDocument/didChange', { textDocument: { uri, version }, contentChanges: [{ text: version === 12 ? 'fn main() {}' : text }] });
    assert.deepEqual((await diagnostic(12)).params.diagnostics, []);
    assert.equal(messages.some(message => message.params?.version > 2 && message.params?.version < 12), false);
    send('textDocument/didChange', { textDocument: { uri, version: 13 }, contentChanges: [{ text: 'x'.repeat(512 * 1024 + 1) }] });
    assert.equal((await diagnostic(13)).params.diagnostics[0].code, 'SVR_LSP');
    send('textDocument/didChange', { textDocument: { uri, version: 14 }, contentChanges: [{ text: 'fn main() {\r\n    print("λ😀"); print(missing)\r\n}' }] });
    const multiline = (await diagnostic(14)).params.diagnostics[0];
    assert.equal(multiline.range.start.line, 1);
    assert.equal(multiline.range.start.character, '    print("λ😀"); print('.length);
    send('textDocument/didChange', { textDocument: { uri, version: 15 }, contentChanges: [{ text: '\ud800' }] });
    assert.equal((await diagnostic(15)).params.diagnostics[0].code, 'SVR_LSP');
    const marker = resolve(scratch, 'must-not-execute.txt');
    send('textDocument/didChange', { textDocument: { uri, version: 16 }, contentChanges: [{ text: `fn main() { std::write_text(${JSON.stringify(marker)}, "executed") }` }] });
    assert.deepEqual((await diagnostic(16)).params.diagnostics, []);
    await assert.rejects(access(marker), { code: 'ENOENT' });
    worker.stdin.write(Buffer.concat([Buffer.from('Content-Length: 1\r\n\r\n'), Buffer.from([0xff])]));
    assert.equal((await wait(message => message.error?.code === -32700)).id, null);
    send('textDocument/references', { textDocument: { uri } }, 3);
    assert.equal((await response(3)).error.code, -32601);
    const symbolsText = 'fn add(x: Int) -> Int { return x + 1 }\r\nfn main() { print("λ😀"); print(add(2)) }';
    send('textDocument/didChange', { textDocument: { uri, version: 17 }, contentChanges: [{ text: symbolsText }] });
    send('textDocument/documentSymbol', { textDocument: { uri } }, 36);
    assert.deepEqual((await response(36)).result.map(value => value.name), ['add', 'main']);
    assert.deepEqual((await diagnostic(17)).params.diagnostics, []);
    const point = { line: 1, character: symbolsText.split('\r\n')[1].indexOf('add(2)') + 1 };
    send('textDocument/hover', { textDocument: { uri }, position: point }, 30);
    assert.match((await response(30)).result.contents.value, /fn add\(x: Int\) -> Int/);
    send('textDocument/definition', { textDocument: { uri }, position: point }, 31);
    assert.deepEqual((await response(31)).result, { uri, range: { start: { line: 0, character: 3 }, end: { line: 0, character: 6 } } });
    send('textDocument/documentSymbol', { textDocument: { uri } }, 32);
    assert.deepEqual((await response(32)).result.map(value => value.name), ['add', 'main']);
    send('textDocument/hover', { textDocument: { uri }, position: { line: 1, character: 14 } }, 33);
    assert.match((await response(33)).result.contents.value, /std::print/);
    send('textDocument/definition', { textDocument: { uri }, position: { line: 1, character: 14 } }, 34);
    assert.equal((await response(34)).result, null);
    send('textDocument/didChange', { textDocument: { uri, version: 18 }, contentChanges: [{ text }] });
    send('textDocument/hover', { textDocument: { uri }, position: point }, 37);
    send('$/cancelRequest', { id: 37 });
    assert.equal((await response(37)).error.code, -32800);
    send('textDocument/definition', { textDocument: { uri }, position: point }, 35);
    assert.equal((await response(35)).result, null);
    await diagnostic(18);
    send('textDocument/didChange', { textDocument: { uri, version: 19 }, contentChanges: [{ text: symbolsText }] });
    send('textDocument/documentSymbol', { textDocument: { uri } }, 38);
    send('textDocument/didChange', { textDocument: { uri, version: 20 }, contentChanges: [{ text: symbolsText }] });
    assert.equal((await response(38)).error.code, -32801);
    await diagnostic(20);
    const localText = 'fn keep(value: Int) -> Int {\r\n let x: Int = value;\r\n if true { let x = x + 1; print(x); }\r\n return x;\r\n}';
    send('textDocument/didChange', { textDocument: { uri, version: 21 }, contentChanges: [{ text: localText }] });
    const localLines = localText.split('\r\n');
    send('textDocument/hover', { textDocument: { uri }, position: { line: 1, character: localLines[1].indexOf('value') + 1 } }, 40);
    assert.match((await response(40)).result.contents.value, /parameter value: Int/);
    await diagnostic(21);
    send('textDocument/definition', { textDocument: { uri }, position: { line: 1, character: localLines[1].indexOf('value') + 1 } }, 41);
    assert.equal((await response(41)).result.range.start.character, 'fn keep('.length);
    send('textDocument/definition', { textDocument: { uri }, position: { line: 2, character: localLines[2].indexOf('print(x)') + 6 } }, 42);
    assert.deepEqual((await response(42)).result.range.start, { line: 2, character: localLines[2].indexOf('let x') + 4 });
    send('textDocument/definition', { textDocument: { uri }, position: { line: 2, character: localLines[2].indexOf('x + 1') } }, 43);
    assert.deepEqual((await response(43)).result.range.start, { line: 1, character: localLines[1].indexOf('x:') });
    send('textDocument/definition', { textDocument: { uri }, position: { line: 3, character: localLines[3].indexOf('x;') } }, 44);
    assert.deepEqual((await response(44)).result.range.start, { line: 1, character: localLines[1].indexOf('x:') });
    send('textDocument/hover', { textDocument: { uri }, position: { line: 2, character: localLines[2].indexOf('print(x)') + 6 } }, 45);
    assert.equal((await response(45)).result.contents.value, 'let x: Int (inferred)');
    const unicodeText = '// 😀 preface\r\nfn main() {\r\n\tprint("e\u0301😀\\n"); std::len(42);\r\n\tprint(missing);\r\n}\r\n';
    send('textDocument/didChange', { textDocument: { uri, version: 22 }, contentChanges: [{ text: unicodeText }] });
    const unicodeErrors = (await diagnostic(22)).params.diagnostics;
    assert.equal(unicodeErrors.length, 2);
    for (const [code, spelling, line] of [['E3007', '42', 2], ['E3001', 'missing', 3]]) {
        const prefix = unicodeText.split('\r\n')[line].split(spelling)[0];
        const item = unicodeErrors.find(value => value.code === code);
        assert.deepEqual(item.range, { start: { line, character: prefix.length }, end: { line, character: prefix.length + spelling.length } });
        if (code === 'E3007') assert.notEqual(prefix.length, Array.from(prefix).length);
    }
    const fixedUnicode = unicodeText.replace('std::len(42)', 'std::len("valid")').replace('print(missing)', 'print(42)');
    send('textDocument/didChange', { textDocument: { uri, version: 23 }, contentChanges: [{ text: fixedUnicode }] });
    assert.deepEqual((await diagnostic(23)).params.diagnostics, []);
    send('textDocument/didClose', { textDocument: { uri } });
    assert.deepEqual((await diagnostic(23)).params.diagnostics, []);
    const deepUri = 'untitled:deep-inference';
    const deepSource = 'fn main() { let v0 = 1; ' + Array.from({ length: 129 }, (_, index) => `let v${index + 1} = [v${index}];`).join('') + 'print(v129); }';
    send('textDocument/didOpen', { textDocument: { uri: deepUri, languageId: 'sovra', version: 1, text: deepSource } });
    send('textDocument/hover', { textDocument: { uri: deepUri }, position: { line: 0, character: deepSource.indexOf('print(v129)') + 6 } }, 65);
    assert.equal((await response(65)).result.contents.value, `let v129: ${'Array<'.repeat(129)}Int${'>'.repeat(129)} (inferred)`);
    const deepDiagnostic = () => wait(message => message.method === 'textDocument/publishDiagnostics' && message.params.uri === deepUri);
    assert.deepEqual((await deepDiagnostic()).params.diagnostics, []);
    send('textDocument/didClose', { textDocument: { uri: deepUri } });
    assert.deepEqual((await deepDiagnostic()).params.diagnostics, []);
    // Qualified nominal identities can exhaust metadata within a small valid buffer.
    const unavailableUri = 'untitled:unavailable-evidence';
    const unavailableSource = `mod ${'m'.repeat(3500)} { ` + Array.from({ length: 160 }, (_, index) => `struct R${index} { x: Int }`).join('') + 'fn helper() {' + Array.from({ length: 160 }, (_, index) => `let p${index} = R${index} { x: 1 };`).join('') + '} }';
    send('textDocument/didOpen', { textDocument: { uri: unavailableUri, languageId: 'sovra', version: 1, text: unavailableSource } });
    send('textDocument/hover', { textDocument: { uri: unavailableUri }, position: { line: 0, character: unavailableSource.indexOf('let p0') + 4 } }, 66);
    assert.equal((await response(66)).result, null);
    const unavailableDiagnostic = () => wait(message => message.method === 'textDocument/publishDiagnostics' && message.params.uri === unavailableUri);
    assert.deepEqual((await unavailableDiagnostic()).params.diagnostics, []);
    send('textDocument/documentSymbol', { textDocument: { uri: unavailableUri } }, 67);
    assert.deepEqual((await response(67)).result, []);
    send('textDocument/didClose', { textDocument: { uri: unavailableUri } });
    assert.deepEqual((await unavailableDiagnostic()).params.diagnostics, []);
    // Queue/cancel/close in one frame batch: no timing sleeps or compiler-speed assumptions.
    const lifecycleUri = 'untitled:lifecycle';
    const batch = entries => worker.stdin.write(Buffer.concat(entries.map(([method, params, id]) =>
        frame({ jsonrpc: '2.0', method, params, ...(id === undefined ? {} : { id }) }))));
    const lifecycleDiagnostic = version => wait(message => message.method === 'textDocument/publishDiagnostics'
        && message.params.uri === lifecycleUri && message.params.version === version);
    batch([
        ['textDocument/didOpen', { textDocument: { uri: lifecycleUri, languageId: 'sovra', version: 90, text: symbolsText } }],
        ['textDocument/hover', { textDocument: { uri: lifecycleUri }, position: point }, 50],
        ['textDocument/documentSymbol', { textDocument: { uri: lifecycleUri } }, 51],
        ['$/cancelRequest', { id: 50 }],
    ]);
    assert.equal((await response(50)).error.code, -32800);
    assert.deepEqual((await response(51)).result.map(value => value.name), ['add', 'main']);
    assert.deepEqual((await lifecycleDiagnostic(90)).params.diagnostics, []);
    batch([
        ['textDocument/didChange', { textDocument: { uri: lifecycleUri, version: 91 }, contentChanges: [{ text }] }],
        ['textDocument/documentSymbol', { textDocument: { uri: lifecycleUri } }, 52],
        ['textDocument/didClose', { textDocument: { uri: lifecycleUri } }],
        ['textDocument/hover', { textDocument: { uri: lifecycleUri }, position: point }, 53],
        ['textDocument/didOpen', { textDocument: { uri: lifecycleUri, languageId: 'sovra', version: 1, text: 'fn fresh() {}' } }],
        ['textDocument/documentSymbol', { textDocument: { uri: lifecycleUri } }, 54],
    ]);
    assert.equal((await response(52)).error.code, -32801);
    assert.deepEqual((await lifecycleDiagnostic(91)).params.diagnostics, []);
    assert.equal((await response(53)).result, null);
    assert.deepEqual((await response(54)).result.map(value => value.name), ['fresh']);
    assert.deepEqual((await lifecycleDiagnostic(1)).params.diagnostics, []);
    // Inferred hover follows the current checked generation and handles holes.
    const hoverSource = 'fn fresh() { let value = 1; let nested = [[1]]; let empty = []; print(value); }';
    send('textDocument/didChange', { textDocument: { uri: lifecycleUri, version: 2 }, contentChanges: [{ text: hoverSource }] });
    send('textDocument/hover', { textDocument: { uri: lifecycleUri }, position: { line: 0, character: hoverSource.indexOf('print(value)') + 6 } }, 60);
    assert.equal((await response(60)).result.contents.value, 'let value: Int (inferred)');
    await lifecycleDiagnostic(2);
    send('textDocument/hover', { textDocument: { uri: lifecycleUri }, position: { line: 0, character: hoverSource.indexOf('nested') } }, 61);
    assert.equal((await response(61)).result.contents.value, 'let nested: Array<Array<Int>> (inferred)');
    send('textDocument/hover', { textDocument: { uri: lifecycleUri }, position: { line: 0, character: hoverSource.indexOf('empty') } }, 62);
    assert.equal((await response(62)).result.contents.value, 'let empty (inferred type unavailable)');
    const changedHover = hoverSource.replace('value = 1', 'value = "λ😀"');
    send('textDocument/didChange', { textDocument: { uri: lifecycleUri, version: 3 }, contentChanges: [{ text: changedHover }] });
    send('textDocument/hover', { textDocument: { uri: lifecycleUri }, position: { line: 0, character: changedHover.indexOf('print(value)') + 6 } }, 63);
    assert.equal((await response(63)).result.contents.value, 'let value: String (inferred)');
    await lifecycleDiagnostic(3);
    const failedHover = changedHover.replace('print(value)', 'print(missing)');
    send('textDocument/didChange', { textDocument: { uri: lifecycleUri, version: 4 }, contentChanges: [{ text: failedHover }] });
    send('textDocument/hover', { textDocument: { uri: lifecycleUri }, position: { line: 0, character: failedHover.indexOf('value') } }, 64);
    assert.equal((await response(64)).result, null);
    assert.ok((await lifecycleDiagnostic(4)).params.diagnostics.length > 0);
    // A rejected replacement must discard previously checked symbols immediately.
    send('textDocument/didChange', { textDocument: { uri: lifecycleUri, version: 5 }, contentChanges: [{ text: '\ud800' }] });
    assert.equal((await lifecycleDiagnostic(5)).params.diagnostics[0].code, 'SVR_LSP');
    send('textDocument/documentSymbol', { textDocument: { uri: lifecycleUri } }, 55);
    assert.deepEqual((await response(55)).result, []);
    send('textDocument/hover', { textDocument: { uri: lifecycleUri }, position: { line: 0, character: 4 } }, 56);
    assert.equal((await response(56)).result, null);
    send('textDocument/didClose', { textDocument: { uri: lifecycleUri } });
    assert.deepEqual((await lifecycleDiagnostic(5)).params.diagnostics, []);
    for (let index = 0; index < 32; index++) send('textDocument/didOpen', { textDocument: { uri: `untitled:cap-${index}`, languageId: 'sovra', version: 1, text: '' } });
    send('textDocument/didOpen', { textDocument: { uri: 'untitled:over-cap', languageId: 'sovra', version: 1, text: '' } });
    const capped = await wait(message => message.params?.uri === 'untitled:over-cap');
    assert.match(capped.params.diagnostics[0].message, /32 open/);
    for (let index = 0; index < 32; index++) send('textDocument/didClose', { textDocument: { uri: `untitled:cap-${index}` } });
    for (let index = 0; index < 8; index++) send('textDocument/didOpen', { textDocument: { uri: `untitled:bytes-${index}`, languageId: 'sovra', version: 1, text: '//' + 'x'.repeat(512 * 1024 - 2) } });
    send('textDocument/didOpen', { textDocument: { uri: 'untitled:over-bytes', languageId: 'sovra', version: 1, text: 'x' } });
    const bytesCap = await wait(message => message.params?.uri === 'untitled:over-bytes');
    assert.equal(bytesCap.params.diagnostics[0].code, 'SVR_LSP');
    for (let index = 0; index < 8; index++) send('textDocument/didClose', { textDocument: { uri: `untitled:bytes-${index}` } });
    send('textDocument/didClose', { textDocument: { uri: 'untitled:over-bytes' } });
    send('shutdown', null, 4);
    assert.equal((await response(4)).result, null);
    send('textDocument/completion', {}, 5);
    assert.equal((await response(5)).error.code, -32600);
    send('exit');
    assert.equal(await closed, 0);
    assert.equal(stderr, '');
    assert.equal(messages.some(message => [50, 51, 52, 53, 54, 55, 56].includes(message.id)), false, 'Lifecycle requests must receive exactly one response');
    assert.equal(messages.some(message => message.params?.uri === lifecycleUri), false, 'Closed/replaced documents must not publish stale diagnostics');
    console.log('LSP: real diagnostics, UTF-16/CRLF, function hover/definition/outline, pending/cancelled/stale queries, framing, lifecycle, caps and shutdown passed.');
} finally {
    worker.kill();
    for (const waiter of waiters) clearTimeout(waiter.timer);
    await rm(scratch, { recursive: true, force: true });
}
