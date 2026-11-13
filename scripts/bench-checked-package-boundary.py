"""Manual aggregate AST/IR boundary qualification. Compiles; never executes input."""
import argparse
import datetime
import hashlib
import json
import os
import platform
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--probe', type=Path, default=ROOT / ('target/release/examples/checked_package_boundary.exe' if os.name == 'nt' else 'target/release/examples/checked_package_boundary'))
parser.add_argument('--report', type=Path, default=ROOT / 'target/benchmarks/checked-package-boundary.json')
args = parser.parse_args()
sha = lambda data: hashlib.sha256(data).hexdigest()
probe = args.probe.resolve()
report = {'schema': 1, 'corpus_version': 2, 'success': False,
          'recorded_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
          'platform': platform.platform(), 'python': sys.version,
          'rustc': subprocess.check_output(['rustc', '--version'], text=True).strip(),
          'revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
          'working_tree': subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT, text=True),
          'probe_sha256': sha(probe.read_bytes()),
          'runner_sha256': sha(Path(__file__).read_bytes()),
          'source_hashes': {str(p.relative_to(ROOT)): sha(p.read_bytes()) for p in sorted((ROOT / 'src').rglob('*.rs'))},
          'probe_source_sha256': sha((ROOT / 'examples/checked_package_boundary.rs').read_bytes()),
          'helper_sha256': sha((ROOT / 'scripts/benchmark-process.py').read_bytes()),
          'method': 'One fresh release child per case/mode; no warmups or timing inference. Includes resolution, parsing, checking, lowering and debug IR serialization. Child peak RSS is not allocation accounting, process-tree sum or a hard bound. Inputs never execute. Candidate evidence limits; existing compilation admission limits unchanged.',
          'cases': []}
with tempfile.TemporaryDirectory(prefix='sovra-graph-boundary-') as temporary:
    work = Path(temporary)
    files = {'app/sovra.toml': '[project]\nname = "app"\nentry = "main.svr"\n' + ''.join(f'[dependencies.p{i}]\npath = "../p{i}"\n' for i in range(4)),
             'app/main.svr': 'fn helper() {} fn main() {1;}'}
    for i in range(4):
        files[f'p{i}/sovra.toml'] = f'[project]\nname = "provider{i}"\nentry = "main.svr"\n'
        files[f'p{i}/main.svr'] = 'fn main() {' + '1;' * 124999 + '}'
    for name, source in files.items():
        path = work / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(source)
    report['input_hashes'] = {name: sha(source.encode()) for name, source in files.items()}

    def run(case, instruction_limit, byte_limit, expected, modes=('legacy', 'checked')):
        digest = None
        counters = None
        for mode in modes:
            raw = subprocess.run([sys.executable, str(ROOT / 'scripts/benchmark-process.py'), str(probe), mode, str(work / 'app'), str(instruction_limit), str(byte_limit), expected], cwd=ROOT, capture_output=True, text=True, timeout=40, check=True)
            result = json.loads(raw.stdout)
            if result['status'] != 0:
                raise RuntimeError(result['stderr'])
            observed = sha(result.pop('stdout').encode())
            if digest is not None and observed != digest:
                raise RuntimeError(f'{case}: IR/diagnostic parity failed')
            digest = observed
            if expected == 'ok':
                value = json.loads(result.pop('stderr'))
                if counters is not None and counters != value:
                    raise RuntimeError('counter mismatch')
                counters = value
            elif result.pop('stderr'):
                raise RuntimeError('unexpected stderr')
            report['cases'].append({'case': case, 'mode': mode, 'expected': expected, 'output_sha256': digest, 'counters': counters, **result})
            print(case, mode, result['peak_rss_bytes'], flush=True)
        return counters

    measured = run('exact-aggregate-ast', 4_000_000, 256 * 1024 * 1024, 'ok')
    assert measured['instructions'] == 999994
    run('exact-linked-ir', measured['instructions'], measured['string_bytes'], 'ok')
    run('instruction-overflow', measured['instructions'] - 1, measured['string_bytes'], 'E3021')
    run('string-overflow', measured['instructions'], measured['string_bytes'] - 1, 'E3022')
    run('aggregate-metadata-overflow', 4_000_000, 256 * 1024 * 1024, 'E3025', modes=('checked-tight',))
    # Original four providers plus root are exactly 1,000,000 AST nodes.
    with (work / 'app/main.svr').open('a') as output:
        output.write('fn excess() {}')
    report['overflow_root_sha256'] = sha((work / 'app/main.svr').read_bytes())
    run('aggregate-ast-overflow', 4_000_000, 256 * 1024 * 1024, 'E3020')
    # Active graph type admission: four semantically valid providers each
    # contribute 65,537 interned types, exceeding the cumulative 262,144 ceiling.
    count = 65_536
    typed = ''.join(f'struct T{i} {{}}' for i in range(count)) + 'fn f(' + ','.join(f'p{i}: T{i}' for i in range(count)) + ') {} fn main() {}'
    (work / 'app/main.svr').write_text('fn main() {}')
    for i in range(4):
        (work / f'p{i}/main.svr').write_text(typed)
    report['active_type_input_hashes'] = {str(p.relative_to(work)): sha(p.read_bytes()) for p in sorted(work.rglob('*')) if p.is_file()}
    run('active-types-legacy-acceptance', 4_000_000, 256 * 1024 * 1024, 'ok', modes=('legacy',))
    run('active-types-checked-rejection', 4_000_000, 256 * 1024 * 1024, 'E3025', modes=('checked',))
report['success'] = True
args.report.parent.mkdir(parents=True, exist_ok=True)
args.report.write_text(json.dumps(report, indent=2) + '\n')
print('PASS parity boundaries, cumulative metadata exhaustion and active type admission; report', args.report)
