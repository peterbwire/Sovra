// Shared view admission and HTML rendering: no application-specific rules.
import assert from 'node:assert/strict';
import { decodeUiView, renderUiView } from '../toolchain/ui/shared/view.mjs';
import { decodeUiView as compatibilityDecode } from '../dogfood/01-task-manager/gui/ui-view.mjs';
import { createSovraProgram } from '../toolchain/ui/shared/program.mjs';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const token = 'a'.repeat(64);
let checks = 0;
const rejected = operation => { assert.throws(operation); checks++; };
const decode = (text, version = 1) => { checks++; return decodeUiView(text, { version }); };
const flat = 'SVR-UI-1\nheading|Hello <Sovra>\ninput|save|Name | value|Save\nbutton|reset|Reset\n';
assert.deepEqual(decode(flat), compatibilityDecode(flat));
assert.equal(renderUiView(decode(flat), token),
    `<h1>Hello &lt;Sovra&gt;</h1><form method="post" action="/action"><input type="hidden" name="token" value="${token}"><input type="hidden" name="action" value="save"><input name="value" aria-label="Name | value" required><button>Save</button></form><form method="post" action="/action"><input type="hidden" name="token" value="${token}"><input type="hidden" name="action" value="reset"><button>Reset</button></form>`);
const grouped = 'SVR-UI-2\nheading|Shared UI\ngroup|card|vertical|card\ntext|Literal <img src=x onerror=alert(1)>\ngroup|row|horizontal|plain\ninput|save|Name|Save\nbutton|reset|Reset\nend|row\nend|card\n';
const nodes = decode(grouped, 2);
const html = renderUiView(nodes, token, '/ui-action', 'Saved <data>');
assert.match(html, /data-sovra-group="row" class="sovra-ui-group sovra-ui-horizontal/);
assert.match(html, /role="status">Saved &lt;data&gt;/);
assert.ok(!html.includes('<img'));
rejected(() => decodeUiView(grouped));
rejected(() => decodeUiView(flat, { version: 2 }));
rejected(() => decodeUiView(flat, { version: 3 }));
for (const character of [...Array(32).keys(), ...Array.from({ length: 33 }, (_, index) => index + 127)]) {
    rejected(() => decodeUiView(`SVR-UI-1\nheading|bad${String.fromCodePoint(character)}text\n`));
}
for (const text of ['', 'SVR-UI-1\n', 'SVR-UI-1\ntext|No heading\n',
    'SVR-UI-1\nheading|Hello\nbutton|save|One\ninput|save|Name|Save\n',
    'SVR-UI-1\nheading|\ud800\n', 'SVR-UI-1\nheading|\udc00\n',
    '\ufeffSVR-UI-1\nheading|BOM\n']) rejected(() => decodeUiView(text));
for (const payload of ['x'.repeat(65518), 'λ'.repeat(32759), '😀'.repeat(16379) + 'λ']) {
    const frame = `SVR-UI-1\nheading|${payload}\n`;
    assert.equal(Buffer.byteLength(frame), 65536);
    assert.equal(decode(Buffer.from(frame))[0].text, payload);
    rejected(() => decodeUiView(frame.slice(0, -1) + 'x\n'));
}
for (const bytes of [[255], [192, 128], [237, 160, 128], [240, 40, 140, 40]]) {
    rejected(() => decodeUiView(Buffer.concat([Buffer.from('SVR-UI-1\nheading|'), Buffer.from(bytes), Buffer.from('\n')])));
}
assert.equal(decode('SVR-UI-1\nheading|😀�\n')[0].text, '😀�');
assert.equal(decode('SVR-UI-1\nheading|x\n' + 'text|x\n'.repeat(255)).length, 256);
rejected(() => decodeUiView('SVR-UI-1\nheading|x\n' + 'text|x\n'.repeat(256)));
const nested = depth => 'SVR-UI-2\nheading|x\n' + Array.from({ length: depth }, (_, index) => `group|g-${index}|vertical|plain\n`).join('') + Array.from({ length: depth }, (_, index) => `end|g-${depth - index - 1}\n`).join('');
decode(nested(8), 2);
rejected(() => decodeUiView(nested(9), { version: 2 }));
const siblings = count => 'SVR-UI-2\nheading|x\n' + Array.from({ length: count }, (_, index) => `group|g-${index}|vertical|card\nend|g-${index}\n`).join('');
decode(siblings(256), 2);
rejected(() => decodeUiView(siblings(257), { version: 2 }));
for (const tail of ['end|missing\n', 'group|card|vertical|card\n',
    'group|card|vertical|card\nend|wrong\n',
    'group|card|vertical|card\nend|card\ngroup|card|horizontal|plain\nend|card\n',
    'group|card|diagonal|card\nend|card\n',
    'group|card|vertical|card\nbutton|save|One\nend|card\nbutton|save|Two\n']) {
    rejected(() => decodeUiView('SVR-UI-2\nheading|x\n' + tail, { version: 2 }));
}
// Actions and layout keys intentionally occupy independent namespaces.
decode('SVR-UI-2\nheading|x\ngroup|save|vertical|plain\nbutton|save|Save\nend|save\n', 2);
for (const action of ['bad" autofocus="x', 'save|changed', 'save\n', 'CAPS']) {
    rejected(() => renderUiView([{ kind: 'heading', text: 'x' }, { kind: 'button', action, label: 'Save' }], token));
}
rejected(() => renderUiView([{ kind: 'heading', text: 'x\nbutton|injected|Bad' }], token));
rejected(() => renderUiView(nodes.slice(0, -1), token));
rejected(() => renderUiView(nodes, token + '\n'));
rejected(() => renderUiView(nodes, token, 'https://example.invalid'));
rejected(() => renderUiView(nodes, token, '/action', '\ud800'));
renderUiView(nodes, token, '/action', 'x'.repeat(131072));
rejected(() => renderUiView(nodes, token, '/action', 'x'.repeat(131073)));
const scratch = mkdtempSync(join(tmpdir(), 'sovra-ui-transport-'));
try {
    // Controlled byte producers exercise transport errors; real Sovra execution
    // remains covered separately by the UI component/layout suites.
    const source = join(scratch, 'unused.svr');
    writeFileSync(source, 'fn main() {}');
    writeFileSync(join(scratch, 'run'), 'process.stdout.write(Buffer.from([0xff]));');
    const invalid = createSovraProgram({ compiler: process.execPath, source, cwd: scratch, engine: 'interpreter' });
    try { rejected(() => invalid.run()); } finally { invalid.close(); }
    writeFileSync(join(scratch, 'run'), 'process.stdout.write("SVR-UI-1\\nheading|λ😀�\\n");');
    const valid = createSovraProgram({ compiler: process.execPath, source, cwd: scratch, engine: 'interpreter' });
    try { assert.equal(decode(valid.run().stdout)[0].text, 'λ😀�'); } finally { valid.close(); }
    writeFileSync(join(scratch, 'build'), 'process.stdout.write(Buffer.from([0xff]));');
    rejected(() => createSovraProgram({ compiler: process.execPath, source, cwd: scratch, engine: 'javascript' }));
} finally { rmSync(scratch, { recursive: true, force: true }); }
console.log(`PASS shared UI protocol: ${checks} admission checks; strict UTF-8, exact quotas, grouped views, HTML and compatibility.`);
