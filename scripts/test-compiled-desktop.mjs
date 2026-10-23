// Compile and run the reusable desktop adapter directly, without a PowerShell host.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { cpSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

assert.equal(process.platform, 'win32', 'Compiled WPF adapter requires Windows');
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const scratch = mkdtempSync(join(tmpdir(), 'sovra-compiled-desktop-'));
const args = process.argv.slice(2);
assert.ok(args.length === 0 || (args.length === 2 && args[0] === '--package'),
    'Usage: node scripts/test-compiled-desktop.mjs [--package DIRECTORY]');
const evidence = join(root, 'target/compiled-desktop/verification.json');
mkdirSync(dirname(evidence), { recursive: true });
const hash = path => createHash('sha256').update(readFileSync(path)).digest('hex');
const report = {
    schema_version: 1, kind: 'compiled-desktop-qualification', started_at: new Date().toISOString(),
    status: 'running', success: false, checks: null, compiler_sha256: null, sources: [], package: null,
    clean_machine_qualified: false,
};
writeFileSync(evidence, JSON.stringify(report, null, 2) + '\n');
let failure;
function run(binary, args, timeout = 120000) {
    const result = spawnSync(binary, args, { cwd: root, encoding: 'utf8', windowsHide: true,
        timeout, maxBuffer: 4 * 1024 * 1024 });
    assert.ifError(result.error);
    assert.equal(result.signal, null);
    assert.equal(result.status, 0, result.stderr + result.stdout);
    return result.stdout;
}
try {
    const framework = join(process.env.WINDIR, 'Microsoft.NET/Framework64/v4.0.30319');
    const sources = ['Bridge.cs', 'DesktopWindow.cs', 'DesktopTheme.cs', 'ApplicationProfile.cs', 'ProfileLease.cs', 'ProcessLifetime.cs', 'InvocationWorker.cs'].map(name => join(root, 'toolchain/ui/windows/compiled', name));
    report.sources = [...sources, join(root, 'toolchain/ui/windows/compiled/Entry.cs'),
        join(root, 'scripts/fixtures/CompiledDesktopTests.cs'), join(root, 'scripts/fixtures/UiProcessFixture.cs'),
        join(root, 'scripts/fixtures/ui-moving.svr'), join(root, 'dogfood/01-task-manager/main.svr'),
        join(root, 'examples/ui-desktop/component.svr')]
        .map(path => ({ path, sha256: hash(path) }));
    report.compiler_sha256 = hash(join(root, 'target/debug/svr.exe'));
    const executable = join(scratch, 'compiled desktop tests.exe');
    run(join(framework, 'csc.exe'), ['/nologo', '/target:exe', '/platform:x64', '/optimize+',
        '/reference:System.Xaml.dll', '/reference:System.Web.Extensions.dll', ...['PresentationFramework', 'PresentationCore', 'WindowsBase', 'UIAutomationClient', 'UIAutomationTypes'].map(name =>
            '/reference:' + join(framework, 'WPF', name + '.dll')), '/out:' + executable,
        ...sources, join(root, 'scripts/fixtures/CompiledDesktopTests.cs')]);
    run(join(framework, 'csc.exe'), ['/nologo', '/target:exe', '/platform:x64', '/optimize+',
        '/out:' + join(scratch, 'ui-process-fixture.exe'), join(root, 'scripts/fixtures/UiProcessFixture.cs'),
        join(root, 'toolchain/ui/windows/compiled/ProfileLease.cs'),
        join(root, 'toolchain/ui/windows/compiled/ProcessLifetime.cs')]);
    const testArgs = [root, scratch];
    if (args.length) {
        report.package = { path: resolve(args[1]), host_sha256: hash(join(resolve(args[1]), 'Sovra UI.exe')),
            application_sha256: hash(join(resolve(args[1]), 'application.exe')) };
        const app = join(scratch, 'installed app with spaces !');
        cpSync(resolve(args[1]), app, { recursive: true });
        function verifyNoScripts(directory) {
            for (const entry of readdirSync(directory, { withFileTypes: true })) {
                if (entry.isDirectory()) verifyNoScripts(join(directory, entry.name));
                else assert.doesNotMatch(entry.name, /\.(ps1|psm1)$/i);
            }
        }
        verifyNoScripts(app);
        const profile = JSON.parse(readFileSync(join(app, 'app.json'), 'utf8'));
        profile.id = 'compiled-test-' + scratch.split(/[/\\]/).at(-1).toLowerCase();
        writeFileSync(join(app, 'app.json'), JSON.stringify(profile));
        testArgs.push(app, profile.id);
    }
    const output = run(executable, testArgs).trim();
    const total = /^PASS compiled desktop: (\d+) checks;/m.exec(output);
    assert.ok(total, 'Compiled harness did not report completed checks');
    report.checks = Number(total[1]);
    console.log(output);
} catch (error) {
    failure = error;
} finally {
    try {
        rmSync(scratch, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
    } catch (error) { failure ??= error; }
    report.status = failure ? 'failed' : 'passed';
    report.success = !failure;
    if (failure) report.failure = failure.stack || String(failure);
    report.finished_at = new Date().toISOString();
    writeFileSync(evidence, JSON.stringify(report, null, 2) + '\n');
}
if (failure) throw failure;
