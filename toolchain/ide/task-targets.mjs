// Task preparation saves only compiler-selected source buffers, never config drafts.
import { dirname, isAbsolute } from 'node:path';
const path = p => typeof p === 'string' && p.isWellFormed() && !p.includes('\0') && Buffer.byteLength(p) <= 4096 && isAbsolute(p);
export function validateTaskTarget(plan, active) {
    if (plan?.schema_version !== 1 || typeof plan.package !== 'boolean' || !path(plan.target) || !path(plan.cwd) || !Array.isArray(plan.sources) || !plan.sources.length || plan.sources.length > 1024 || !plan.sources.every(path) || new Set(plan.sources).size !== plan.sources.length || !plan.sources.includes(active) || !Array.isArray(plan.manifests) || plan.manifests.length > 1024 || !plan.manifests.every(path) || new Set(plan.manifests).size !== plan.manifests.length || Buffer.byteLength(JSON.stringify(plan)) > 1024 * 1024) throw new Error('Invalid compiler task target');
    if (plan.package ? plan.cwd !== plan.target || !plan.manifests.length : plan.target !== active || plan.cwd !== dirname(active) || plan.sources.length !== 1 || plan.manifests.length) throw new Error('Inconsistent task target');
    return plan;
}
export async function prepareTaskTarget(active, getDocuments, resolveTarget, canonicalFile, settle = () => new Promise(done => setTimeout(done, 150))) {
    if (active.isDirty) { if (!await active.save()) throw new Error('Source save cancelled; task not started'); await settle(); }
    let selected;
    for (let attempt = 0; attempt < 3; attempt++) {
        const version = active.version;
        let plan;
        try { plan = await resolveTarget(active); }
        catch (error) { if (error.code === -32801 || error.code === -32800) { await settle(); continue; } throw error; }
        const canonical = await canonicalFile(active.uri.toString());
        if (!canonical || active.isClosed) throw new Error('Task source unavailable');
        validateTaskTarget(plan, canonical);
        const identity = `${plan.package}:${plan.target}`;
        if (selected && selected !== identity) throw new Error('Task target changed; invoke the command again');
        selected = identity;
        const documents = getDocuments().filter(doc => doc.uri.scheme === 'file');
        if (documents.length > 128) throw new Error('Too many open file buffers to prepare a Sovra task');
        const sources = new Set(plan.sources), manifests = new Set(plan.manifests), related = [], seen = new Set();
        for (const doc of documents) {
            const file = await canonicalFile(doc.uri.toString());
            if (!sources.has(file) && !manifests.has(file)) continue;
            if (seen.has(file)) throw new Error('Close duplicate canonical source editors before running a task');
            seen.add(file);
            if (manifests.has(file) && doc.isDirty) throw new Error('Save package manifests before running this task');
            if (sources.has(file)) related.push({ doc, version: doc.version });
        }
        if (active.version !== version || getDocuments().filter(doc => doc.uri.scheme === 'file').some(doc => !documents.includes(doc))) continue;
        const dirty = related.filter(({ doc }) => doc.isDirty);
        if (dirty.length > 32) throw new Error('Save package sources before running this task');
        if (dirty.length) {
            for (const { doc } of dirty) if (!await doc.save()) throw new Error('Source save cancelled; task not started');
            await settle();
            continue; // Resolve again after saves; watch events may replace generations.
        }
        if (related.some(({ doc, version }) => doc.isClosed || doc.version !== version || doc.isDirty)) continue;
        return plan;
    }
    throw new Error('Sources kept changing during task preparation; try again');
}
