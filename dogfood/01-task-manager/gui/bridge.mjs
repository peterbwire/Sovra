// Reusable Node host adapter for executing a Sovra source program.
import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

export function createSovraProgram({ compiler, source, engine = 'javascript', cwd = process.cwd() }) {
    if (!['javascript', 'interpreter'].includes(engine)) throw new Error('Invalid Sovra engine');
    const binary = resolve(compiler);
    const input = resolve(source);
    let directory;
    let generated;
    const cleanup = () => {
        if (directory) rmSync(directory, { recursive: true, force: true });
        directory = undefined;
    };
    if (engine === 'javascript') {
        const built = spawnSync(binary, ['build', '--emit', 'js', input], {
            cwd, encoding: 'utf8', timeout: 30000, windowsHide: true,
            maxBuffer: 16 * 1024 * 1024,
        });
        if (built.error) throw built.error;
        if (built.status !== 0) throw new Error(`Sovra JavaScript build failed: ${built.stderr}`);
        directory = mkdtempSync(join(tmpdir(), 'sovra-gui-bridge-'));
        generated = join(directory, 'program.cjs');
        try { writeFileSync(generated, built.stdout, { mode: 0o600 }); }
        catch (error) { cleanup(); throw error; }
        process.once('exit', cleanup);
    }
    let closed = false;
    return {
        engine,
        run(args = []) {
            if (closed) throw new Error('Sovra program is closed');
            if (!Array.isArray(args) || !args.every(value => typeof value === 'string')) {
                throw new TypeError('Sovra program arguments must be strings');
            }
            const executable = generated ? process.execPath : binary;
            const commandArgs = generated ? [generated, ...args] : ['run', input, '--', ...args];
            const result = spawnSync(executable, commandArgs, {
                cwd, encoding: 'utf8', timeout: 30000, windowsHide: true,
                maxBuffer: 32 * 1024 * 1024,
            });
            if (result.error) throw result.error;
            return { status: result.status, stdout: result.stdout, stderr: result.stderr };
        },
        close() {
            if (closed) return;
            closed = true;
            process.removeListener('exit', cleanup);
            cleanup();
        },
    };
}
