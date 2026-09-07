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

## Remaining cost boundaries

The terminal-write loop already presents the largest available pending slice.
The observed terminal accepts about 1,024 bytes per successful write, so joining
more application buffers does not reduce its successful syscall count. Byte and
terminal-command reduction occur before that boundary.

`append_ndjson` is outside the measured `presentation_us` interval and runs only
for selected trace records. Its open-per-record shape is present in the graph,
with five caller edges, but it does not account for the terminal stall.

Style emission remains proportional to visual style transitions. The current
encoder carries foreground and background state across cells and runs, omits
foreground changes for spaces, and quantizes adjacent colors before this audit.
Its remaining multiplicity is an output characteristic rather than one effect
call per source cell.
