import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { checkDocLinks } from './check-doc-links.mjs';

const scratch = mkdtempSync(join(tmpdir(), 'sovra-doc-links-'));
try {
    const root = join(scratch, 'repo'); mkdirSync(root);
    writeFileSync(join(root, 'notes with spaces.md'), '# Notes\n');
    writeFileSync(join(root, 'README.md'), '[note](<notes with spaces.md>)\n[root](/notes%20with%20spaces.md#notes)\n[title](notes%20with%20spaces.md "A title")\n[reference][note]\n[note]: notes%20with%20spaces.md\n[web](https://example.invalid/path)\n[heading](#local)\n```markdown\n[example](missing.md)\n```\n~~~\n[another example](absent.md)\n~~~\n');
    for (const directory of ['target', 'node_modules', '.git']) { mkdirSync(join(root, directory)); writeFileSync(join(root, directory, 'generated.md'), '[ignored](missing.md)'); }
    assert.deepEqual(checkDocLinks(root).errors, []);
    assert.equal(checkDocLinks(root).local_links, 4);
    writeFileSync(join(scratch, 'outside.md'), 'outside');
    writeFileSync(join(root, 'broken.md'), '[missing](missing.md)\n![image](absent.png)\n[escape](../outside.md)\n[invalid](bad%zz.md)\n[file](file:///tmp/only-here.md)\n[drive](C:/only-here.md)\n');
    const errors = checkDocLinks(root).errors;
    assert.equal(errors.length, 6);
    assert.ok(errors.some(error => error.includes('escapes repository')));
    assert.ok(errors.some(error => error.includes('invalid URL escape')));
    console.log('PASS documentation checker: real targets, encoded/angle paths, reference definitions, ignored fences/builds, missing images/files and repository escape detection.');
} finally { rmSync(scratch, { recursive: true, force: true }); }
