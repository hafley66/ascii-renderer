#!/bin/bash
cd /Users/chrishafley/projects/ascii-renderer
unset TMUX
export XDG_CONFIG_HOME=/Users/chrishafley/projects/ascii-renderer/perf/results/9_iterm_io_1/config
export ASCII_TRACE_PATH=/Users/chrishafley/projects/ascii-renderer/perf/results/9_iterm_io_1/animation.ndjson
export ASCII_TRACE_ALL=1
/usr/bin/python3 -c 'import os,json,time; print(json.dumps({"ts_ms":time.time_ns()//1000000,"pid":os.getpid(),"ppid":os.getppid(),"tty":os.ttyname(0),"size":list(os.get_terminal_size(0)),"tmux":os.environ.get("TMUX")}))' > /Users/chrishafley/projects/ascii-renderer/perf/results/9_iterm_io_1/launch.json
/usr/bin/env DYLD_INSERT_LIBRARIES=/private/tmp/ascii_io_probe.dylib ASCII_SYSCALL_DIR=/Users/chrishafley/projects/ascii-renderer/perf/results/9_iterm_io_1 /Users/chrishafley/projects/ascii-renderer/target/release/ascii-renderer 42 demo
printf '\033[0m\033[48;2;0;255;0m\033[2J\033[HREPRO_DONE'
/usr/bin/python3 -c 'import time;print(time.time_ns()//1000000)' > /Users/chrishafley/projects/ascii-renderer/perf/results/9_iterm_io_1/exit_ms.txt
read -r -n 1
printf '\033[0m\033[2J\033[H'
