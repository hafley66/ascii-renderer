"""Low-load guard checks. No renderer or GUI stress is launched."""
import importlib.util
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock

path = pathlib.Path(__file__).with_name('5_probe_guard.py')
spec = importlib.util.spec_from_file_location('guard', path)
guard = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guard)


class GuardTests(unittest.TestCase):
    def test_artifact_rename_retries_scan_but_persistent_failure_stops(self):
        with tempfile.TemporaryDirectory() as temp:
            stable = pathlib.Path(temp) / 'stats.json'
            stable.write_bytes(b'12345')
            missing = pathlib.Path(temp) / 'stats.tmp'
            directory = Mock()
            directory.rglob.side_effect = [[missing], [stable]]
            self.assertEqual(guard.artifact_size(directory), 5)
            self.assertEqual(directory.rglob.call_count, 2)
            directory.rglob.side_effect = [[missing], [missing]]
            with self.assertRaises(FileNotFoundError):
                guard.artifact_size(directory)

    def test_limits_and_descendants(self):
        good = dict(elapsed=0, max_seconds=15, owned_kib=10, max_owned_kib=100,
                    watched_kib=10, baseline_kib=10, max_watched_kib=100,
                    max_growth_kib=20, artifact_bytes=1, max_artifact_bytes=100,
                    free_bytes=100, min_free_bytes=50, owner_alive=True)
        self.assertIsNone(guard.violation(**good))
        for changes, expected in [({'elapsed': 15}, 'wall_time'),
                                  ({'owned_kib': 100}, 'owned_rss'),
                                  ({'watched_kib': 100}, 'watched_rss'),
                                  ({'watched_kib': 30}, 'watched_rss_growth'),
                                  ({'artifact_bytes': 100}, 'artifact_bytes'),
                                  ({'free_bytes': 49}, 'disk_free'),
                                  ({'owner_alive': False}, 'observer_owner_exited')]:
            with self.subTest(expected=expected):
                self.assertEqual(guard.violation(**(good | changes)), expected)
        self.assertEqual(guard.descendants({10: (1, 0), 11: (10, 0), 12: (11, 0),
                                           15: (1, 0)}, 10), {10, 11, 12})

    def test_wall_time_kills_sleeping_child_and_grandchild(self):
        with tempfile.TemporaryDirectory() as temp:
            log = pathlib.Path(temp) / 'guard.ndjson'
            pidfile = pathlib.Path(temp) / 'grandchild'
            program = ('import subprocess,sys,time,pathlib; '
                       'p=subprocess.Popen([sys.executable,"-c","import time;time.sleep(60)"]); '
                       f'pathlib.Path({str(pidfile)!r}).write_text(str(p.pid)); time.sleep(60)')
            result = subprocess.run([sys.executable, str(path), '--state', str(log),
                '--artifact-dir', temp, '--max-seconds', '1', '--', sys.executable,
                '-c', program], timeout=5, capture_output=True, text=True)
            self.assertEqual(result.returncode, 124, result.stderr)
            events = [json.loads(line) for line in log.read_text().splitlines()]
            breaker = next(r for r in events if r['kind'] == 'breaker')
            stopped = next(r for r in events if r['kind'] == 'stopped')
            self.assertEqual(breaker['reason'], 'wall_time')
            grandchild = int(pidfile.read_text())
            self.assertIn(grandchild, stopped['owned_pids'])
            self.assertNotIn(os.getpid(), stopped['owned_pids'])
            stat = subprocess.run(['ps', '-p', str(grandchild), '-o', 'stat='],
                                  capture_output=True, text=True).stdout.strip()
            self.assertTrue(not stat or stat.startswith('Z'), stat)

    def test_rss_breaker_stops_small_python_child(self):
        with tempfile.TemporaryDirectory() as temp:
            log = pathlib.Path(temp) / 'guard.ndjson'
            result = subprocess.run([sys.executable, str(path), '--state', str(log),
                '--artifact-dir', temp, '--max-owned-mib', '1', '--', sys.executable,
                '-c', 'import time;time.sleep(60)'], timeout=5,
                capture_output=True, text=True)
            self.assertEqual(result.returncode, 124, result.stderr)
            events = [json.loads(line) for line in log.read_text().splitlines()]
            self.assertEqual(next(r['reason'] for r in events if r['kind'] == 'breaker'),
                             'owned_rss')
            self.assertEqual(events[-1]['kind'], 'stopped')


if __name__ == '__main__':
    unittest.main()
