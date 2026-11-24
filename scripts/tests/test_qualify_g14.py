"""Qualification must reject command failures, source drift and silent target skips."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'qualify-g14.py'
SPEC = importlib.util.spec_from_file_location('qualify_g14', SCRIPT)
Q = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(Q)


class QualificationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        subprocess.run(['git', 'init', '-q', str(self.root)], check=True)
        (self.root / 'source.svr').write_text('original')
        (self.root / '.gitignore').write_text('evidence/\n')
        self.out = self.root / 'evidence' / 'run'

    def run_case(self, commands, timeout=10):
        report = Q.qualify(self.root, self.out, commands, dict(os.environ), timeout)
        self.assertEqual(report, json.loads((self.out / 'verification.json').read_text()))
        return report

    def test_success_retains_output_and_frozen_source(self):
        report = self.run_case([('probe', [sys.executable, '-c', 'print("evidence")'])])
        self.assertTrue(report['success'])
        self.assertEqual(report['source_sha256'], report['source_sha256_end'])
        self.assertIn('evidence', (self.out / 'probe.log').read_text())
        with self.assertRaises(FileExistsError):
            Q.qualify(self.root, self.out, [], dict(os.environ))

    def test_failure_stops_and_retains_exit_code(self):
        report = self.run_case([('bad', [sys.executable, '-c', 'raise SystemExit(7)']),
                                ('never', [sys.executable, '-c', 'print("unexpected")'])])
        self.assertFalse(report['success'])
        self.assertEqual(report['runs'][0]['exit_code'], 7)
        self.assertFalse((self.out / 'never.log').exists())

    def test_missing_command_is_retained(self):
        report = self.run_case([('missing', [str(self.root / 'not-a-command')])])
        self.assertFalse(report['success'])
        self.assertEqual(report['runs'][0]['exit_code'], 127)

    def test_source_edit_during_command_invalidates_success(self):
        report = self.run_case([('edit', [sys.executable, '-c', 'from pathlib import Path;Path("source.svr").write_text("changed")'])])
        self.assertFalse(report['success'])
        self.assertIn('source changed', report['failure'])

    def test_new_nonignored_inputs_change_fingerprint(self):
        original = Q.fingerprint(self.root)
        (self.root / 'new.svr').write_text('new input')
        self.assertNotEqual(original, Q.fingerprint(self.root))

    def test_timeout_retains_failure(self):
        report = self.run_case([('timeout', [sys.executable, '-c', 'import time;time.sleep(2)'])], timeout=.05)
        self.assertFalse(report['success'])
        self.assertEqual(report['runs'][0]['exit_code'], 124)
        self.assertEqual(report['runs'][0]['error'], 'timeout')

    def test_rust_target_coverage_rejects_empty_skipped_or_missing(self):
        rows = {name: dict(passed=1, failed=0, ignored=0, filtered=0) for name in ['lib', *Q.TARGETS]}
        self.assertTrue(Q.validate_rust(rows))
        for key in ['passed', 'ignored', 'filtered', 'failed']:
            changed = {name: dict(row) for name, row in rows.items()}
            changed['http_server_v4'][key] = 0 if key == 'passed' else 1
            self.assertFalse(Q.validate_rust(changed))
        del rows['http_server_v4']
        self.assertFalse(Q.validate_rust(rows))

    def test_rust_parser_counts_targets_including_failures_and_rejects_duplicates(self):
        text = 'Running unittests src/lib.rs (target/test)\ntest result: ok. 600 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\nRunning tests/http_server_v4.rs (target/test)\ntest result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out\n'
        parsed = Q.rust_results(text)
        self.assertEqual(parsed['lib']['passed'], 600)
        self.assertEqual(parsed['http_server_v4']['failed'], 1)
        with self.assertRaises(ValueError):
            Q.rust_results(text + text)
        report = self.run_case([('rust', [sys.executable, '-c', 'print("test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out")'])])
        self.assertFalse(report['success'])


if __name__ == '__main__':
    unittest.main()
