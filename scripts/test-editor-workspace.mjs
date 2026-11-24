import assert from 'node:assert/strict';
import { validateWorkspace, MAX_BYTES, MAX_FILES } from '../toolchain/editor/web/workspace.js';

const original = { schema_version: 1, id: 'test-workspace', active: 'main.svr', files: [{ name: 'main.svr', source: 'fn main() {}' }] };
assert.deepEqual(validateWorkspace(original), original);
const restored = validateWorkspace(JSON.parse(JSON.stringify(original)));
restored.files[0].source = 'changed';
assert.equal(original.files[0].source, 'fn main() {}');
for (const name of ['../escape.svr', '/absolute.svr', 'C:\\escape.svr', 'file.txt', '<img>.svr', 'a\n.svr']) {
    assert.throws(() => validateWorkspace({ ...original, active: name, files: [{ name, source: '' }] }));
}
for (const value of [null, {}, { ...original, schema_version: 2 }, { ...original, active: 'absent.svr' }, { ...original, files: [] }, { ...original, id: '<img>' }]) assert.throws(() => validateWorkspace(value));
assert.throws(() => validateWorkspace({ ...original, files: [...original.files, { name: 'MAIN.svr', source: '' }] }));
assert.throws(() => validateWorkspace({ ...original, files: [{ name: 'main.svr', source: '\ud800' }] }));
const files = Array.from({ length: MAX_FILES }, (_, index) => ({ name: `file${index}.svr`, source: '' }));
assert.equal(validateWorkspace({ ...original, active: 'file0.svr', files }).files.length, MAX_FILES);
assert.throws(() => validateWorkspace({ ...original, active: 'file0.svr', files: [...files, { name: 'extra.svr', source: '' }] }));
assert.equal(validateWorkspace({ ...original, files: [{ name: 'main.svr', source: 'λ'.repeat(MAX_BYTES / 2) }] }).files.length, 1);
assert.throws(() => validateWorkspace({ ...original, files: [{ name: 'main.svr', source: 'λ'.repeat(MAX_BYTES / 2) }, { name: 'extra.svr', source: 'x' }] }));
console.log('Editor workspace: roundtrip, identity, unique names, traversal rejection, Unicode, exact aggregate/file count limits passed.');
