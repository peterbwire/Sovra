// Exercise the private File Manager browser with real Sovra file operations.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, unlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');
const compiler = resolve(process.argv[2] ?? join(root, 'target', 'debug',
    process.platform === 'win32' ? 'svr.exe' : 'svr'));
const serverFile = join(root, 'dogfood/file-manager/gui/server.mjs');
const source = join(root, 'dogfood/file-manager/main.svr');
const scratch = mkdtempSync(join(tmpdir(), 'sovra-file-manager-'));
let child;
try {
    const results = [];
    for (const engine of ['javascript', 'interpreter']) {
        const work = join(scratch, engine);
        mkdirSync(work);
        mkdirSync(join(work, 'subdir'));
        const content = 'First <script> & café\nSecond line\n';
        child = spawn(process.execPath, [serverFile, '--root', work, '--compiler', compiler,
            '--source', source, '--engine', engine, '--port', '0'],
            { stdio: ['ignore', 'pipe', 'pipe'], windowsHide: true });
        const address = await new Promise((resolveAddress, reject) => {
            let output = '';
            let errors = '';
            const timer = setTimeout(() => reject(new Error(`File Manager startup timeout: ${errors}`)), 10000);
            child.stderr.on('data', chunk => { errors += chunk; });
            child.stdout.on('data', chunk => {
                output += chunk;
                const match = output.match(/Sovra File Manager \([^)]+\): (http:\/\/127\.0\.0\.1:\d+\/)/);
                if (match) { clearTimeout(timer); resolveAddress(match[1]); }
            });
            child.once('error', reject);
            child.once('exit', code => reject(new Error(`File Manager exited ${code}: ${errors}`)));
        });
        const emptyDirectory = await (await fetch(address)).text();
        assert.match(emptyDirectory, /Page 1 of 1/);
        assert.match(emptyDirectory, /No regular files\./);
        assert.match(emptyDirectory, /name="action" value="create"/);
        assert.ok(!emptyDirectory.includes('name="action" value="preview-0"'));
        assert.ok(!emptyDirectory.includes('value="page-next"'));
        assert.ok(!emptyDirectory.includes('subdir'), 'directory shown in empty listing');
        writeFileSync(join(work, 'alpha.txt'), content);
        writeFileSync(join(work, 'empty.txt'), '');
        writeFileSync(join(work, "quote'&.txt"), 'quoted\n');
        writeFileSync(join(work, 'r\u00e9sum\u00e9.txt'), 'non-ASCII name\n');
        writeFileSync(join(work, 'invalid.dat'), Buffer.from([0xff]));
        writeFileSync(join(work, 'large.txt'), 'x'.repeat(65537));
        const initial = await (await fetch(address)).text();
        assert.match(initial, /alpha\.txt/);
        assert.match(initial, /quote&#39;&amp;\.txt/);
        assert.match(initial, /r\u00e9sum\u00e9\.txt/);
        assert.match(initial, /name="action" value="preview-0"/);
        assert.match(initial, /name="action" value="copy-0"/);
        assert.ok(!initial.includes('<h2>Preview</h2>'));
        assert.ok(!initial.includes('subdir'), 'directory shown as a regular file');
        const token = initial.match(/name="token" value="([0-9a-f]{64})"/)?.[1];
        assert.ok(token);
        const action = (actionName, value, suppliedToken = token) =>
            fetch(new URL('action', address), {
                method: 'POST', body: new URLSearchParams({ token: suppliedToken, action: actionName,
                    ...(value === undefined ? {} : { value }) }),
            });
        // A new file sorts before alpha, but an existing form must still refer
        // to alpha rather than silently switching its target by list index.
        writeFileSync(join(work, '0-before.txt'), 'new file\n');
        const preview = await action('preview-0');
        assert.equal(preview.status, 200);
        const html = await preview.text();
        assert.match(html, /First &lt;script&gt; &amp; café/);
        assert.ok(!html.includes('<script>'));
        const copied = await action('copy-0', 'copy.txt');
        assert.equal(copied.status, 200);
        assert.match(await copied.text(), /copied bytes:/);
        assert.equal(readFileSync(join(work, 'copy.txt'), 'utf8'), content);
        assert.equal((await action('copy-0', '../outside.txt')).status, 400);
        assert.equal((await action('copy-0', 'subdir')).status, 400);
        assert.equal((await action('preview-2')).status, 400);
        assert.equal((await action('preview-3')).status, 400);
        assert.equal((await action('preview-0', undefined, 'wrong')).status, 403);
        assert.equal((await fetch(new URL('action', address), {
            method: 'POST', headers: { origin: 'https://attacker.example' },
            body: new URLSearchParams({ token, action: 'preview-0' }),
        })).status, 403);
        assert.equal((await action('preview-999')).status, 400);
        assert.equal((await fetch(new URL('action', address), {
            method: 'POST', body: new URLSearchParams({ token, action: 'preview-0', source: '../outside.txt' }),
        })).status, 400);
        assert.ok(!readFileSync(join(work, 'alpha.txt'), 'utf8').includes('copy.txt'));
        const emptyListing = await (await fetch(address)).text();
        const emptyToken = emptyListing.match(/name="token" value="([0-9a-f]{64})"/)?.[1];
        assert.ok(emptyToken);
        // Sorted entries are 0-before, alpha, copy, empty, invalid, large,
        // quote, and the non-ASCII filename.
        assert.match(emptyListing, /empty\.txt/);
        const emptyPreview = await action('preview-3', undefined, emptyToken);
        assert.equal(emptyPreview.status, 200);
        assert.match(await emptyPreview.text(), /<h2>Preview<\/h2><pre><\/pre>/);
        const unicodePreview = await action('preview-7', undefined, emptyToken);
        assert.equal(unicodePreview.status, 200);
        assert.match(await unicodePreview.text(), /non-ASCII name/);
        for (let index = 0; index < 85; index++) {
            writeFileSync(join(work, `z${String(index).padStart(3, '0')}.txt`), `page ${index}\n`);
        }
        const firstPage = await (await fetch(address)).text();
        assert.match(firstPage, /Page 1 of 2/);
        assert.match(firstPage, /value="page-next"/);
        assert.ok(!firstPage.includes('z072.txt'));
        const firstToken = firstPage.match(/name="token" value="([0-9a-f]{64})"/)?.[1];
        assert.ok(firstToken);
        const secondResponse = await action('page-next', undefined, firstToken);
        assert.equal(secondResponse.status, 200);
        const secondPage = await secondResponse.text();
        assert.match(secondPage, /Page 2 of 2/);
        assert.match(secondPage, /value="page-previous"/);
        assert.match(secondPage, /z072\.txt/);
        const secondToken = secondPage.match(/name="token" value="([0-9a-f]{64})"/)?.[1];
        assert.ok(secondToken);
        const lastPreview = await action('preview-0', undefined, secondToken);
        assert.equal(lastPreview.status, 200);
        assert.match(await lastPreview.text(), /page 72/);
        const lastCopy = await action('copy-0', 'last-page-copy.txt', secondToken);
        assert.equal(lastCopy.status, 200);
        assert.match(await lastCopy.text(), /Page 2 of 2/);
        assert.equal(readFileSync(join(work, 'last-page-copy.txt'), 'utf8'), 'page 72\n');
        const previousPage = await action('page-previous', undefined, secondToken);
        assert.equal(previousPage.status, 200);
        const returnedPage = await previousPage.text();
        assert.match(returnedPage, /Page 1 of 2/);
        assert.match(returnedPage, /name="action" value="create"/);
        const createToken = returnedPage.match(/name="token" value="([0-9a-f]{64})"/)?.[1];
        assert.ok(createToken);
        assert.equal((await action('create', '../outside.txt', createToken)).status, 400);
        assert.equal((await action('create', 'alpha.txt', createToken)).status, 400);
        assert.equal(readFileSync(join(work, 'alpha.txt'), 'utf8'), content);
        const created = await action('create', 'new-empty.txt', createToken);
        assert.equal(created.status, 200);
        assert.match(await created.text(), /Created new-empty\.txt/);
        assert.equal(readFileSync(join(work, 'new-empty.txt'), 'utf8'), '');
        assert.equal((await action('create', 'new-empty.txt', createToken)).status, 400);
        unlinkSync(join(work, 'z072.txt'));
        assert.equal((await action('preview-0', undefined, secondToken)).status, 400);
        assert.equal((await action('copy-0', 'missing-source-copy.txt', secondToken)).status, 400);
        assert.equal((await action('create', 'missing-source-copy.txt', createToken)).status, 200);
        assert.equal(readFileSync(join(work, 'missing-source-copy.txt'), 'utf8'), '');
        results.push(readFileSync(join(work, 'copy.txt'), 'utf8'));
        const closed = once(child, 'close');
        child.kill();
        await closed;
        child = undefined;
    }
    assert.equal(results[0], results[1]);
    console.log('PASS File Manager browser: Sovra preview/copy/create, pagination, UTF-8, path checks, both engines');
} finally {
    child?.kill();
    rmSync(scratch, { recursive: true, force: true });
}
