#!/usr/bin/env python3
"""Retain frozen-source Linux G14 evidence. Does not establish production qualification."""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import signal
import stat
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
TARGETS = [
    'network_v4', 'network_host_v4', 'network_policy_v4', 'network_cli_v4',
    'http_request_v4', 'http_response_v4', 'http_chunked_v4', 'http_trailers_v4',
    'http_access_v4', 'http_admission_v4', 'http_authentication_v4', 'http_command_authentication_v4', 'http_authenticated_server_v4', 'http_exchange_v4', 'http_server_v4', 'host_v3', 'generic_data_cli_v2',
]
NODE = ['test-compiler-security', 'test-language-tests', 'test-example-catalog', 'check-doc-links']
OVERRIDES = dict(CARGO_BUILD_JOBS='1', CARGO_INCREMENTAL='0',
                 CARGO_PROFILE_DEV_DEBUG='0', CARGO_PROFILE_TEST_DEBUG='0', CARGO_TERM_COLOR='never',
                 SOVRA_REQUIRE_CLI_EXECUTION='1')


MAX_SOURCE_MANIFEST_BYTES = 4 * 1024 * 1024
MAX_SOURCE_FILES = 10000
MAX_SOURCE_FILE_BYTES = 16 * 1024 * 1024
MAX_SOURCE_BYTES = 128 * 1024 * 1024


class SourceInputError(Exception):
    """The source manifest cannot establish an admitted regular-file snapshot."""


def _signature(info):
    return (info.st_dev, info.st_ino, info.st_size, info.st_mode,
            info.st_mtime_ns, info.st_ctime_ns)


def _open_source(root_fd, name):
    parts = Path(os.fsdecode(name)).parts
    if not parts or Path(os.fsdecode(name)).is_absolute() or any(p in {'.', '..'} for p in parts):
        raise SourceInputError('invalid source path')
    parent = os.dup(root_fd)
    try:
        for part in parts[:-1]:
            child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=parent)
            os.close(parent)
            parent = child
        if not stat.S_ISREG(os.stat(parts[-1], dir_fd=parent, follow_symlinks=False).st_mode):
            raise SourceInputError('source input is not a regular file')
        return os.open(parts[-1], os.O_RDONLY | os.O_NONBLOCK | os.O_NOFOLLOW, dir_fd=parent)
    finally:
        os.close(parent)


def fingerprint(root=ROOT):
    """Bounded regular-file capture through no-follow directory/file descriptors on Linux."""
    if sys.platform != 'linux':
        raise SourceInputError('source capture requires Linux')
    manifest = subprocess.Popen(
        ['git', 'ls-files', '-z', '--cached', '--others', '--exclude-standard'],
        cwd=root, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    try:
        names = manifest.stdout.read(MAX_SOURCE_MANIFEST_BYTES + 1)
        if len(names) > MAX_SOURCE_MANIFEST_BYTES:
            raise SourceInputError('source manifest byte limit exceeded')
        if manifest.wait(timeout=30) != 0:
            raise SourceInputError('source manifest requires a readable Git checkout')
    finally:
        manifest.stdout.close()
        if manifest.poll() is None:
            manifest.kill()
        manifest.wait()
    paths = sorted(set(names.split(b'\0')) - {b''})
    if len(paths) > MAX_SOURCE_FILES:
        raise SourceInputError('source file-count limit exceeded')
    digest = hashlib.sha256()
    total = 0
    root_fd = os.open(root, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        for name in paths:
            descriptor = _open_source(root_fd, name)
            with os.fdopen(descriptor, 'rb') as stream:
                before = os.fstat(stream.fileno())
                if not stat.S_ISREG(before.st_mode):
                    raise SourceInputError('source input is not a regular file')
                if before.st_size > MAX_SOURCE_FILE_BYTES or total + before.st_size > MAX_SOURCE_BYTES:
                    raise SourceInputError('source byte limit exceeded')
                digest.update(name + b'\0' + str(before.st_mode & 0o111).encode() + b'\0')
                size = 0
                for chunk in iter(lambda: stream.read(65536), b''):
                    size += len(chunk)
                    if size > MAX_SOURCE_FILE_BYTES or total + size > MAX_SOURCE_BYTES:
                        raise SourceInputError('source grew beyond byte limits')
                    digest.update(chunk)
                after = os.fstat(stream.fileno())
                if size != before.st_size or _signature(before) != _signature(after):
                    raise SourceInputError('source changed during capture')
                # Reopen through the guarded path to detect replacement/parent-link changes.
                check_fd = _open_source(root_fd, name)
                try:
                    if _signature(os.fstat(check_fd)) != _signature(after):
                        raise SourceInputError('source identity changed during capture')
                finally:
                    os.close(check_fd)
                total += size
                digest.update(b'\0')
    except OSError as exc:
        raise SourceInputError('source input could not be captured without links') from exc
    finally:
        os.close(root_fd)
    return digest.hexdigest()


def rust_results(text):
    """Keep each target's counts; missing/zero targets cannot establish runtime acceptance."""
    current = None
    results = {}
    for line in text.splitlines():
        match = re.search(r'Running (?:tests/([^ ]+)\.rs|unittests src/lib\.rs)', line)
        if match:
            current = match[1] or 'lib'
        match = re.search(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;.*? (\d+) filtered out', line)
        if match:
            if current is None or current in results:
                raise ValueError('missing or repeated Rust target identity')
            results[current] = dict(zip(['passed', 'failed', 'ignored', 'filtered'], map(int, match.groups())))
    return results


def validate_rust(results):
    expected = {'lib', *TARGETS}
    return set(results) == expected and all(
        row['passed'] > 0 and row['failed'] == row['ignored'] == row['filtered'] == 0
        for row in results.values())


def execute(command, log, cwd, env, timeout):
    """Retain launch/timeout/interrupt failures and terminate the command's process group."""
    started = time.monotonic()
    record = {'command': command, 'exit_code': None}
    process = None
    with log.open('w') as output:
        try:
            process = subprocess.Popen(command, cwd=cwd, env=env, stdout=output,
                                       stderr=subprocess.STDOUT, start_new_session=True)
            record['exit_code'] = process.wait(timeout=timeout)
        except (subprocess.TimeoutExpired, KeyboardInterrupt) as exc:
            if process is not None:
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                process.wait()
            record.update(exit_code=124 if isinstance(exc, subprocess.TimeoutExpired) else 130,
                          error='timeout' if isinstance(exc, subprocess.TimeoutExpired) else 'interrupted')
        except OSError:
            record.update(exit_code=127, error='command could not be launched')
    record['seconds'] = round(time.monotonic() - started, 3)
    return record


def qualify(root, out, commands, env, timeout=1200):
    """Write progress after every stage, failing closed on source drift or partial coverage."""
    out.mkdir(parents=True, exist_ok=False)
    report = {'success': False, 'profile': 'experimental Linux loopback HTTP subset',
              'started_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
              'platform': platform.platform(), 'python_version': platform.python_version(),
              'planned_stages': [name for name, _ in commands], 'source_sha256': None,
              'runs': [], 'external_gates': ['client authentication/identity authorization',
              'complete HTTP conformance', 'production TLS/concurrency/backpressure/deployment',
              'Windows/macOS runtime and browser/mobile support']}
    def save():
        (out / 'verification.json').write_text(json.dumps(report, indent=2) + '\n')
    save()
    try:
        report['source_sha256'] = fingerprint(root)
        save()
        for name, command in commands:
            print('RUN', name, flush=True)
            if fingerprint(root) != report['source_sha256']:
                report['failure'] = 'source changed before ' + name
                break
            record = execute(command, out / (name + '.log'), root, env, timeout)
            record['name'] = name
            report['runs'].append(record)
            record['source_sha256_end'] = fingerprint(root)
            if name == 'build-cli' and record['exit_code'] == 0:
                binary = root / 'target/debug/svr'
                report['compiler_sha256'] = hashlib.sha256(binary.read_bytes()).hexdigest()
            if name == 'rust' and record['exit_code'] == 0:
                try:
                    record['targets'] = rust_results((out / (name + '.log')).read_text(errors='replace'))
                    record['coverage_valid'] = validate_rust(record['targets'])
                except ValueError:
                    record['coverage_valid'] = False
                if not record['coverage_valid']:
                    report['failure'] = 'missing, skipped or partial Rust target coverage'
            if record['source_sha256_end'] != report['source_sha256']:
                report['failure'] = 'source changed during ' + name
            if record['exit_code'] != 0:
                report['failure'] = 'command failed: ' + name
            save()
            if 'failure' in report:
                break
            print('PASS', name, flush=True)
        else:
            report['source_sha256_end'] = fingerprint(root)
            report['success'] = report['source_sha256_end'] == report['source_sha256']
            if not report['success']:
                report['failure'] = 'source changed after final stage'
    except SourceInputError as exc:
        report['failure'] = 'source capture failed: ' + str(exc)
    except (OSError, subprocess.SubprocessError, KeyboardInterrupt) as exc:
        report['failure'] = 'qualification interrupted: ' + type(exc).__name__
    report['finished_utc'] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    report['rust_passed'] = sum(row['passed'] for run in report['runs']
                                for row in run.get('targets', {}).values())
    save()
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--windows-compile', action='store_true', help='Also compile-check installed Windows GNU target; never runtime acceptance')
    args = parser.parse_args()
    if sys.platform != 'linux' or platform.machine().lower() not in {'x86_64', 'amd64', 'aarch64', 'i386', 'i686'}:
        parser.error('G14 runtime qualification requires a supported Linux host')
    stamp = datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ')
    out = ROOT / 'target/checked-evidence/g14-qualification' / stamp
    commands = [
        ('rust-version', ['rustc', '+stable', '--version']),
        ('msrv-version', ['rustc', '+1.74.0', '--version']),
        ('node-version', ['node', '--version']),
        ('fmt', ['cargo', '+stable', 'fmt', '--all', '--', '--check']),
        ('check', ['cargo', '+stable', 'check', '--locked', '--all-targets']),
        ('clippy', ['cargo', '+stable', 'clippy', '--locked', '--all-targets', '--all-features', '--', '-D', 'warnings']),
        ('rust', ['cargo', '+stable', 'test', '--locked', '--lib',
                  *[arg for target in TARGETS for arg in ['--test', target]], '--', '--test-threads=1']),
        ('msrv', ['cargo', '+1.74.0', 'check', '--locked', '--all-targets']),
        ('build-cli', ['cargo', '+stable', 'build', '--locked', '--bin', 'svr', '--target-dir', str(ROOT / 'target')]),
    ]
    if args.windows_compile:
        commands.append(('windows-compile', ['cargo', '+stable', 'check', '--locked', '--all-targets', '--target', 'x86_64-pc-windows-gnu']))
    commands += [(name, ['node', 'scripts/' + name + '.mjs']) for name in NODE]
    report = qualify(ROOT, out, commands, {**os.environ, **OVERRIDES})
    print('Evidence:', out, flush=True)
    print('PASS' if report['success'] else 'FAIL', report.get('failure', ''), report['rust_passed'], 'Rust tests', flush=True)
    return 0 if report['success'] else 1


if __name__ == '__main__':
    sys.exit(main())
