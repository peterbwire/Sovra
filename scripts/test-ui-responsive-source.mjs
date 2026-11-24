import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, writeFileSync, existsSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { decodeUiView } from '../toolchain/ui/shared/view.mjs';

const compiler = resolve('target/debug', process.platform === 'win32' ? 'svr.exe' : 'svr');
const source = resolve('scripts/fixtures/ui-responsive.svr');
const scratch = mkdtempSync(join(tmpdir(), 'sovra-responsive-ui-'));
function run(executable, args) {
    const result = spawnSync(executable, args, { encoding: 'utf8', timeout: 15000, maxBuffer: 1024 * 1024 });
    assert.ifError(result.error); assert.equal(result.status, 0, result.stderr);
    return result.stdout.replaceAll('\r\n', '\n');
}
try {
    const state = join(scratch, 'must remain absent.txt');
    const interpreted = run(compiler, ['run', source, '--', 'ui-layout-view', state]);
    const program = join(scratch, 'component.cjs');
    writeFileSync(program, run(compiler, ['build', '--emit', 'js', source]), { mode: 0o600 });
    const generated = run(process.execPath, [program, 'ui-layout-view', state]);
    assert.equal(generated, interpreted);
    const view = decodeUiView(interpreted, { version: 2 });
    assert.ok(view.some(node => node.kind === 'group' && node.key === 'actions' && node.orientation === 'horizontal'));
    assert.ok(view.some(node => node.kind === 'input' && node.action === 'draft' && node.label.length > 80));
    assert.ok(view.some(node => node.kind === 'button' && node.label.length > 80));
    assert.equal(existsSync(state), false);
    console.log('PASS responsive native layout source: both engines, valid grouped view, long labels and read-only view generation. Native rendering requires Windows.');
} finally { rmSync(scratch, { recursive: true, force: true }); }
