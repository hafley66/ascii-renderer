# Extract N+1 audit

Date: 2026-09-07. `extract fast` analyzed every Rust source under `src/`
and `tests/`. `extract slow` read the repository compiler SCIP index. No live
renderer or terminal probe was launched.

## Extracted graph

| Extractor | Records | Relevant resolved path |
| --- | ---: | --- |
| `extract fast` | 40,553 | `AnsiFrameEncoder::encode -> collect_dirty_runs -> encode_runs -> encode_span -> write_sgr` |
| `extract slow` | 18,848 | Compiler definitions, references, local symbols, and function edges for the same source tree |

The fast output contained 18,287 resolved edges, 743 imports, 318 type edges,
and 21,205 unresolved records. The compiler output contained 5,475 function
edges, 1,668 references, 1,328 definitions, 1,327 names, 8,763 locals, and 208
callee-type records. The unresolved fast records were retained as lexical
candidates; compiler SCIP supplied the typed cross-check.

## Repeated effects

| Boundary | Multiplicity in the measured 392x134 case | Fixed cost per item | Classification |
| --- | ---: | --- | --- |
| Dirty run to cursor command in `encode_runs` | about 2,100 per frame | ANSI control parsing plus 3 to 12 output bytes | N+1 terminal commands |
| Cell to style/glyph encoding in `encode_span` | 3,850 to 6,000 changed cells per frame | color comparison, optional SGR, UTF-8 append | Required scan with command repetition on style changes |
| Encoded bytes to terminal writes in `pump` | 9,038 attempts per second in the latest relay sample | `write(2)` plus readiness/retry bookkeeping | N+1 syscalls caused by PTY backpressure |
| Trace event to `append_ndjson` | one open and one write per emitted event | directory check, open, JSON allocation, write, close | N+1 file operations at trace frequency |
| Input event to JSON control record | up to 256 drained events per input poll | JSON allocation and pipe write attempt | Bounded N+1 at interactive event frequency |

At 2000x2000, recorded delta frames contained 68,040 to 79,951 dirty runs and
16.3 to 18.0 MB of encoded output. The run count therefore multiplies terminal
control parsing even when Rust generation stays near 30 ms.

## Implemented boundary reduction

`encode_runs` now retains the logical cursor position after each dirty run.
For another run on the same row it emits no movement when already positioned,
otherwise it selects relative movement or absolute-column movement by encoded
byte cost. A row change still uses absolute row-and-column positioning. The
previous implementation emitted an absolute row-and-column command for every
dirty run.

This preserves the existing dirty-run partition and changes only cursor command
selection. Full-frame encoding and visual cell selection retain their current
behavior.

A deterministic 1,000-column fixture with 100 sparse runs encodes to 893 bytes
with one absolute cursor command per run and 506 bytes with retained cursor
selection. That is 387 fewer bytes, or 43.3% of the fixture's encoded stream.

The trace-writer lead was also implemented. A process now retains one O_APPEND
file per trace path behind a mutex. A 100-record test observes one retained file
and 100 complete lines, reducing opens from 100 to 1 while keeping the existing
single-write record boundary.

## Remaining cost boundaries

The terminal-write loop already presents the largest available pending slice.
The observed terminal accepts about 1,024 bytes per successful write, so joining
more application buffers does not reduce its successful syscall count. Byte and
terminal-command reduction occur before that boundary.

`append_ndjson` is outside the measured `presentation_us` interval and runs only
for selected trace records. Its five caller edges now converge on the retained
writer map.

Style emission remains proportional to visual style transitions. The current
encoder carries foreground and background state across cells and runs, omits
foreground changes for spaces, and quantizes adjacent colors before this audit.
Its remaining multiplicity is an output characteristic rather than one effect
call per source cell.

## Foreground fan-out experiment

Following `encode_span -> write_sgr -> push_color` produced a byte-composition
probe over 60 all-max `gem-aetherium-2` frames at 392x134. With the prior RGB
encoding, 19,432,499 total bytes partitioned into 14,383,110 foreground-control
bytes, 4,054,749 glyph bytes, 994,400 cursor bytes, and 240 reset bytes. The
1,053,638 control sequences averaged 17,561 per frame. Foreground commands were
74.0% of output; cursor commands were 5.1%.

Mapping animation RGB colors to the xterm 6x6x6 indexed-color cube reduced the
same deterministic stream to 10,296,932 bytes. Foreground bytes fell to
5,474,748 and total controls fell to 764,426. Total bytes decreased 47.0%; the
control count decreased 27.4%. Constant threshold comparisons implement the
nearest cube coordinate without allocation or search.

The full test run passed with 431 unit tests, three integration tests, and 186
snapshots. The guarded raw-iTerm validator did not produce a post-change sample:
the first dedicated window failed to launch its command, and the retry tripped
the unchanged 768 MiB watched-iTerm RSS ceiling before renderer startup. Limits
were not raised.
