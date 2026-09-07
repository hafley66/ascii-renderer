"""Bounded Ratatui drawing lab. Pexpect transports bytes; pyte interprets cells."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import statistics
import subprocess
import sys
import time
import pexpect
import pyte

ROOT = Path(__file__).resolve().parent.parent

def worker(args):
    directory = args.directory
    env = dict(os.environ)
    env.pop('NO_COLOR', None)
    env.pop('ASCII_FUNCTION_TRACE', None)
    command = [str(ROOT/'target/release/examples/0_terminal_lab'), args.backend, args.pattern,
               '400', '200', '8', 'memory' if args.consumer == 'memory' else 'stdout', str(directory/'frames.ndjson')]
    (directory/'command.json').write_text(json.dumps(command))
    started = time.monotonic_ns()
    parse_ns = byte_count = 0
    if args.consumer == 'memory':
        subprocess.run(command, env=env, stdout=subprocess.DEVNULL, check=True, timeout=10)
    elif args.consumer == 'native':
        driver = subprocess.Popen([str(ROOT/'target/release/examples/1_native_terminal'),
            '400','200',str(directory/'terminal.ansi.gz'),*command], env=env,
            stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True)
        def request(op, **fields):
            driver.stdin.write(json.dumps(dict(op=op,**fields))+'\n');driver.stdin.flush()
            return json.loads(driver.stdout.readline())
        try:
            assert json.loads(driver.stdout.readline())['ready']
            deadline=time.monotonic()+10
            while True:
                status=request('status')
                if status['reader_done']:break
                if time.monotonic()>deadline:raise TimeoutError('native terminal completion')
                time.sleep(.005)
            assert status['exit_code']==0 and status['error'] is None,status
            byte_count=status['bytes'];parse_ns=status['parse_us']*1000
            snapshot=request('capture')
            (directory/'cells.sha256').write_text(hashlib.sha256(json.dumps(snapshot['rows']).encode()).hexdigest()+'\n')
            driver.stdin.write('{"op":"quit"}\n');driver.stdin.flush()
            driver.wait(timeout=1)
        finally:
            if driver.poll() is None:
                driver.kill();driver.wait()
    else:
        child = pexpect.spawn(command[0], command[1:], env=env, dimensions=(200,400), maxread=65536)
        screen = pyte.Screen(400,200)
        stream = pyte.ByteStream(screen)
        try:
            with (directory/'terminal.ansi').open('wb') as out:
                while True:
                    try:
                        data = child.read_nonblocking(65536, timeout=2)
                    except pexpect.EOF:
                        break
                    out.write(data)
                    byte_count += len(data)
                    if args.consumer == 'pyte':
                        before = time.monotonic_ns()
                        stream.feed(data)
                        parse_ns += time.monotonic_ns()-before
            child.close()
            assert child.exitstatus == 0, (child.exitstatus,child.signalstatus)
        finally:
            if child.isalive(): child.close(force=True)
        if args.consumer == 'pyte':
            cells = [[tuple(screen.buffer[y][x]) for x in range(400)] for y in range(200)]
            digest = hashlib.sha256(json.dumps(cells).encode()).hexdigest()
            (directory/'cells.sha256').write_text(digest+'\n')
    rows = [json.loads(line)['fields'] for line in (directory/'frames.ndjson').read_text().splitlines()]
    assert len(rows) == 8 and [r['frame'] for r in rows] == list(range(8)), rows
    steady = rows[1:]
    metrics = {key: {'median': statistics.median(r[key] for r in steady), 'max': max(r[key] for r in steady)}
               for key in ('fill_us','library_us','output_us','total_us','bytes')}
    result = dict(status='passed', backend=args.backend, pattern=args.pattern, consumer=args.consumer,
        terminal=[400,200], frames=8, steady_frames=7, metrics=metrics, bytes_read=byte_count,
        parser_us=parse_ns//1000, elapsed_us=(time.monotonic_ns()-started)//1000,
        scope='drawing lab; application generation and GUI painting excluded')
    (directory/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result),flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--directory',type=Path,required=True)
    parser.add_argument('--worker',action='store_true')
    parser.add_argument('--backend',choices=['crossterm','termion'])
    parser.add_argument('--pattern',choices=['static','sparse','mono','color'])
    parser.add_argument('--consumer',choices=['memory','drain','native','pyte'])
    args=parser.parse_args();args.directory=args.directory.resolve()
    if args.worker:
        worker(args);return
    args.directory.mkdir(parents=True,exist_ok=False)
    results=[]
    for pattern in ('static','sparse','mono','color'):
        for backend in ('crossterm','termion'):
            for consumer in ((args.consumer,) if args.consumer else ('memory','drain','native')):
                directory=args.directory/f'{pattern}-{backend}-{consumer}'
                directory.mkdir()
                command=[sys.executable,str(ROOT/'scripts/5_probe_guard.py'),
                    '--state',str(directory/'guard.ndjson'),'--artifact-dir',str(directory),
                    '--max-seconds','15','--max-owned-mib','256','--max-artifact-mib','32',
                    '--owner-pid',str(os.getpid()),'--',sys.executable,str(Path(__file__).resolve()),
                    '--worker','--directory',str(directory),'--backend',backend,'--pattern',pattern,'--consumer',consumer]
                with (directory/'stdout.log').open('w') as out, (directory/'stderr.log').open('w') as err:
                    status=subprocess.run(command,stdout=out,stderr=err,timeout=20)
                if status.returncode:
                    raise SystemExit(f'Lab case stopped: {directory}; inspect guard and stderr')
                results.append(json.loads((directory/'result.json').read_text()))
                print(f'{pattern} {backend} {consumer}: {results[-1]["metrics"]["total_us"]}',flush=True)
                (args.directory/'results.json').write_text(json.dumps(results,indent=2)+'\n')
        if args.consumer in (None, 'native', 'pyte'):
            consumer=args.consumer or 'native'
            left=(args.directory/f'{pattern}-crossterm-{consumer}/cells.sha256').read_text()
            right=(args.directory/f'{pattern}-termion-{consumer}/cells.sha256').read_text()
            assert left==right, f'{pattern}: library terminal cells differ'
    checked = args.consumer in (None, 'native', 'pyte')
    (args.directory/'equivalence.json').write_text(json.dumps({'status':'passed' if checked else 'not_checked','patterns':4 if checked else 0,'cells_per_pattern':80000})+'\n')

if __name__=='__main__':main()
