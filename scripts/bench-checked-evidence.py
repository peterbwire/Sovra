"""Compare opt-in evidence with legacy lowering; no input programs are executed."""
import argparse
import hashlib
import json
import platform
import os
import statistics
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--probe', type=Path, default=ROOT / ('target/release/examples/checked_evidence_probe.exe' if os.name == 'nt' else 'target/release/examples/checked_evidence_probe'))
parser.add_argument('--samples', type=int, choices=range(1, 31), default=5)
parser.add_argument('--profile', choices=('generous', 'candidate'), default='generous')
parser.add_argument('--report', type=Path, default=ROOT / 'target/benchmarks/checked-evidence.json')
args = parser.parse_args()
probe = args.probe.resolve()
sha = lambda data: hashlib.sha256(data).hexdigest()

def command(argv):
    return subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=40, check=True).stdout

def fingerprint(path):
    return {'path': str(path.relative_to(ROOT)), 'sha256': sha(path.read_bytes())}

manifest_path = ROOT / 'benchmarks/compiler-runtime/corpus.json'
manifest = json.loads(manifest_path.read_text())
measurements = []
report = {
    'schema': 1, 'success': False, 'recorded_utc': datetime.now(timezone.utc).isoformat(),
    'corpus_version': 3, 'evidence_profile': args.profile, 'base_corpus_version': manifest['version'], 'samples_per_mode': args.samples, 'warmups_per_mode': 1,
    'machine': {'system': platform.platform(), 'architecture': platform.machine(), 'python': sys.version},
    'revision': command(['git', 'rev-parse', 'HEAD']).strip(),
    'working_tree': command(['git', 'status', '--porcelain']),
    'probe_sha256': sha(probe.read_bytes()),
    'runner': fingerprint(Path(__file__).resolve()),
    'helper': fingerprint(ROOT / 'scripts/benchmark-process.py'),
    'rustc': command(['rustc', '--version']).strip(),
    'corpus': fingerprint(manifest_path),
    'source_fingerprints': [fingerprint(p) for p in sorted((ROOT / 'src').rglob('*.rs'))] +
                           [fingerprint(ROOT / name) for name in ('examples/checked_evidence_probe.rs', 'Cargo.toml', 'Cargo.lock')],
    'method': 'One fresh child per sample; alternating legacy/checked order, one warmup per mode. POSIX child peak RSS via benchmark-process.py (process-launch/inherited RSS floor can mask small differences); null if unavailable. Includes parsing, checking, lowering, process startup and debug IR serialization. AST-only standalone snapshots; no retained source copy. Package path includes resolver/interface work. Not isolated stage or allocator measurements, total allocation counts, hard memory bounds, cold-cache results, or cross-platform qualification. Probe limits are experimental, not production defaults.',
    'measurements': measurements,
}
with tempfile.TemporaryDirectory(prefix='sovra-evidence-bench-') as work:
    cases = []
    for case in manifest['cases']:
        source = (ROOT / 'benchmarks/compiler-runtime' / case['source']).read_text() if 'source' in case else ''.join(
            f'fn value_{i}(x: Int) -> Int {{ return x + {i} }}\n' for i in range(case['declarations'])) + f"fn main() {{ print(value_{case['declarations'] - 1}(1)) }}\n"
        path = Path(work) / (case['id'] + '.svr')
        path.write_text(source)
        cases.append((case['id'], 'source', path, sha(source.encode())))
    # Additional retained-name/context pressure, independent shallow declarations.
    for size in (1024, 4096):
        source = ''.join(f'fn f_{i}(x: Float) -> Float {{ let y = [x, {i}]; return y[0]; }}\n' for i in range(size)) + 'fn main() { print(f_0(1)); }\n'
        path = Path(work) / f'contexts-{size}.svr'
        path.write_text(source)
        cases.append((f'contexts-{size}', 'source', path, sha(source.encode())))
    shapes = {
        'inferred-array-depth-192': 'fn main() { let a0 = 1; ' + ''.join(f'let a{i} = [a{i-1}]; ' for i in range(1, 193)) + '}',
        'nominal-types-512': ''.join(f'struct T{i} {{ value: Float }}\n' for i in range(512)) + 'fn main() { ' + ''.join(f'let v{i} = T{i} {{ value: {i} }}; ' for i in range(512)) + '}',
        'long-binding-names-256': 'fn main() { ' + ''.join(f'let local_{i}_' + 'x' * 1000 + f' = {i}; ' for i in range(256)) + '}',
        'wide-call-contexts-64': 'fn f(' + ', '.join(f'p{i}: Float' for i in range(64)) + ') -> Float { return p0; } fn main() { ' + ''.join('print(f(' + ', '.join(str(i) for i in range(64)) + ')); ' for _ in range(64)) + '}',
    }
    for name, source in shapes.items():
        path = Path(work) / (name + '.svr')
        path.write_text(source)
        cases.append((name, 'source', path, sha(source.encode())))
    combined = ''.join(f'struct R{i} {{ value: Float }} fn make{i}(x: Float) -> R{i} {{ let v = R{i} {{ value: x }}; return v; }}\n' for i in range(16384)) + 'fn main() { print(make0(1).value); }'
    path = Path(work) / 'combined-nominal-contexts.svr'
    path.write_text(combined)
    cases.append(('combined-nominal-contexts-16384', 'source', path, sha(combined.encode())))
    package = ROOT / 'examples/local-packages/app' 
    package_inputs = [fingerprint(p) for p in sorted((ROOT / 'examples/local-packages').rglob('*')) if p.is_file()]
    report['package_inputs'] = package_inputs
    cases.append(('local-packages', 'package', package, sha(json.dumps(package_inputs, sort_keys=True).encode())))
    # Repeated attached interfaces through aliases pressure consumer metadata.
    graph = Path(work) / 'aliases'
    (graph / 'app').mkdir(parents=True)
    (graph / 'lib').mkdir()
    files = {
        'lib/sovra.toml': '[project]\nname = "provider"\nentry = "main.svr"\n',
        'lib/main.svr': 'mod api {\n' + ''.join(f'export fn f_{i}(x: Float) -> Float {{ return x + {i}; }}\n' for i in range(128)) + '}\nfn main() {}\n',
        'app/sovra.toml': '[project]\nname = "consumer"\nentry = "main.svr"\n' + ''.join(f'[dependencies.a{i}]\npath = "../lib"\n' for i in range(16)),
        'app/main.svr': ''.join(f'use a{i}::api;\n' for i in range(16)) + 'fn main() {\n' + ''.join(f'print(a{i}::api::f_0(1));\n' for i in range(16)) + '}\n',
    }
    for name, source in files.items():
        (graph / name).write_text(source)
    report['generated_package_inputs'] = {name: sha(source.encode()) for name, source in files.items()}
    cases.append(('package-aliases-16-exports-128', 'package', graph / 'app', sha(json.dumps(files, sort_keys=True).encode())))
    diamond = Path(work) / 'diamond'
    diamond_files = {
        'base/sovra.toml': '[project]\nname = "base"\nentry = "main.svr"\n',
        'base/main.svr': 'mod api { export fn value(x: Float) -> Float { return x; } } fn main() {}',
        'app/sovra.toml': '[project]\nname = "app"\nentry = "main.svr"\n' + ''.join(f'[dependencies.b{i}]\npath = "../b{i}"\n' for i in range(8)),
        'app/main.svr': ''.join(f'use b{i}::api;\n' for i in range(8)) + 'fn main() { ' + ''.join(f'print(b{i}::api::f0(1)); ' for i in range(8)) + '}',
    }
    for i in range(8):
        diamond_files[f'b{i}/sovra.toml'] = f'[project]\nname = "branch{i}"\nentry = "main.svr"\n[dependencies.base]\npath = "../base"\n'
        diamond_files[f'b{i}/main.svr'] = 'use base::api; mod api { ' + ''.join(f'export fn f{j}(x: Float) -> Float {{ return base::api::value(x); }} ' for j in range(32)) + '} fn main() {}'
    for name, source in diamond_files.items():
        path = diamond / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(source)
    report['diamond_package_inputs'] = {name: sha(source.encode()) for name, source in diamond_files.items()}
    cases.append(('package-diamond-8-branches-32-exports', 'package', diamond / 'app', sha(json.dumps(diamond_files, sort_keys=True).encode())))
    large = Path(work) / 'large-graph'
    large_files = {
        'app/sovra.toml': '[project]\nname = "app"\nentry = "main.svr"\n' + ''.join(f'[dependencies.p{i}]\npath = "../p{i}"\n' for i in range(4)),
        'app/main.svr': ''.join(f'use p{i}::api;\n' for i in range(4)) + 'fn main() { ' + ''.join(f'print(p{i}::api::entry(1)); ' for i in range(4)) + '}',
    }
    for i in range(4):
        large_files[f'p{i}/sovra.toml'] = f'[project]\nname = "provider{i}"\nentry = "main.svr"\n'
        large_files[f'p{i}/main.svr'] = 'mod api { export fn entry(x: Float) -> Float { return x; } } ' + ''.join(f'fn f{j}(x: Float) -> Float {{ let y = [x, {j}]; return y[0]; }}\n' for j in range(8192)) + 'fn main() {}'
    for name, source in large_files.items():
        path = large / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(source)
    report['large_package_inputs'] = {name: sha(source.encode()) for name, source in large_files.items()}
    cases.append(('package-4-providers-8192-functions', 'package', large / 'app', sha(json.dumps(large_files, sort_keys=True).encode())))
    for name, kind, path, input_hash in cases:
        samples = {'legacy': [], 'checked': []}
        ir_hash = None
        evidence = None
        for iteration in range(-1, args.samples):
            for mode in (('legacy', 'checked') if iteration % 2 else ('checked', 'legacy')):
                result = json.loads(command([sys.executable, str(ROOT / 'scripts/benchmark-process.py'), str(probe), ('candidate' if mode == 'checked' and args.profile == 'candidate' else mode), kind, str(path)]))
                if result['status'] != 0:
                    raise RuntimeError(f"{name}/{mode}: {result['stderr']}")
                digest = sha(result['stdout'].encode())
                if ir_hash is not None and digest != ir_hash:
                    raise RuntimeError(f'{name}: exact IR parity failed')
                ir_hash = digest
                if mode == 'checked':
                    if evidence is not None and evidence != result['stderr']:
                        raise RuntimeError(f'{name}: nondeterministic evidence counters')
                    evidence = result['stderr']
                elif result['stderr']:
                    raise RuntimeError(result['stderr'])
                if iteration >= 0:
                    samples[mode].append({key: result[key] for key in ('wall_ms', 'peak_rss_bytes')})
        summary = {}
        for mode, values in samples.items():
            rss = [v['peak_rss_bytes'] for v in values if v['peak_rss_bytes'] is not None]
            summary[mode] = {'median_wall_ms': statistics.median(v['wall_ms'] for v in values), 'median_peak_rss_bytes': statistics.median(rss) if rss else None}
        measurements.append({'case': name, 'input_sha256': input_hash, 'ir_sha256': ir_hash, 'evidence_counters': evidence.strip(), 'samples': samples, 'summary': summary})
        print(name, json.dumps(summary), flush=True)
report['success'] = True
args.report.parent.mkdir(parents=True, exist_ok=True)
args.report.write_text(json.dumps(report, indent=2) + '\n')
print(f'PASS exact IR parity for {len(measurements)} cases; report {args.report}')
