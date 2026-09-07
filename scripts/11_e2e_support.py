"""Bounded trace reading and explicit pass/fail evidence for the terminal E2E suite."""
import gzip
import hashlib
import json
import math
from pathlib import Path


def write_json(path, value):
    path = Path(path)
    temp = path.with_suffix('.tmp')
    temp.write_text(json.dumps(value, indent=2) + '\n')
    temp.replace(path)


class Trace:
    """Read each complete NDJSON record once; retain incomplete writes for the next poll."""
    def __init__(self, path):
        self.path = Path(path)
        self.offset = 0
        self.pending = b''
        self.rows = []

    def read(self):
        if not self.path.exists():
            return self.rows
        with self.path.open('rb') as file:
            file.seek(self.offset)
            chunk = file.read(4 * 1024 * 1024 + 1)
            if len(chunk) > 4 * 1024 * 1024 or self.offset + len(chunk) > 16 * 1024 * 1024:
                raise AssertionError('trace exceeded bounded reader budget')
            self.offset += len(chunk)
        lines = (self.pending + chunk).split(b'\n')
        self.pending = lines.pop()
        self.rows.extend(json.loads(line) for line in lines if line)
        return self.rows


def frame_summary(frames):
    result = {'frames': len(frames)}
    for field in ['render_us', 'encoding_us', 'presentation_us', 'dur_us', 'interval_ms', 'bytes']:
        values = sorted(f[field] for f in frames)
        result[field] = None if not values else {
            'p50': values[(len(values)-1)//2],
            'p95': values[math.ceil(.95*len(values))-1], 'max': values[-1]}
    return result


def profile_summary(path):
    # Cap decompression as well as the compressed artifact's watchdog budget.
    with gzip.open(path, 'rb') as file:
        data = file.read(64*1024*1024+1)
    if len(data) > 64*1024*1024:
        raise AssertionError('profile exceeds 64 MiB decompression budget')
    profile = json.loads(data)
    threads = [{'pid': t['pid'], 'tid': t['tid'], 'name': t['name'],
                'samples': t['samples']['length']} for t in profile['threads']]
    return {'threads': threads, 'samples': sum(t['samples'] for t in threads)}


def binary_identity(binary, root):
    import subprocess
    def git(*args):
        return subprocess.check_output(['git', *args], cwd=root, text=True).strip()
    return {'path': str(binary), 'sha256': hashlib.file_digest(open(binary, 'rb'), 'sha256').hexdigest(),
            'commit': git('rev-parse', 'HEAD'), 'status': git('status', '--short'),
            'diff_sha256': hashlib.sha256(git('diff').encode()).hexdigest()}
