import assert from 'node:assert/strict';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { pathToFileURL, fileURLToPath } from 'node:url';
import { prepareTaskTarget, validateTaskTarget } from '../toolchain/ide/task-targets.mjs';
function fixture() {
    const root = join(tmpdir(), 'task target with spaces');
    const document = (name, dirty = false) => ({ uri: { scheme: 'file', toString: () => pathToFileURL(join(root, name)).href }, version: 1, isDirty: dirty, saves: 0, async save() { this.saves++; this.isDirty = false; return true; } });
    const active = document('src/main.svr', true), provider = document('lib/main.svr', true), unrelated = document('notes.svr', true), manifest = document('sovra.toml');
    const docs = [active, provider, unrelated, manifest];
    const plan = { schema_version: 1, package: true, target: root, cwd: root, sources: [join(root, 'src/main.svr'), join(root, 'lib/main.svr')], manifests: [join(root, 'sovra.toml')] };
    let resolves = 0;
    const prepare = (resolver = async () => { resolves++; return plan; }) => prepareTaskTarget(active, () => docs, resolver, async uri => fileURLToPath(uri), async () => {});
    return { root, active, provider, unrelated, manifest, docs, plan, prepare, resolves: () => resolves };
}
{
    const f = fixture(); const result = await f.prepare(); assert.equal(result.target, f.root); assert.equal(result.cwd, f.root); assert.equal(f.active.saves, 1); assert.equal(f.provider.saves, 1); assert.equal(f.manifest.saves, 0); assert.equal(f.unrelated.saves, 0); assert.equal(f.unrelated.isDirty, true); assert.equal(f.resolves(), 2);
}
{
    const f = fixture(); f.manifest.isDirty = true; await assert.rejects(f.prepare(), /Save package manifests/); assert.equal(f.provider.saves, 0); assert.equal(f.manifest.saves, 0);
}
{
    const f = fixture(); f.provider.save = async () => false; await assert.rejects(f.prepare(), /save cancelled/);
}
{
    const f = fixture(); let calls = 0; await assert.rejects(f.prepare(async () => { calls++; f.active.version++; return f.plan; }), /kept changing/); assert.equal(calls, 3);
}
{
    const f = fixture(); let calls = 0; const result = await f.prepare(async () => { if (++calls === 1) throw Object.assign(new Error('changed'), { code: -32801 }); return f.plan; }); assert.equal(result.package, true); assert.equal(calls, 3);
}
{
    const f = fixture(); let calls = 0; await assert.rejects(f.prepare(async () => ++calls === 1 ? f.plan : { ...f.plan, target: join(f.root, 'changed'), cwd: join(f.root, 'changed') }), /target changed/);
}
{
    const f = fixture(); f.docs.push({ ...f.provider, uri: { scheme: 'file', toString: () => f.provider.uri.toString() } }); await assert.rejects(f.prepare(), /duplicate/); assert.equal(f.provider.saves, 0);
}
{
    const f = fixture(); for (const mutate of [p => { p.target = '--help'; }, p => { p.sources = []; }, p => { p.manifests = []; }, p => { p.cwd = join(f.root, 'wrong'); }]) { const p = structuredClone(f.plan); mutate(p); assert.throws(() => validateTaskTarget(p, f.plan.sources[0])); }
    assert.equal(validateTaskTarget({ schema_version: 1, package: false, target: f.plan.sources[0], cwd: join(f.root, 'src'), sources: [f.plan.sources[0]], manifests: [] }, f.plan.sources[0]).package, false);
}
console.log('Task targeting: root/cwd, related-only saves, manifest refusal, failed saves, bounded cancellation/change retries, duplicate editors and strict descriptors passed.');
