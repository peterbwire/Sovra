import { createRequire } from 'node:module';
import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawn } from 'node:child_process';

// Optional official host test; dependencies come from NODE_PATH or local tools.
const { runTests, downloadAndUnzipVSCode } = createRequire(import.meta.url)('@vscode/test-electron');
const scratch = await mkdtemp(join(tmpdir(), 'sovra-vscode-test-'));
const artifacts = resolve('target/ide');
await mkdir(artifacts, { recursive: true });
const untrusted = process.env.SOVRA_TEST_UNTRUSTED === '1';
const packageWorkspace = process.env.SOVRA_TEST_PACKAGE_WORKSPACE === '1';
if (untrusted && packageWorkspace) throw new Error('Choose one host acceptance mode');
const workspace = join(scratch, 'workspace with spaces');
if (packageWorkspace) {
    await mkdir(join(workspace, 'app/src'), { recursive: true }); await mkdir(join(workspace, 'lib'), { recursive: true });
    await writeFile(join(workspace, 'app/sovra.toml'), '[project]\nname = "app"\nentry = "src/main.svr"\n[dependencies.api]\npath = "../lib"\n');
    await writeFile(join(workspace, 'lib/sovra.toml'), '[project]\nname = "lib"\nentry = "main.svr"\n');
    await writeFile(join(workspace, 'app/src/main.svr'), 'use api::exports; fn main() { print(api::exports::value()); }');
    await writeFile(join(workspace, 'lib/main.svr'), '// λ😀\r\nmod exports { export fn value() -> Int { return 1; } } fn main() {}');
    await writeFile(join(workspace, 'notes.svr'), 'fn main() {}');
}
if (untrusted) {
    await mkdir(join(scratch, 'workspace'));
    await mkdir(join(scratch, 'profile/User'), { recursive: true });
    await writeFile(join(scratch, 'profile/User/settings.json'), JSON.stringify({ 'security.workspace.trust.startupPrompt': 'never', 'security.workspace.trust.emptyWindow': false }));
}
try {
    const options = {
        version: '1.106.0', cachePath: process.env.SOVRA_VSCODE_CACHE || join(scratch, 'downloads'),
        extensionDevelopmentPath: resolve('toolchain/ide'),
        extensionTestsPath: resolve(packageWorkspace ? 'scripts/fixtures/vscode-package-tests.cjs' : 'scripts/fixtures/vscode-extension-tests.cjs'),
        extensionTestsEnv: { SOVRA_TEST_COMPILER: resolve('target/debug', process.platform === 'win32' ? 'svr.exe' : 'svr'), SOVRA_TEST_NODE: process.execPath, SOVRA_TEST_ARTIFACTS: artifacts, SOVRA_TEST_UNTRUSTED: untrusted ? '1' : '0', SOVRA_TEST_WORKSPACE: packageWorkspace ? workspace : '' },
        launchArgs: ['--no-sandbox', '--disable-gpu', '--skip-welcome', '--skip-release-notes', ...(untrusted ? [join(scratch, 'workspace')] : [...(packageWorkspace ? [workspace] : []), '--disable-workspace-trust']), '--user-data-dir', join(scratch, 'profile'), '--extensions-dir', join(scratch, 'extensions'), ...(process.env.SOVRA_VSCODE_HEADLESS === '1' ? ['--ozone-platform=headless'] : [])],
    };
    if (untrusted) {
        // runTests adds --disable-workspace-trust. Direct launch verifies actual restricted mode.
        const executable = await downloadAndUnzipVSCode({ version: options.version, cachePath: options.cachePath });
        const code = await new Promise((accept, reject) => {
            const child = spawn(executable, [...options.launchArgs, `--extensionDevelopmentPath=${options.extensionDevelopmentPath}`, `--extensionTestsPath=${options.extensionTestsPath}`, '--disable-updates'], { stdio: 'inherit', env: { ...process.env, ...options.extensionTestsEnv } });
            child.on('error', reject); child.on('close', accept);
        });
        if (code !== 0) throw new Error(`Restricted VS Code host failed: ${code}`);
    } else await runTests(options);
} finally { await rm(scratch, { recursive: true, force: true }); }
