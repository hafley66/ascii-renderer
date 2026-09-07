"""Deterministic checks of the evidence reader and fail-closed E2E safety inputs."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(file))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

support = load('support', '11_e2e_support.py')
guard = load('guard', '5_probe_guard.py')


class EvidenceTests(unittest.TestCase):
    def test_partial_ndjson_is_read_once_and_malformed_complete_records_fail(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp)/'trace.ndjson'
            trace = support.Trace(path)
            self.assertEqual(trace.read(), [])
            path.write_bytes(b'{"frame":1}\n{"frame":')
            self.assertEqual(trace.read(), [{'frame':1}])
            self.assertEqual(trace.read(), [{'frame':1}])
            with path.open('ab') as file: file.write(b'2}\n')
            self.assertEqual(trace.read(), [{'frame':1},{'frame':2}])
            with path.open('ab') as file: file.write(b'broken\n')
            with self.assertRaises(json.JSONDecodeError): trace.read()

    def test_focus_and_stale_observer_fail_closed(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp)/'observer.json'
            self.assertEqual(guard.observer_violation(path,2000),'observer_invalid')
            for state, expected in [
                ({'ts_ms':1000,'focused':True},None),
                ({'ts_ms':499,'focused':True},'observer_stale'),
                ({'ts_ms':2001,'focused':True},'observer_stale'),
                ({'ts_ms':1000,'focused':False},'observer_focus_lost'),
                ({'ts_ms':1000,'focused':False,'error':'timeout'},'observer_failure'),
                ({'ts_ms':1000,'focused':False,'stop_reason':'cleanup'},'observer_requested_stop'),
                ({'ts_ms':1000,'focused':'true'},'observer_focus_lost'),
                ({'ts_ms':1000,'mode':'headless','transport_alive':True},None),
                ({'ts_ms':1000,'mode':'headless','transport_alive':False},'observer_transport_lost'),
                ({'ts_ms':1000,'mode':'headless'},'observer_transport_lost'),
                ({'ts_ms':499,'mode':'headless','transport_alive':True},'observer_stale'),
                ({'ts_ms':1000,'mode':'headless','transport_alive':True,'error':'reader stopped'},'observer_failure'),
                ({},'observer_invalid')]:
                support.write_json(path,state)
                self.assertEqual(guard.observer_violation(path,2000),expected)

    def test_frame_budgets_retain_outliers_and_empty_evidence(self):
        keys=['render_us','encoding_us','presentation_us','dur_us','interval_ms','bytes']
        rows=[dict.fromkeys(keys,v) for v in [10]*18+[40,10000]]
        result=support.frame_summary(rows)
        self.assertEqual(result['interval_ms'],{'p50':10,'p95':40,'max':10000})
        self.assertIsNone(support.frame_summary([])['interval_ms'])


if __name__=='__main__': unittest.main()
