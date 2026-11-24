import { validateWorkspace, MAX_BYTES } from './workspace.js';
const $ = id => document.getElementById(id);
const source = $('source');
const token = document.querySelector('meta[name="sovra-token"]').content;
const initial = '// Welcome to Sovra.\nfn main() {\n    print("Hello, Sovra!")\n}\n';
let revision = 0;
let workspace = { schema_version: 1, id: crypto.randomUUID(), active: 'main.svr', files: [{ name: 'main.svr', source: initial }] };
try {
    const saved = localStorage.getItem('sovra.editor.workspace');
    if (saved) workspace = validateWorkspace(JSON.parse(saved));
    else workspace.files[0].source = localStorage.getItem('sovra.editor.source') ?? initial;
} catch { /* Keep the starter workspace if storage is unavailable or damaged. */ }
const selected = () => workspace.files.find(file => file.name === workspace.active);
const views = new Map();
source.value = selected().source;
function fileList() {
    $('filename').textContent = workspace.active;
    $('documents').replaceChildren(...workspace.files.map(file => {
        const button = document.createElement('button');
        button.textContent = `◇ ${file.name}`;
        button.setAttribute('aria-current', String(file.name === workspace.active));
        if (file.name === workspace.active) button.className = 'selected';
        button.onclick = () => activate(file.name);
        return button;
    }));
}
function activate(name) {
    views.set(workspace.active, { start: source.selectionStart, end: source.selectionEnd, scroll: source.scrollTop });
    workspace.active = name;
    source.value = selected().source;
    changed(); fileList();
    const view = views.get(name);
    if (view) { source.setSelectionRange(view.start, view.end); source.scrollTop = view.scroll; }
    position();
}
function position() {
    const before = source.value.slice(0, source.selectionStart).split('\n');
    $('position').textContent = `Ln ${before.length}, Col ${Array.from(before.at(-1)).length + 1}`;
}
function changed() {
    revision++;
    selected().source = source.value;
    $('lines').textContent = source.value.split('\n').map((_, i) => i + 1).join('\n');
    $('diagnostics').replaceChildren();
    $('status').textContent = 'Source changed · check again';
    try { validateWorkspace(workspace); localStorage.setItem('sovra.editor.workspace', JSON.stringify(workspace)); $('dirty').textContent = 'Workspace saved in this browser'; }
    catch { $('dirty').textContent = 'Draft not saved · workspace invalid, too large or storage full'; }
    position();
}
$('new-file').onsubmit = event => {
    event.preventDefault();
    try {
        const name = $('new-name').value.trim();
        workspace = validateWorkspace({ ...workspace, files: [...workspace.files, { name, source: '// New Sovra source\n' }], active: name });
        source.value = selected().source; changed(); fileList(); $('new-name').value = ''; source.focus();
    } catch (error) { $('status').textContent = error.message; }
};
source.addEventListener('input', changed);
source.addEventListener('keyup', position);
source.addEventListener('click', position);
source.addEventListener('scroll', () => { $('lines').scrollTop = source.scrollTop; });
source.addEventListener('keydown', event => {
    if (event.key === 'Tab') { event.preventDefault(); source.setRangeText('    ', source.selectionStart, source.selectionEnd, 'end'); changed(); }
    if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') { event.preventDefault(); document.querySelector('[data-action="check"]').click(); }
});
$('open').onclick = () => $('file').click();
$('file').onchange = async () => {
    const file = $('file').files[0];
    if (!file) return;
    if (!file.name.endsWith('.svr') || file.size > 512 * 1024) { $('status').textContent = 'Choose a .svr file up to 512 KiB'; return; }
    try {
        const text = new TextDecoder('utf-8', { fatal: true }).decode(await file.arrayBuffer());
        const files = workspace.files.filter(item => item.name !== file.name);
        workspace = validateWorkspace({ ...workspace, files: [...files, { name: file.name, source: text }], active: file.name });
        source.value = selected().source; changed(); fileList();
    } catch (error) { $('status').textContent = `Cannot open source: ${error.message}`; }
    finally { $('file').value = ''; }
};
function download(text, name, type) {
    const url = URL.createObjectURL(new Blob([text], { type }));
    const anchor = document.createElement('a'); anchor.href = url; anchor.download = name; anchor.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
}
$('save').onclick = () => download(source.value, workspace.active, 'text/plain;charset=utf-8');
$('export-workspace').onclick = () => {
    try { download(JSON.stringify(validateWorkspace(workspace), null, 2), 'sovra-workspace.json', 'application/json'); }
    catch (error) { $('status').textContent = error.message; }
};
$('import-workspace').onclick = () => $('workspace-file').click();
$('workspace-file').onchange = async () => {
    const file = $('workspace-file').files[0];
    if (!file) return;
    try {
        if (file.size > 4 * 1024 * 1024) throw new Error('Workspace JSON exceeds 4 MiB');
        const next = validateWorkspace(JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(await file.arrayBuffer())));
        workspace = next; views.clear(); source.value = selected().source; changed(); fileList();
    } catch (error) { $('status').textContent = `Cannot import workspace: ${error.message}`; }
    finally { $('workspace-file').value = ''; }
};
async function post(path, body) {
    const response = await fetch(path, { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Sovra-Token': token }, body: JSON.stringify(body) });
    const result = await response.json();
    if (!response.ok) throw new Error(result.error);
    return result;
}
$('stop').onclick = () => post('/stop', {}).catch(error => { $('status').textContent = error.message; });
document.querySelectorAll('[data-action]').forEach(button => { button.onclick = async () => {
    const snapshot = source.value, version = revision;
    if (new TextEncoder().encode(snapshot).length > MAX_BYTES) { $('status').textContent = 'Selected source exceeds 512 KiB'; return; }
    document.querySelectorAll('[data-action]').forEach(item => { item.disabled = true; });
    $('stop').disabled = false; $('status').textContent = 'Compiler working…'; $('diagnostics').replaceChildren();
    $('result-title').textContent = button.dataset.action === 'run' ? 'PROGRAM OUTPUT' : 'COMPILER';
    try {
        const result = await post('/compile', { action: button.dataset.action, source: snapshot });
        if (version !== revision) { $('status').textContent = 'Source changed · result discarded'; return; }
        $('output').textContent = result.error || [result.stdout, result.stderr].filter(Boolean).join('\n') || '(No output)';
        $('status').textContent = result.status === 0 && !result.error ? 'Completed successfully' : 'Stopped or failed';
        if (button.dataset.action === 'check' && result.stdout) {
            const report = JSON.parse(result.stdout);
            if (report.schema_version !== 1 || !Array.isArray(report.diagnostics)) throw new Error('Unsupported compiler report');
            $('output').textContent = report.success ? 'No compiler errors.' : `${report.diagnostics.length} diagnostic(s)`;
            for (const diagnostic of report.diagnostics) {
                const item = document.createElement('li'), link = document.createElement('button');
                const location = diagnostic.location;
                link.textContent = `${diagnostic.code}: ${diagnostic.message}${location ? ` · ${workspace.active}:${location.line + 1}:${location.column + 1}` : ''}`;
                link.onclick = () => {
                    if (revision !== version || !location) return;
                    // Compiler offsets are UTF-8 bytes; textarea selection uses UTF-16 units.
                    const bytes = new TextEncoder().encode(snapshot);
                    const start = new TextDecoder().decode(bytes.subarray(0, location.start)).length;
                    const end = new TextDecoder().decode(bytes.subarray(0, location.end)).length;
                    source.focus(); source.setSelectionRange(start, end); position();
                };
                item.append(link); $('diagnostics').append(item);
            }
        }
    } catch (error) { $('status').textContent = 'Request failed'; $('output').textContent = error.message; }
    finally { document.querySelectorAll('[data-action]').forEach(item => { item.disabled = false; }); $('stop').disabled = true; }
}; });
changed(); fileList(); $('status').textContent = 'Ready';
