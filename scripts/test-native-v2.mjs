// Actual source-backed v2 native build, relocation and closed-host acceptance.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
const compiler = resolve(process.env.SOVRA_TEST_COMPILER || 'target/debug/svr');
const scratch = mkdtempSync(join(tmpdir(), 'sovra native v2 '));
const artifacts = resolve('target/checked-evidence'); mkdirSync(artifacts, { recursive: true });
const hash = file => createHash('sha256').update(readFileSync(file)).digest('hex');
const run = (binary, args, options = {}) => {
    const result = spawnSync(binary, args, { cwd: scratch, encoding: 'utf8', timeout: 30000, maxBuffer: 2 * 1024 * 1024, ...options });
    assert.ifError(result.error); assert.equal(result.signal, null); return result;
};
const success = result => { assert.equal(result.status, 0, result.stderr); return result; };
const source = `mod data{enum Outcome{Value(Int),Empty}type Result=Outcome;fn hidden()->Result{return Result::Value(7);}export fn obtain()->Result{return data::hidden();}}
type Alias=data::Result;
fn obtain()->Alias{return data::obtain();}
fn main(){
 if std::arg_count()>0{
  let mode=std::arg(0);
  if mode=="loop"{while true{}return;}
  if mode=="fail"{print("pending");let zero=0;print(1/zero);return;}
 }
 let result=obtain();
 match result{data::Result::Value(n)=>{print(n);},Alias::Empty()=>{print(0);}}
 print(std::arg_count());if std::arg_count()>0{print(std::arg(0));}else{print("");}std::set_exit_code(7);
}`;
try {
    const project = join(scratch, 'project'); mkdirSync(project);
    writeFileSync(join(project, 'sovra.toml'), '[project]\nname="native"\nentry="main.svr"\n');
    writeFileSync(join(project, 'main.svr'), source);
    for (const args of [[], ['λ😀']]) {
        const interpreted = run(compiler, ['run', '--experimental-v2', join(project, 'main.svr'), '--', ...args]);
        assert.equal(interpreted.status, 7, interpreted.stderr);
        assert.equal(interpreted.stdout, `7\n${args.length}\n${args[0] || ''}\n`);
    }
    success(run(compiler, ['lock', '--experimental-v2', project]));
    const executable = join(scratch, process.platform === 'win32' ? 'application.exe' : 'application');
    success(run(compiler, ['build', '--experimental-v2', '--locked', '--emit', 'exe', '--output', executable, project], { timeout: 600000 }));
    const digest = hash(executable);
    const existing = run(compiler, ['build', '--experimental-v2', '--emit', 'exe', '--output', executable, project]);
    assert.equal(existing.status, 1); assert.match(existing.stderr, /already exists/); assert.equal(hash(executable), digest);
    writeFileSync(join(project, 'main.svr'), source + '\n// changed');
    const staleOutput = join(scratch, 'must-not-exist');
    const stale = run(compiler, ['build', '--experimental-v2', '--locked', '--emit', 'exe', '--output', staleOutput, project]);
    assert.equal(stale.status, 1); assert.equal(existsSync(staleOutput), false);
    const deniedSource = join(scratch, 'denied.svr');
    writeFileSync(deniedSource, 'fn main(){print("pending");std::write_text("marker","bad");let line=std::read_line();print(line.text);}');
    const deniedExecutable = join(scratch, process.platform === 'win32' ? 'denied.exe' : 'denied');
    success(run(compiler, ['build', '--experimental-v2', '--emit', 'exe', '--output', deniedExecutable, deniedSource], { timeout: 600000 }));
    rmSync(deniedSource);
    const moved = join(scratch, 'relocated'); mkdirSync(moved);
    const relocated = join(moved, process.platform === 'win32' ? 'application.exe' : 'application'); copyFileSync(executable, relocated);
    if (process.platform !== 'win32') { const { chmodSync } = await import('node:fs'); chmodSync(relocated, 0o700); }
    rmSync(project, { recursive: true }); rmSync(executable);
    const env = { ...process.env }; for (const key of Object.keys(env)) if (key.toLowerCase() === 'path') delete env[key];
    env.PATH = process.platform === 'win32' ? `${process.env.SystemRoot}\\System32` : '';
    const cases = [];
    for (const args of [[], ['λ😀'], ['loop'], ['fail']]) {
        const result = run(relocated, args, { cwd: moved, env });
        if (!args.length || args[0] === 'λ😀') {
            assert.equal(result.status, 7, result.stderr); assert.equal(result.stdout, `7\n${args.length}\n${args[0] || ''}\n`); assert.equal(result.stderr, '');
        } else {
            assert.equal(result.status, 1); assert.equal(result.stdout, ''); assert.ok(result.stderr.length > 0);
            if (args[0] === 'loop') assert.match(result.stderr, /Instruction/i);
        }
        cases.push({ arguments: args, status: result.status, output: result.stdout, diagnostic: result.stderr });
    }
    const denied = run(deniedExecutable, [], { cwd: moved, env });
    assert.equal(denied.status, 1); assert.equal(denied.stdout, ''); assert.match(denied.stderr, /UnsupportedStandard/);
    cases.push({ arguments: [], status: denied.status, output: denied.stdout, diagnostic: denied.stderr, program: 'denied-hosts' });
    assert.equal(existsSync(join(moved, 'marker')), false);
    assert.deepEqual(readdirSync(scratch).filter(name => name.startsWith('.sovra-exe-stage-')), []);
    writeFileSync(join(artifacts, 'g13-v2-module-function-native-host-verification.json'), JSON.stringify({ success: true, platform: process.platform, architecture: process.arch, compiler_sha256: hash(compiler), executable_sha256: digest, representation: 'bounded source image, rechecked and lowered at startup', checks: ['locked package build', 'no-overwrite publication', 'stale lock refusal', 'relocation after source removal', 'empty PATH', 'UTF-8 arguments/status', 'transparent enum alias constructors/patterns', 'inline type module context', 'qualified exported calls and private helpers', 'no file/stdin grants', 'finite instruction refusal', 'success-only output', 'staging cleanup'], cases }, null, 2) + '\n');
    console.log('V2 native host passed: locked build, relocation without source/PATH tools, enum calls, arguments/status, no host grants, finite fuel, no overwrite, stale locks and cleanup.');
} finally { rmSync(scratch, { recursive: true, force: true }); }
