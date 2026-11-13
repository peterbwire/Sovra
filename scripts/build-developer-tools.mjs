import { readFile, writeFile, mkdir, mkdtemp, copyFile, rename, rm, lstat } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { createHash } from 'node:crypto';
import { verifyIdeVsix } from './verify-ide-vsix.mjs';

const root = fileURLToPath(new URL('..', import.meta.url));
const files = [
    'developer-tools/launch.mjs', 'developer-tools/README.md',
    'editor/README.md', 'editor/server.mjs', 'editor/web/index.html', 'editor/web/app.js', 'editor/web/workspace.js', 'editor/web/style.css',
    'ide/package.json', 'ide/extension.cjs', 'ide/task-targets.mjs', 'ide/README.md', 'ide/LICENSE', 'ide/LICENSE-APACHE', 'ide/language-configuration.json',
    'ide/syntaxes/sovra.tmLanguage.json', 'ide/language-server/server.mjs', 'ide/language-server/transport.mjs', 'ide/language-server/symbols.mjs', 'ide/language-server/packages.mjs', 'ide/language-server/package-navigation.mjs',
];
export async function buildDeveloperTools(output, vsix) {
    const destination = resolve(output);
    try { await lstat(destination); throw new Error('Developer-tools output already exists'); } catch (error) { if (error.code !== 'ENOENT') throw error; }
    const cargo = await readFile(join(root, 'Cargo.toml'), 'utf8');
    const compilerVersion = /^version = "([0-9.]+)"$/m.exec(cargo)?.[1];
    const ide = JSON.parse(await readFile(join(root, 'toolchain/ide/package.json'), 'utf8'));
    if (!compilerVersion || !/^\d+\.\d+\.\d+$/.test(compilerVersion) || !/^\d+\.\d+\.\d+$/.test(ide.version)) throw new Error('Compiler or IDE version unavailable');
    await mkdir(dirname(destination), { recursive: true });
    const stage = await mkdtemp(join(dirname(destination), '.sovra-dev-tools-'));
    try {
        for (const name of files) {
            const input = join(root, 'toolchain', name);
            if (!(await lstat(input)).isFile()) throw new Error(`Developer-tool source is not a regular file: ${name}`);
            const target = join(stage, name.startsWith('developer-tools/') ? name.slice('developer-tools/'.length) : name);
            await mkdir(dirname(target), { recursive: true }); await copyFile(input, target);
        }
        let extension = null;
        if (vsix) {
            const info = await lstat(vsix);
            if (!info.isFile() || info.size < 4 || info.size > 8 * 1024 * 1024) throw new Error('VSIX must be a regular file of at most 8 MiB');
            const bytes = await readFile(vsix);
            if (bytes.length > 8 * 1024 * 1024 || bytes.readUInt32LE(0) !== 0x04034b50) throw new Error('Invalid or oversized VSIX');
            const expected = new Map();
            for (const name of files.filter(name => name.startsWith('ide/'))) {
                const member = name === 'ide/LICENSE' ? 'license.txt' : name.slice(4);
                expected.set(`extension/${member}`.toLowerCase(), await readFile(join(root, 'toolchain', name)));
            }
            verifyIdeVsix(bytes, expected, ide);
            const name = `sovra-language-${ide.version}.vsix`;
            await copyFile(vsix, join(stage, name));
            extension = { file: name, sha256: createHash('sha256').update(bytes).digest('hex') };
        }
        await writeFile(join(stage, 'manifest.json'), JSON.stringify({ schema_version: 1, compiler_version: compilerVersion, ide_version: ide.version, minimum_node_major: 22, vsix: extension }, null, 2) + '\n');
        await rename(stage, destination);
    } finally { await rm(stage, { recursive: true, force: true }); }
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
    if (!process.argv[2] || process.argv.length > 4) throw new Error('Usage: node build-developer-tools.mjs OUTPUT [VSIX]');
    await buildDeveloperTools(process.argv[2], process.argv[3]);
}
