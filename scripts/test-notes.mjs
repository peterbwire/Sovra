// Cross-engine persistence tests for the reusable Sovra Notes package.
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { createSovraProgram } from '../dogfood/01-task-manager/gui/bridge.mjs';

const root = resolve(import.meta.dirname, '..');
const compiler = resolve(process.argv[2] ?? join(root, 'target', 'debug',
    process.platform === 'win32' ? 'svr.exe' : 'svr'));
const source = join(root, 'dogfood/sovra-notes/app');
const scratch = mkdtempSync(join(tmpdir(), 'sovra-notes-'));
try {
    for (const engine of ['interpreter', 'javascript']) {
        const path = join(scratch, `${engine}.txt`);
        const program = createSovraProgram({ compiler, source, engine, cwd: root });
        try {
            const run = (args, status, output) => {
                const result = program.run(args);
                assert.equal(result.status, status, result.stderr || result.stdout);
                assert.equal(result.stderr, '');
                assert.equal(result.stdout.replaceAll('\r\n', '\n'), output);
            };
            run(['list', path], 0, 'count 0\n');
            run(['put', path, 'greeting', 'hello'], 0, 'saved greeting\n');
            run(['put', path, 'café', 'line | invalid'], 1, 'invalid value\n');
            run(['put', path, 'café', 'bonjour'], 0, 'saved café\n');
            run(['get', path, 'greeting'], 0, 'hello\n');
            run(['put', path, 'greeting', 'updated'], 0, 'saved greeting\n');
            run(['get', path, 'greeting'], 0, 'updated\n');
            run(['list', path], 0, 'greeting\ncafé\ncount 2\n');
            run(['delete', path, 'café'], 0, 'deleted café\n');
            run(['get', path, 'café'], 1, 'note not found\n');
            assert.equal(readFileSync(path, 'utf8'), 'SVR-NOTES-1\ngreeting|updated\n');
            const corrupt = 'SVR-NOTES-1\ngreeting|one\ngreeting|two\n';
            writeFileSync(path, corrupt);
            run(['put', path, 'other', 'value'], 1, 'invalid notes file\n');
            assert.equal(readFileSync(path, 'utf8'), corrupt);
        } finally { program.close(); }
    }
    console.log('PASS Sovra Notes: persistent CRUD, Unicode, corruption preservation, both engines');
} finally { rmSync(scratch, { recursive: true, force: true }); }
