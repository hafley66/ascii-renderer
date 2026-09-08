"""Same Gem 2 frames through three native library renderers, under the probe guard."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import statistics
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parent.parent

def run_case(args, backend):
    directory = args.directory / backend
    directory.mkdir()
    log = directory / 'frames.ndjson'
    binary = ROOT / 'target/release/gem-render-lab'
    command = [str(binary), 'gem-lab', backend, str(args.frames), str(log),
               str(args.fixture), str(args.width), str(args.height), 'hold']
    (directory / 'command.json').write_text(json.dumps(command))
    guarded = [sys.executable, str(ROOT/'scripts/5_probe_guard.py'),
        '--state', str(directory/'guard.ndjson'), '--artifact-dir', str(directory),
        '--owner-pid', str(os.getpid()), '--max-seconds', '15', '--max-owned-mib', '256',
        '--max-artifact-mib', '32', '--', str(ROOT/'target/release/examples/1_native_terminal'),
        str(args.cols), str(args.rows), str(directory/'terminal.ansi.gz'), *command]
    env = dict(os.environ)
    env.pop('ASCII_FUNCTION_TRACE', None)
    env.pop('NO_COLOR', None)
    with (directory/'stderr.log').open('w') as err:
        child = subprocess.Popen(guarded, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                 stderr=err, text=True, env=env)
        def request(op, **fields):
            child.stdin.write(json.dumps(dict(op=op, **fields))+'\n')
            child.stdin.flush()
            line = child.stdout.readline()
            if not line:
                raise RuntimeError(f'{backend}: guard or native driver exited; inspect {directory}')
            return json.loads(line)
        try:
            assert json.loads(child.stdout.readline())['ready']
            deadline = time.monotonic() + 12
            records = []
            while time.monotonic() < deadline:
                if log.exists():
                    records = [json.loads(line)['fields'] for line in log.read_text().splitlines() if line.endswith('}')]
                    if any(r.get('kind') == 'complete' for r in records):
                        break
                status = request('status')
                assert status['alive'], status
                time.sleep(.01)
            else:
                raise TimeoutError(f'{backend}: no complete frame sequence')
            # The final frame is held while the native parser drains terminal bytes.
            before = request('status')['bytes']
            for _ in range(30):
                time.sleep(.01)
                status = request('status')
                if status['bytes'] == before:
                    break
                before = status['bytes']
            cells = request('cells', width=args.width, height=args.height)['cells']
            (directory/'cells.json').write_text(json.dumps(cells))
            assert any(c[0] != ' ' and c[1] != 'Default' for row in cells for c in row), 'missing colored art'
            request('send', text='q')
            deadline = time.monotonic() + 2
            while time.monotonic() < deadline:
                status = request('status')
                if status['reader_done']: break
                time.sleep(.01)
            assert status['reader_done'] and status['exit_code'] == 0 and status['error'] is None, status
            child.stdin.write('{"op":"quit"}\n'); child.stdin.flush()
            assert child.wait(timeout=2) == 0
        finally:
            if child.poll() is None:
                child.terminate()
                try: child.wait(timeout=3)
                except subprocess.TimeoutExpired: child.kill(); child.wait()
    frames = [r for r in records if r.get('kind') == 'frame']
    assert len(frames) == args.frames
    source = json.loads(log.with_suffix('.grid').read_text())
    def color(value):
        return 'Default' if value is None else f'Idx({value})'
    expected = [[[ch, 'Default' if ch == ' ' else color(fg), color(bg)] for ch,fg,bg in row] for row in source]
    mismatches = [(x,y) for y,row in enumerate(cells) for x,cell in enumerate(row) if cell != expected[y][x]]
    assert not mismatches, f'{backend}: {len(mismatches)} cells differ from scene; first {mismatches[:5]}'
    steady = frames[1:]
    result = dict(backend=backend, frames=len(frames),
        inputs=json.loads(next(r['inputs'] for r in records if r.get('kind') == 'inputs')),
        scene_sha256=hashlib.sha256(log.with_suffix('.grid').read_bytes()).hexdigest(),
        cells_sha256=hashlib.sha256(json.dumps(cells).encode()).hexdigest(),
        binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(), native=status,
        timings={key:dict(median=statistics.median(f[key] for f in steady), maximum=max(f[key] for f in steady))
                 for key in ('render_us','normalize_us','adapt_us','present_us','total_us')})
    (directory/'summary.json').write_text(json.dumps(result,indent=2)+'\n')
    return result, cells

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--directory', type=Path, required=True)
    parser.add_argument('--fixture', default=str(ROOT/'perf/fixtures/12_gem_aetherium_2_bad_roll6.json'))
    parser.add_argument('--frames', type=int, default=12)
    parser.add_argument('--width', type=int, default=366)
    parser.add_argument('--height', type=int, default=199)
    parser.add_argument('--cols', type=int, default=400)
    parser.add_argument('--rows', type=int, default=200)
    parser.add_argument('--backends', nargs='+', choices=['ratatui','console','termwiz'],
                        default=['ratatui','console','termwiz'])
    args = parser.parse_args()
    if (not 2 <= args.frames <= 12 or not 1 <= args.cols <= 400
            or not 2 <= args.rows <= 200 or args.cols * args.rows > 80000
            or not 1 <= args.width <= args.cols or not 1 <= args.height < args.rows):
        parser.error('probe limits: 2..12 frames, terminal <=400x200, art fits with one spare row')
    args.directory = args.directory.resolve()
    args.directory.mkdir(parents=True,exist_ok=False)
    results = []
    reference = None
    for backend in args.backends:
        result, cells = run_case(args, backend)
        results.append(result)
        (args.directory/'results.json').write_text(json.dumps(results,indent=2)+'\n')
        print(backend, json.dumps(result['timings']), flush=True)
        if reference is None:
            reference = cells
        else:
            differences = [(x,y) for y,row in enumerate(cells) for x,cell in enumerate(row) if cell != reference[y][x]]
            result['different_cells'] = len(differences)
            (args.directory/'results.json').write_text(json.dumps(results,indent=2)+'\n')
            if differences:
                raise AssertionError(f'{backend}: {len(differences)} terminal cells differ; first {differences[:5]}')
        assert result['scene_sha256'] == results[0]['scene_sha256'], 'scene input/render mismatch'
    print(f'{len(results)} backend(s): final scene grids and colored terminal cell captures match.')

if __name__ == '__main__':
    main()
