import { readdirSync, readFileSync, existsSync, statSync } from 'node:fs';
import { resolve, dirname, relative, join, isAbsolute, sep } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const excluded = new Set(['.git', 'target', 'node_modules', '.vscode-test']);
export function checkDocLinks(root) {
    root = resolve(root);
    const files = [], errors = [];
    function visit(directory) {
        for (const entry of readdirSync(directory, { withFileTypes: true })) {
            if (excluded.has(entry.name) || entry.isSymbolicLink()) continue;
            const path = join(directory, entry.name);
            if (entry.isDirectory()) visit(path);
            else if (entry.isFile() && entry.name.endsWith('.md')) files.push(path);
        }
    }
    visit(root);
    let links = 0;
    for (const file of files) {
        let fence;
        const lines = readFileSync(file, 'utf8').split('\n');
        for (let index = 0; index < lines.length; index++) {
            const line = lines[index];
            const marker = /^\s{0,3}(`{3,}|~{3,})/.exec(line)?.[1];
            if (marker) { if (!fence) fence = marker; else if (marker[0] === fence[0] && marker.length >= fence.length) fence = undefined; continue; }
            if (fence) continue;
            const targets = [...line.matchAll(/!?\[[^\]]*\]\((<[^>]+>|[^)\n]+)\)/g)].map(match => match[1]);
            const definition = /^\s{0,3}\[[^\]]+\]:\s*(<[^>]+>|\S+)/.exec(line);
            if (definition) targets.push(definition[1]);
            for (const raw of targets) {
                let target = raw.startsWith('<') ? raw.slice(1, -1) : raw.trim().split(/\s+["']/)[0];
                if (/^(?:file:|[a-z]:[\\/])/i.test(target)) { errors.push(`${relative(root, file)}:${index + 1}: machine-local target is not portable: ${target}`); continue; }
                if (/^(?:[a-z][a-z0-9+.-]*:|\/\/|#)/i.test(target)) continue;
                target = target.split('#')[0].split('?')[0];
                if (!target) continue;
                try { target = decodeURIComponent(target); }
                catch { errors.push(`${relative(root, file)}:${index + 1}: invalid URL escape: ${raw}`); continue; }
                links++;
                const path = target.startsWith('/') ? resolve(root, '.' + target) : resolve(dirname(file), target);
                const location = relative(root, path);
                if (location === '..' || location.startsWith('..' + sep) || isAbsolute(location)) errors.push(`${relative(root, file)}:${index + 1}: target escapes repository: ${target}`);
                else if (!existsSync(path) || !(statSync(path).isFile() || statSync(path).isDirectory())) errors.push(`${relative(root, file)}:${index + 1}: missing local target: ${target}`);
            }
        }
    }
    return { markdown_files: files.length, local_links: links, errors };
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
    const root = process.argv[2] ? resolve(process.argv[2]) : fileURLToPath(new URL('..', import.meta.url));
    const report = checkDocLinks(root);
    if (report.errors.length) { console.error(report.errors.join('\n')); process.exitCode = 1; }
    else console.log(`PASS documentation targets: ${report.markdown_files} Markdown files, ${report.local_links} local links. External URLs and heading fragments not checked.`);
}
