import { readFileSync, existsSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { join } from 'node:path';

export function requireNode(version = process.versions.node) {
    if (!/^\d+\./.test(version) || Number(version.split('.')[0]) < 22) throw new Error('Sovra developer tools require Node.js 22 or newer.');
}

// Installed entry point. Resolve all code/compiler paths from this package,
// never from a project directory or a caller-supplied source path.
export async function launch() { try {
    requireNode();
    const mode = process.argv[2];
    if (process.argv.length !== 3 || !['editor', 'lsp', '--check'].includes(mode)) throw new Error('Usage: node launch.mjs editor|lsp|--check');
    const directory = fileURLToPath(new URL('.', import.meta.url));
    const prefix = fileURLToPath(new URL('../', import.meta.url));
    const manifest = JSON.parse(readFileSync(join(directory, 'manifest.json'), 'utf8'));
    if (manifest.schema_version !== 1 || !/^\d+\.\d+\.\d+$/.test(manifest.compiler_version)) throw new Error('Invalid developer-tools manifest.');
    const compiler = join(prefix, 'bin', process.platform === 'win32' ? 'svr.exe' : 'svr');
    if (!existsSync(compiler)) throw new Error('Installed Sovra compiler is missing.');
    const version = spawnSync(compiler, ['--version'], { encoding: 'utf8', timeout: 5000, maxBuffer: 4096, windowsHide: true });
    if (version.error || version.status !== 0 || version.stdout.trim() !== `svr ${manifest.compiler_version}`) throw new Error('Developer tools and installed compiler version differ, or the compiler could not start.');
    if (mode === '--check') console.log(JSON.stringify({ success: true, compiler_version: manifest.compiler_version, ide_version: manifest.ide_version, node_version: process.versions.node, vsix: manifest.vsix }));
    else {
        const script = mode === 'editor' ? join(directory, 'editor/server.mjs') : join(directory, 'ide/language-server/server.mjs');
        process.argv = [process.execPath, script, compiler];
        await import(pathToFileURL(script).href);
    }
} catch (error) { console.error(`Sovra developer tools: ${error.message}`); process.exitCode = 1; } }
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) await launch();
