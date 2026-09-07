"""Bound a foreground probe independently of its GUI observer.

Default ceilings: 15 s wall time, 256 MiB owned RSS, 128 MiB watched RSS growth,
768 MiB watched RSS total, 32 MiB artifacts, and 2 GiB free disk. Limits are
polled every 100 ms; they are trip thresholds, not instantaneous OS quotas.
Only the launched process and its descendants are stopped. The watched shared
terminal process is never signalled. No GUI scripting is needed for shutdown.
"""
import argparse
import json
import os
import pathlib
import shutil
import signal
import subprocess
import sys
import termios
import time


def processes():
    result = subprocess.run(['ps', '-axo', 'pid=,ppid=,rss='], check=True,
                            capture_output=True, text=True, timeout=.5)
    return {pid: (parent, rss) for pid, parent, rss in
            (map(int, line.split()) for line in result.stdout.splitlines())}


def descendants(table, root):
    owned = {root}
    while True:
        expanded = owned | {pid for pid, (parent, _) in table.items() if parent in owned}
        if expanded == owned:
            return owned
        owned = expanded


def observer_violation(path, now_ms):
    """Fail closed when the GUI observer stalls, disconnects, or loses focus."""
    if path is None:
        return None
    try:
        state = json.loads(pathlib.Path(path).read_text())
        age = now_ms - state['ts_ms']
        if not 0 <= age <= 1500:
            return 'observer_stale'
        if state.get('stop_reason'):
            return 'observer_requested_stop'
        if state.get('error'):
            return 'observer_failure'
        if state.get('mode') == 'headless':
            if state.get('transport_alive') is not True:
                return 'observer_transport_lost'
        elif state['focused'] is not True:
            return 'observer_focus_lost'
    except (OSError, ValueError, KeyError, TypeError):
        return 'observer_invalid'
    return None


def violation(*, elapsed, max_seconds, owned_kib, max_owned_kib,
              watched_kib, baseline_kib, max_watched_kib, max_growth_kib,
              artifact_bytes, max_artifact_bytes, free_bytes, min_free_bytes,
              owner_alive):
    checks = [
        (not owner_alive, 'observer_owner_exited'),
        (elapsed >= max_seconds, 'wall_time'),
        (owned_kib >= max_owned_kib, 'owned_rss'),
        (watched_kib >= max_watched_kib, 'watched_rss'),
        (watched_kib - baseline_kib >= max_growth_kib, 'watched_rss_growth'),
        (artifact_bytes >= max_artifact_bytes, 'artifact_bytes'),
        (free_bytes < min_free_bytes, 'disk_free'),
    ]
    return next((reason for tripped, reason in checks if tripped), None)


def run(args):
    state = pathlib.Path(args.state)
    artifact = pathlib.Path(args.artifact_dir)
    started = time.monotonic()
    child = None
    owned = set()
    reason = None
    tty_fd = None
    tty_state = None
    def record(kind, **values):
        with state.open('a') as log:
            log.write(json.dumps(dict(kind=kind, ts_ms=time.time_ns()//1_000_000,
                                      **values)) + '\n')
    def interrupted(signum, frame):
        nonlocal reason
        reason = f'signal_{signum}'
    for sig in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(sig, interrupted)
    try:
        try:
            tty_fd = os.open('/dev/tty', os.O_RDWR | os.O_NONBLOCK)
            tty_state = termios.tcgetattr(tty_fd)
        except OSError:
            pass
        table = processes()
        if args.watch_pid and args.watch_pid not in table:
            raise RuntimeError('watched process missing')
        baseline = table.get(args.watch_pid, (0, 0))[1]
        while True:
            table = processes()
            if child:
                owned |= descendants(table, child.pid)
            own_rss = sum(table.get(pid, (0, 0))[1] for pid in owned)
            if args.owner_pid is not None and args.owner_pid not in owned:
                own_rss += table.get(args.owner_pid, (0, 0))[1]
            watched = table.get(args.watch_pid, (0, 0))[1]
            if args.watch_pid and args.watch_pid not in table:
                reason = reason or 'watched_process_exited'
            size = sum(p.stat().st_size for p in artifact.rglob('*') if p.is_file())
            values = dict(elapsed=time.monotonic()-started, max_seconds=args.max_seconds,
                owned_kib=own_rss, max_owned_kib=args.max_owned_mib*1024,
                watched_kib=watched, baseline_kib=baseline,
                max_watched_kib=args.max_watched_mib*1024,
                max_growth_kib=args.max_growth_mib*1024,
                artifact_bytes=size, max_artifact_bytes=args.max_artifact_mib*1024**2,
                free_bytes=shutil.disk_usage(artifact).free,
                min_free_bytes=args.min_free_gib*1024**3,
                owner_alive=args.owner_pid is None or args.owner_pid in table)
            reason = reason or observer_violation(args.observer_state, time.time_ns()//1_000_000)
            reason = reason or violation(**values)
            record('check', **values)
            if reason:
                record('breaker', reason=reason, owned_pids=sorted(owned))
                break
            if child is None:
                child = subprocess.Popen(args.command)
                owned.add(child.pid)
                record('started', pid=child.pid)
            elif child.poll() is not None:
                record('completed', code=child.returncode)
                return child.returncode
            time.sleep(.1)
        return 124
    except BaseException as error:
        reason = reason or 'monitor_failure'
        record('breaker', reason=reason, error=str(error), owned_pids=sorted(owned))
        return 125
    finally:
        # Freeze parents before terminating descendants so they cannot spawn
        # replacement workers while cleanup runs. Never signal watch_pid.
        if child and (reason or child.poll() is None):
            for _ in range(2):
                try:
                    owned |= descendants(processes(), child.pid)
                except Exception:
                    pass
                for pid in owned - {args.watch_pid, os.getpid()}:
                    try:
                        os.kill(pid, signal.SIGSTOP)
                    except ProcessLookupError:
                        pass
            for pid in sorted(owned - {child.pid, args.watch_pid, os.getpid()}):
                try:
                    os.kill(pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
            # Stop the workload first. An explicitly selected profiler parent
            # may resume only to save its samples, under a fixed two-second cap.
            if args.save_profiler_on_stop and child.poll() is None:
                try:
                    os.kill(child.pid, signal.SIGINT)
                    os.kill(child.pid, signal.SIGCONT)
                    child.wait(timeout=2)
                    record('profiler_saved', code=child.returncode)
                except (ProcessLookupError, subprocess.TimeoutExpired):
                    pass
            if child.poll() is None:
                child.kill()
            child.wait(timeout=2)
            record('stopped', owned_pids=sorted(owned))
        if tty_state is not None:
            termios.tcsetattr(tty_fd, termios.TCSANOW, tty_state)
            try:
                os.write(tty_fd, b'\x18\x1b[0m\x1b[?25h\x1b[?1049l')
            except OSError:
                pass
        if tty_fd is not None:
            os.close(tty_fd)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--state', required=True)
    parser.add_argument('--artifact-dir', required=True)
    parser.add_argument('--owner-pid', type=int)
    parser.add_argument('--observer-state', help='atomic GUI heartbeat JSON; stale after 1.5 s')
    parser.add_argument('--save-profiler-on-stop', action='store_true',
                        help='after killing workload descendants, allow profiler parent 2 s to save')
    parser.add_argument('--watch-pid', type=int)
    parser.add_argument('--max-seconds', type=float, default=15)
    parser.add_argument('--max-owned-mib', type=float, default=256)
    parser.add_argument('--max-watched-mib', type=float, default=768)
    parser.add_argument('--max-growth-mib', type=float, default=128)
    parser.add_argument('--max-artifact-mib', type=float, default=32)
    parser.add_argument('--min-free-gib', type=float, default=2)
    parser.add_argument('command', nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.command[:1] == ['--']:
        args.command = args.command[1:]
    if not args.command:
        parser.error('a command is required')
    for name in ['max_seconds', 'max_owned_mib', 'max_watched_mib', 'max_growth_mib',
                 'max_artifact_mib', 'min_free_gib']:
        if getattr(args, name) <= 0:
            parser.error(f'{name} must be positive')
    sys.exit(run(args))
