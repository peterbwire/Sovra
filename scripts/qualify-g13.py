#!/usr/bin/env python3
"""Retain exact-source Linux G13 regression evidence; optional full Rust suite.

Deletes only generated test executables after successful runs to bound disk use.
No installed toolchains, source files, CLI executable, or native artifacts are removed.
"""
import argparse, hashlib, json, os, pathlib, re, stat, subprocess, time
ROOT = pathlib.Path(__file__).resolve().parents[1]
OUT = ROOT / 'target/checked-evidence/g13-closure'

def fingerprint():
    h=hashlib.sha256()
    paths=[ROOT/'Cargo.toml',ROOT/'Cargo.lock',ROOT/'.github/workflows/ci.yml']
    for folder in ['src','tests','toolchain','scripts','examples']:
        paths.extend(p for p in (ROOT/folder).rglob('*') if p.is_file())
    for p in sorted(paths):
        h.update(str(p.relative_to(ROOT)).encode());h.update(b'\0');h.update(p.read_bytes());h.update(b'\0')
    return h.hexdigest()

def clean(names):
    deps=ROOT/'target/debug/deps'
    if not deps.exists(): return
    for p in deps.iterdir():
        m=re.fullmatch(r'(.+)-[0-9a-f]{16}',p.name)
        if m and m[1] in names and p.is_file() and p.stat().st_mode & stat.S_IXUSR:
            p.unlink()

def main():
    ap=argparse.ArgumentParser();ap.add_argument('--all-rust',action='store_true');ap.add_argument('--node',action='store_true');ap.add_argument('--vscode-host',action='store_true',help='Run optional Electron host acceptance; requires @vscode/test-electron and a usable display/headless host');args=ap.parse_args()
    OUT.mkdir(parents=True,exist_ok=True)
    env=dict(os.environ,CARGO_BUILD_JOBS='1',CARGO_INCREMENTAL='0',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0')
    report={'source_sha256':fingerprint(),'platform':os.uname().sysname+' '+os.uname().machine,'started':time.time(),'runs':[],'external_host_gates':[] if args.vscode_host else ['VS Code Electron host acceptance not run; opt in with --vscode-host and provision its dependencies']}
    def run(name,cmd,extra_env=None):
        print('RUN',name,flush=True);started=time.time()
        with (OUT/(name+'.log')).open('w') as log:
            result=subprocess.run(cmd,cwd=ROOT,env={**env, **(extra_env or {})},stdout=log,stderr=subprocess.STDOUT)
        text=(OUT/(name+'.log')).read_text(errors='replace')
        counts=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored',text)
        record={'name':name,'command':cmd,'exit_code':result.returncode,'seconds':round(time.time()-started,2)}
        if extra_env:record['environment_overrides']=extra_env
        if counts:record.update(passed=int(counts[-1][0]),ignored=int(counts[-1][2]))
        report['runs'].append(record);report['source_sha256_end']=fingerprint()
        (OUT/'verification.json').write_text(json.dumps(report,indent=2)+'\n')
        print(('PASS' if result.returncode==0 else 'FAIL'),name,record.get('passed',''),flush=True)
        if result.returncode:raise SystemExit(result.returncode)
    run('fmt',['cargo','+stable','fmt','--check'])
    run('check',['cargo','+stable','check','--all-targets','--locked'])
    run('clippy',['cargo','+stable','clippy','--all-targets','--all-features','--','-D','warnings'])
    run('lib',['cargo','+stable','test','--lib','--locked']);clean({'sovra'})
    selected=['generic_package_overlays','generic_package_context','cli_generic_fmt','cli_v2_fmt','package_capture_v2','generic_data_types_v2','runtime_json_v2','host_v3','generic_data_cli_v2','generic_collection_values_v2','runtime_bytes_v2','generic_package_native_v2','generic_runtime_v2','generic_propagation_concrete_v2']
    if args.all_rust:selected=sorted(p.stem for p in (ROOT/'tests').glob('*.rs'))
    for target in selected:
        run(target,['cargo','+stable','test','--locked','--test',target,'--','--test-threads=1'])
        clean({target.replace('-','_')})
    run('doctests',['cargo','+stable','test','--doc','--locked'])
    run('msrv',['cargo','+1.74.0','check','--all-targets','--locked'])
    run('build-cli',['cargo','+stable','build','--locked','--bin','svr'])
    if args.node:
        for name in ['test-doc-links','check-doc-links','test-compiler-security','test-language-tests','test-example-catalog','test-editor','test-editor-workspace','test-lsp','test-lsp-packages','test-lsp-navigation','test-lsp-v2','test-lsp-generics','test-task-targets','test-ide-symbols','test-ide-vsix','test-developer-tools']:
            run(name,['node','scripts/'+name+'.mjs'])
    if args.vscode_host:
        modes={'default':None,'v2':'SOVRA_TEST_V2','generics':'SOVRA_TEST_GENERICS','package':'SOVRA_TEST_PACKAGE_WORKSPACE','generic-package':'SOVRA_TEST_GENERIC_PACKAGE_WORKSPACE','untrusted':'SOVRA_TEST_UNTRUSTED'}
        for name,flag in modes.items():
            overrides={key:'0' for key in modes.values() if key}
            if flag:overrides[flag]='1'
            run('test-vscode-'+name,['node','scripts/test-vscode-extension.mjs'],overrides)
    report['finished']=time.time();report['source_sha256_end']=fingerprint();report['success']=report['source_sha256']==report['source_sha256_end'];report['rust_passed']=sum(r.get('passed',0) for r in report['runs'])
    (OUT/'verification.json').write_text(json.dumps(report,indent=2)+'\n')
    if not report['success']:raise SystemExit('Source changed during qualification; rerun affected checks and consolidate exact-source evidence.')
    print('G13 verification completed:',report['rust_passed'],'Rust tests',flush=True)
if __name__=='__main__':main()
