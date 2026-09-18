# Gothic stroke animation and demo controls

Planning baseline: `285a947`, 2026-09-18. Implementation owner: Luna. This document is a plan, with proposed symbols explicitly marked below. No implementation, build, snapshot acceptance, or terminal probe was performed to produce it. Read the worktree `AGENTS.md` before implementation. Preserve existing modes and unrelated edits. `_57_reef.rs` is the last mode in this worktree; `_58` is reserved for the user's unrelated ferrofluid work. Add `_59_gothic_trace.rs`. Do not inspect or edit the primary worktree to obtain ferrofluid.

## 1. Rust signatures

### New file: `src/modes/_59_gothic_trace.rs`

All declarations in this block are proposed. Keep the geometry, animation, cache, parameters, and tests in this file. Imports use existing `registry::{Mode, ModeFrame, AnimKind, Param}`, `types::{Cell, Grid}`, `opts::param_f32`, `color::{darken, lighten}`, `crossterm::style::Color`, and `rand::{SeedableRng, RngExt, rngs::StdRng}`.

```rust
pub(super) struct GothicTrace;
pub(super) static MODE: GothicTrace = GothicTrace;

impl Mode for GothicTrace {
    fn name(&self) -> &'static str;             // "gothic-trace"
    fn help(&self) -> &'static str;
    fn animation(&self) -> AnimKind;            // AnimKind::Iterate
    fn params(&self) -> &'static [Param];        // PARAMS
    fn render(&self, frame: &mut ModeFrame<'_>);
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Point { x: f32, y: f32 }
struct Stroke {
    points: Vec<Point>,
    distance: Vec<f32>, // cumulative physical distance, first = 0
    length: f32,
    depth: u8,
    ink: usize,        // index into frame.palette, 0..5
    delay: f32,        // additional frame-composition delay, seconds
}
#[derive(Clone, Copy)]
struct Knobs {
    scene: usize, depth: usize, slender: f32, asymmetry: f32,
    lobes: usize, winding: usize, bulge: f32, strands: usize,
    speed: f32, draw: f32, stagger: f32, trail: f32, ghost: f32,
}
#[derive(PartialEq, Eq)]
struct GeometryKey { width: usize, height: usize, seed: u64, knobs: [u32; 8] }
struct Geometry { key: GeometryKey, strokes: Vec<Stroke> }
// thread_local! static GEOMETRY: RefCell<Option<Geometry>> = const { RefCell::new(None) };

fn resolve_knobs(frame: &ModeFrame<'_>) -> Knobs;
fn build_geometry(width: usize, height: usize, seed: u64, k: &Knobs) -> Vec<Stroke>;
fn add_stroke(out: &mut Vec<Stroke>, points: Vec<Point>, depth: u8, ink: usize, delay: f32);
fn flatten_cubic(out: &mut Vec<Point>, p: [Point; 4], depth: u8);
fn lancet(cx: f32, base: f32, width: f32, height: f32) -> Vec<Point>;
fn branch(out: &mut Vec<Stroke>, rng: &mut StdRng, cx: f32, base: f32,
          width: f32, height: f32, level: usize, k: &Knobs);
fn guilloche(center: Point, radius: f32, lobes: usize, winding: usize,
             bulge: f32, phase: f32, rotation: f32) -> Vec<Point>;
fn frame_strokes(out: &mut Vec<Stroke>, width: f32, height: f32, seed: u64);
fn draw_ease(x: f32) -> f32;
fn stroke_window(time: f32, index: usize, extra_delay: f32, k: &Knobs)
    -> (f32, f32, bool); // normalized start, end, active pen
fn paint_stroke(grid: &mut Grid, stroke: &Stroke, start: f32, end: f32,
                color: Color, ghost: bool);
fn stroke_point(stroke: &Stroke, fraction: f32) -> Point;
fn draw_trace(grid: &mut Grid, strokes: &[Stroke], palette: &[Color; 5],
              time: f32, k: &Knobs);
```

### Changes inside `src/opts.rs` and `src/morph.rs`

Keep existing public signatures `demo_pick_mode`, `run_demo`, `morph_session`, `morph_worker_session`, `IterateFrameRenderer::new`, and `IterateFrameRenderer::render`. The following small helpers/types are proposed solely to expose the changed transitions to deterministic tests.

```rust
// src/opts.rs, adjacent to demo_pick_mode
fn demo_picker_selection(sel: usize, matches: usize,
                         direction: Option<crossterm::event::KeyCode>) -> usize;

// src/morph.rs, adjacent to morph_worker_session
#[derive(Debug, PartialEq)]
enum SeedInput { Pass, Consumed, Set(u64) }
fn animation_seed_input(
    edit: &mut Option<String>,
    key: crossterm::event::KeyEvent,
    current: u64,
    pane_has_params: bool,
    random_seed: impl FnOnce() -> u64,
) -> SeedInput;

// New method on the existing IterateFrameRenderer, not a new renderer type.
impl IterateFrameRenderer {
    fn reseed(&mut self, seed: u64, theme: &str);
}
```

Existing transport stays `Control::Input(Event)` and `Control::Displayed(u64)` in `src/_1_playback.rs`. Do not add a second input reader, a worker terminal prompt, a signal handler, or a new IPC protocol.

## 2. Commented pseudocode bodies

### 2.1 Mode registration, parameters, and frame entry

Use `param!(key, label, min, max, default, step)` exactly as declared in `src/registry.rs:164` and used by `_57_reef.rs`. Parameter order is positional CLI order and `ModeFrame::param_values` order. Prefix keys to avoid inheriting another mode's environment values.

```rust
const PARAMS: &[Param] = &[
    param!("GT_SCENE", "0 lancet / 1 rosette", 0.0, 1.0, 0.0, 1.0),
    param!("GT_DEPTH", "lancet generations", 1.0, 4.0, 3.0, 1.0),
    param!("GT_SLENDER", "window width / height", 0.4, 1.0, 0.82, 0.02),
    param!("GT_ASYM", "seeded branch asymmetry", 0.0, 0.3, 0.14, 0.01),
    param!("GT_LOBES", "guilloche lobes", 3.0, 17.0, 8.0, 1.0),
    param!("GT_WIND", "guilloche winding", 1.0, 7.0, 3.0, 1.0),
    param!("GT_BULGE", "guilloche petal depth", 0.08, 0.48, 0.3, 0.01),
    param!("GT_STRANDS", "interlaced strands", 1.0, 4.0, 2.0, 1.0),
    param!("GT_SPEED", "time multiplier", 0.0, 3.0, 1.0, 0.1),
    param!("GT_DRAW", "draw duration", 0.5, 8.0, 3.0, 0.25),
    param!("GT_STAGGER", "stroke delay", 0.0, 0.15, 0.04, 0.01),
    param!("GT_TRAIL", "tracing tail fraction", 0.02, 0.5, 0.12, 0.02),
    param!("GT_GHOST", "guide visibility", 0.0, 1.0, 0.18, 0.05),
];

// resolve_knobs:
// for each PARAMS[i]: parse frame.args[i+4], else frame.param_values[i],
// else param_f32(key, default). Nonfinite -> default; clamp to min/max.
// Round integer fields only after clamping. Defaults come from PARAMS.
// Match _57_reef.rs:80-95 precedence. Do not read env inside geometry loops.
// help: list all 13 arguments in this exact order and explain the two scenes.

// Mode::render:
// return immediately for width==0 || height==0.
// resolve k; key = dimensions + frame.seed + the first eight resolved knobs
// represented by their canonical f32.to_bits() (integer fields back to f32).
// GEOMETRY.borrow_mut(): replace its single entry only when key differs.
// build_geometry uses no time, palette, speed, draw, stagger, trail, or ghost.
// clear grid rows with Cell::blank(), then draw_trace(... frame.time ...).
// Do not hold frame.rng across calls; geometry uses its own seeded RNG below.
```

### 2.2 Geometry and coordinates

Constants: `CELL_ASPECT=2.0`, `FLATNESS=0.08`, `MAX_FLATTEN_DEPTH=8`, `TAU=std::f32::consts::TAU`, `LACE=0.045`, `BRAID=0.22`, `SHRINK=0.46`, `TWIST=13.0*std::f32::consts::PI/180.0`, `CHILD_RISE=0.64`, `STONE=0.05`. Geometry coordinates use physical row units with x increasing right and y increasing down. Map world `(x,y)` to terminal `(round(2*x), round(y))`. Thus physical canvas `W=(width-1) as f32/2.0`, `H=(height-1) as f32`; center `(W/2,H/2)`. The two scenes share the outer frame and reveal system.

```rust
// flatten_cubic(out, [p0,p1,p2,p3], depth):
// out initially contains p0. If both control points are within FLATNESS of
// segment p0..p3, or depth==8, append p3. Use point-to-segment distance,
// with the zero-length chord handled by distance to p0.
// Otherwise de Casteljau split at 0.5; recurse left then right.
// A cubic contributes <=256 segments. No time-dependent subdivision.
// A quadratic (a,b,c) converts to cubic (a, a+2/3*(b-a), c+2/3*(b-c), c).

// add_stroke:
// remove adjacent duplicate points; skip <2 points or nonfinite coordinates.
// cumulative distance[i] = distance[i-1] + hypot(delta.x,delta.y).
// skip total length <=1e-6. Preserve push order as stable stroke identity.
// Every disconnected SVG subpath becomes a separate Stroke. Never bridge a Move.

// lancet(cx, base, w, h): matches gothic/src/lib/2a_architecture.ts:4-17.
// spring=base-.55*h; rise=.45*h; top=spring-rise; l=cx-w/2; r=cx+w/2.
// Start (l,base), append (l,spring).
// Cubic [(l,spring),(l,spring-.55*rise),(cx-.22*w,top+.16*rise),(cx,top)].
// Cubic [(cx,top),(cx+.22*w,top+.16*rise),(r,spring-.55*rise),(r,spring)].
// Append (r,base). No base closing segment inside this stroke.

// branch(out,rng,cx,base,w,h,level,k):
// push lancet with depth=level, ink=level%5, delay=0.
// Stop if level+1>=k.depth || w<4 || h<6 physical units.
// gap=w*STONE; share=.5+(rng.random::<f32>()-.5)*k.asymmetry.
// left=(w-gap)*share; right=w-gap-left; childH=h*CHILD_RISE.
// recurse left cx-w/2+left/2, then right cx+w/2-right/2; base stays fixed.
// r=min(.14*w, h*(1-CHILD_RISE)*.37); y=base-childH-1.12*r.
// If r>=1.25, append a guide circle radius 1.12*r and ONE guilloche light
// radius r centered (cx,y), using phase=rng.random::<f32>()*TAU followed by
// rotation=rng.random::<f32>()*TAU, requested lobes/winding/bulge.
// This substitutes a woven crown light for architecture.generate's foilRing.

// guilloche(center,R,lobes,winding,bulge,phase,rotation):
// Advance winding until gcd(winding,lobes)==1, as reference generate does.
// Wheels (frequency, amplitude, offset):
//   (winding,1-bulge,0), (winding-lobes,bulge,phase),
//   (winding+lobes,LACE,-phase).
// Cast winding and lobes to i32 BEFORE frequency subtraction, then to f32.
// scale=R/(1+LACE).
// P(t)=center+scale*sum amplitude*(cos(f*t+offset+rotation),sin(...)).
// V(t)=scale*sum amplitude*f*(-sin(...),cos(...)).
// count=(winding+lobes)*18; dt=TAU/count.
// For i=0..count-1 flatten Hermite cubic:
// [P(i*dt), P(i*dt)+V(i*dt)*dt/3,
//  P((i+1)*dt)-V((i+1)*dt)*dt/3, P((i+1)*dt)].
// Force final point equal to first, eliminating floating seam drift.
// gcd uses a local Euclidean while loop, no new general utility module.

// build_geometry:
// If W<4 || H<4: return one diagonal from (0,H) to (W,0), including 1x1
// via the render-level tiny-grid fallback described below.
// frame_strokes first, then scene strokes. This is reveal order.
// S=min(W,H); margin=1; base=H/2+.40*S; windowH=.76*S;
// windowW=min(windowH*k.slender,W-4).
// scene 0: push two outer casings, i=2 then 1, pad=.018*S*i,
// lancet(cx,base,windowW+2*pad,windowH+1.2*pad), depth=0, ink=0.
// branch(...windowW,windowH,0); add baseline extending .035*S each side.
// Branch RNG=StdRng::seed_from_u64(seed ^ 0x4754_4152_4348).
// scene 1: use R=.36*S, center canvas center; two concentric bands.
// rng=StdRng::seed_from_u64(seed ^ 0x4754_524F_5345); rotation=rng()*TAU.
// Each band consumes one phase=rng()*TAU; r=R*SHRINK.powi(band).
// Stop when r<1.5. Add circle guide at 1.035*r.
// strands=min(k.strands,max(1,floor(TAU*r/(k.lobes*2)))); physical LOD.
// Thread offset=(thread/(strands-1)-.5)*BRAID*TAU, or 0 for one strand.
// Use guilloche(...phase+offset,rotation+band*TWIST), depth=band,
// ink=(band+thread)%5. Generate circle polylines using
// max(16,ceil(TAU*r/.2)) segments; force closed endpoint.
```

`frame_strokes` adapts `legacy/4_frames.js`'s `corners.cusp`, `rails.double`, `finials.diamond`, and `compose` into the same physical coordinate system:

```rust
// p=1; iw=W-2*p; ih=H-2*p; c=min(.16*min(W,H), min(iw,ih)/3).
// If c<.6, omit motifs and draw a rectangle inset by min(.5,W/4,H/4).
// RNG=StdRng::seed_from_u64(seed ^ 0x4754_4652_414D).
// Sample n=c*(.30+.15*rng()) once; mirror the SAME corner four times.
// Corner points/curves: M(c,0), L(1.2*n,0), Q(.55*n,0,n,n),
// Q(0,.55*n,0,1.2*n), L(0,c). Flatten quadratics as above.
// Corner diamond center=(1.55*n,1.55*n), rx=.085*c, ry=1.7*rx;
// omit diamond when rx<.25, keeping its parent cusp.
// Corner transforms in order: (+,+,p,p), (-,+,p+iw,p),
// (+,-,p,p+ih), (-,-,p+iw,p+ih); delays 0,.06,.06,.12.
// Top/bottom rails connect p+c to p+iw-c; left/right analogously.
// Draw inner parallel rail at inward offset max(.5,.12*c), ends inset .2*c.
// If edge span>=9*c, leave center gap +/- .5*c in OUTER rail and insert
// diamond finial at edge midpoint, rx=.12*c, ry=1.8*rx, delay .2.
// Otherwise draw uninterrupted outer rail. Rails delay=.03.
// Rails, corners, diamonds are separate strokes, ink=0, depth=0.
// All four edge transforms keep the local motif's +y pointing inward.
```

Use one `StdRng` stream per named geometry layer. Consume in the stated traversal order. Seed changes affect asymmetric branch widths, curve phase/rotation, and frame cusp size. `GT_ASYM=0` removes branch asymmetry; other seeded geometry remains. Geometry does not vary with time or frame rate.

### 2.3 Reveal, tracing, and ASCII rasterization

The reference CSS uses normalized SVG path length, `stroke-dashoffset: 1 -> 0`, cubic-bezier `(0.2,0.7,0.2,1)`, and stagger capped at 800 ms. The border page places its pen at `getPointAtLength(t*length)`. Reproduce these relationships through cumulative physical path length. Do not reveal by vertex count.

```rust
// draw_ease(x): clamp x to [0,1]; exact endpoints return x.
// Find u with Bx(u)=x using 16 bisection iterations on [0,1].
// Bx=3*(1-u)^2*u*.2 + 3*(1-u)*u^2*.2 + u^3.
// Return By=3*(1-u)^2*u*.7 + 3*(1-u)*u^2*1 + u^3.

// stroke_window:
// D=k.draw; maxDelay=1.0 (800ms capped stagger + 200ms frame finial).
// revealEnd=D+maxDelay; holdEnd=revealEnd+1.0;
// traceEnd=holdEnd+D; cycle=traceEnd+.5.
// Multiply sanitized nonnegative time by speed in f64, rem_euclid(cycle
// as f64), then cast to f32 for tau. Nonfinite input time becomes zero.
// delay=min(.8,index*k.stagger)+extra_delay.
// tau < revealEnd: end=draw_ease(clamp((tau-delay)/D,0,1));
//   return (0,end, tau>delay && tau<delay+D).
// tau < holdEnd: return (0,1,false).
// tau < traceEnd: head=(tau-holdEnd)/D;
//   return (head-k.trail,head,true); negative start denotes wraparound.
// else: return (0,0,false), showing only the ghost before restarting.
// Time unit is ModeFrame.time, not wall seconds: morph's current clock
// advances .06*60 units per wall second. Do not change that global clock.

// stroke_point: clamp fraction; binary-search cumulative distance for
// fraction*length; interpolate that segment. Handle fraction==1 exactly.

// paint_stroke:
// Iterate segments whose cumulative intervals intersect [start,end].
// Clip both endpoints by distance, then project x*=2.
// DDA steps=ceil(max(abs(dx),abs(dy))*2), minimum 1; round to cell indices,
// check bounds before indexing. Zero interval paints nothing.
// Glyph from projected tangent: '-' when |dy|<=.4142*|dx|,
// '|' when |dy|>=2.4142*|dx|, otherwise '\\' if dx*dy>=0 else '/'.
// In ghost pass write '.' into blank cells only; foreground pass replaces it.
// Foreground intersections: if prior foreground glyph differs, use '+';
// retain '+' at subsequent crossings. Color follows deterministic path order.
// No box-drawing, braille, or full-width glyphs: output is ASCII.

// draw_trace:
// Tiny grids W<4 || H<4: render fallback diagonal without animation, with
// '*' at [0][0] for a 1x1 canvas. Empty dimensions return before this.
// Clear grid. If ghost>0, paint whole paths '.', color darkened by
// round(220*(1-ghost)) plus min(40,depth*12), saturating at 255.
// For each path get stroke_window; normal interval paint directly.
// Negative start in trace phase: paint [1+start,1] then [0,end].
// Foreground color=darken(palette[ink],min(80,depth*18)).
// Collect pen endpoints during this pass. Paint all pen '*' cells LAST,
// using lighten(palette[ink],45), so later paths cannot overwrite the tip.
// One tip per currently drawing/tracing path; no tip during full hold/reset.
```

Completed paths persist through the hold, then become moving short traces over the ghost. Every cycle repeats identical geometry. `GT_SPEED=0` freezes the initial ghost; selecting nonzero `ASCII_T` with speed zero also yields that same frame. Snapshot the full drawing during the hold, not only the ghost at time zero.

### 2.4 Slash picker fix

Source: `src/opts.rs:425-542`, `demo_pick_mode`. At the start of each loop, current code does `if sel >= filtered.len() { sel = filtered.len().saturating_sub(1) }`. This turns the selectable cancel row (`cancel_idx=filtered.len()`) into the last mode. The later `if sel > cancel_idx` cannot repair that loss. Arrow branches at lines 528-529 already intend the inclusive cycle through cancel.

```rust
// demo_picker_selection(sel,matches,direction):
// let sel=min(sel,matches); // matches itself is the valid cancel index
// Up => if sel==0 { matches } else { sel-1 }
// Down => if sel==matches { 0 } else { sel+1 }
// _ => sel

// demo_pick_mode:
// filtered=demo_filter_modes(all_modes,&query);
// cancel_idx=filtered.len();
// sel=demo_picker_selection(sel,cancel_idx,None);
// DELETE the earlier >=filtered.len() clamp; retain the existing viewport
// anchor=min(sel,filtered.len().saturating_sub(1)) and offset calculation.
// Up/Down call helper with Some(key.code).
// Keep query edit/backspace -> sel=0, Enter on cancel -> None,
// Esc/Ctrl+C -> None. Empty results: cancel is index zero.
```

Acceptance interpretation: “last” is the existing pinned cancel row. Thus first mode -> Up -> cancel -> Down -> first mode. Last actual mode -> Down -> cancel -> Down -> first remains selectable in both directions. Preserve the cancel item.

### 2.5 Animation seed controls

Current path is `opts::run_demo` (`a` branch, line 916) -> `morph::morph_session` (1293) -> `_1_playback::animate` (385) -> `supervise` (142) -> `pump` (184) -> child `--animation-worker` -> `_1_playback::worker` (411) -> `morph_worker_session` (1322). The supervisor serializes input events as JSON lines to child stdin. A child reader thread sends decoded `Control` values into `sync_channel(32)`. The render loop collects events during `write_frame` and its frame acknowledgement wait, then handles them at lines 1786 onward. These are existing symbols, not proposed transport.

The current worker uses immutable `seed_a` for the native renderer, `vflow`, randomized knobs, palette, pane, and frame log. `n` changes endpoint seeds but cannot reseed `IterateFrameRenderer`; Up/Down have only pane-selection arms; Enter has no reseed arm. `roll` is a separate nonce loaded from `ASCII_ROLL`/`LIVE_ROLL`, then changed by random knob arrows or +/-.

Implement these exact input semantics:

| Context | Event | Result |
|---|---|---|
| No seed prompt, any pane/random state | Enter | Random geometry seed in `0..10000`, guaranteed different from current |
| No seed prompt | `e` | Open empty decimal seed prompt in footer |
| Seed prompt | ASCII digits | Append up to 20 digits, consume event before strategy shortcuts |
| Seed prompt | Backspace | Pop digit |
| Seed prompt | Enter | Parse full `u64`; valid commits, empty/overflow leaves prompt open with error |
| Seed prompt | `e` | Cancel edit, keep geometry seed |
| Seed prompt | Other non-quit keys | Consume, without toggling strategy, knobs, playback, or randomness |
| No prompt, pane closed or no params | Up / Down | Geometry seed wrapping +1 / -1 |
| No prompt, pane open with params | Up / Down | Existing knob selection |
| Native `iterate`, no prompt | `n` | Geometry seed wrapping +1; use the same rebuild path |
| Other strategies, no prompt | `n` | Existing endpoint advancement behavior |
| Any context | q/Q, Esc | Existing immediate supervisor quit, return to demo |
| Any context | Ctrl+C | Existing supervisor interrupt, exit demo |

The prompt says `seed> …  enter=apply e=cancel esc/q=leave`. Esc keeps the existing immediate cancellation contract in `_1_playback::quit`; it does not become a worker-only edit cancellation. No blocking `event::read` inside the worker prompt. Render and pause state continue while digits are entered. Filter `KeyEventKind::Release` before handling worker keys; retain Press and Repeat.

```rust
// animation_seed_input(edit,key,current,pane_has_params,random_seed):
// Release -> Consumed. Quit/Ctrl+C -> Pass (direct/non-Unix fallback).
// if edit is Some:
//   digit -> append if len<20; Consumed
//   Backspace -> pop; Consumed
//   e -> edit=None; Consumed
//   Enter -> parse::<u64>(); on success edit=None, Set(value);
//            on failure leave text, Consumed
//   otherwise -> Consumed
// else:
//   e -> Some(String::new()), Consumed
//   Enter -> sample=random_seed(); debug_assert sample<10000;
//            Set(if sample==current {(sample+1)%10000} else {sample})
//   Up if !pane_has_params -> Set(current.wrapping_add(1))
//   Down if !pane_has_params -> Set(current.wrapping_sub(1))
//   otherwise -> Pass
// Derive prompt error from edit text: nonempty and parse::<u64>().is_err().
// The production closure is || rand::rng().random_range(0..10000u64).
// Tests supply fixed numbers; do not add a runtime test-seed env variable.

// IterateFrameRenderer::reseed:
// self.seed=seed;
// self.palette=if theme.is_empty() {make_palette(seed)} else
//              {named_theme(theme).unwrap_or_else(||make_palette(seed))};
// self.rng=StdRng::seed_from_u64(seed);
// retain grid allocation. Next render clears grid and passes new seed through
// iterate_grid_into -> ModeFrame.seed. Existing mode caches handle their keys.
```

In `morph_worker_session`, add `let mut geometry_seed=seed_a`, `let seed_delta=seed_b.wrapping_sub(seed_a)`, `let mut target_seed=seed_b`, and `let mut seed_edit=None::<String>`. Make the existing native `palette` mutable. Add a per-event `requested_seed: Option<u64>` rather than rebuilding inside each key branch.

```rust
// For each Event::Key, after recording the previous history tuple:
// run animation_seed_input before existing key.code match.
// Set(s) -> requested_seed=Some(s), skip ordinary key handling.
// Consumed -> skip ordinary key handling, still redraw/footer and log event.
// Pass -> existing match, except native `n` requests geometry_seed+1.
// `b`: pop tuple, restore roll/pvals/randomize, request its geometry seed.
// Do not assign geometry_seed before the common invalidation block.
// Handle Resize through existing branch even when seed_edit is Some.

// At the end of each event, if requested_seed differs from geometry_seed:
// geometry_seed=s; target_seed=s.wrapping_add(seed_delta);
// endpoint_a=s; endpoint_b=target_seed; walk_seed=target_seed;
// endpoint_mode_a=mode_a; endpoint_mode_b=mode_b;
// st=None; // discard cached endpoint grids/MorphState
// palette=make_palette(s), or named_theme(theme) with seeded fallback;
// if iterate_renderer is Some: renderer.reseed(s,theme);
// phase=0; dir=1; clock=0; last_frame=Instant::now();
// preserve playing, strat, pane_open, psel, pvals, randomize, roll.
// frame_encoder.invalidate(); previous_status.clear(); previous_pane.clear();
// clear_frame=true. A new seed restarts draw-in even while paused.
// Setting the same seed closes the prompt but does not reset clock/history.
```

Replace uses of startup `seed_a` after worker initialization with `geometry_seed` wherever they represent current geometry: effective knob hashing, native renderer creation on size changes, `vflow`, pane arguments, footer, `FrameInputs.seed`, and resize endpoint reset. Replace current-state uses of `seed_b` with `target_seed`. Keep `seed_a/seed_b` in initial `seed_delta` and initialization only. Endpoint walk behavior remains independent; do not turn every native frame's existing walk phase transition into a reseed.

The top-of-loop renderer validity check must test `renderer.seed != geometry_seed` in addition to dimensions. Rebuild with `IterateFrameRenderer::new(mode_a,geometry_seed,theme,rw,h)` on size change; use `reseed` for seed-only changes. Resize uses pane-reduced `rw`, not the full width, when recreating native storage. All endpoint invalidation takes effect before the next grid is generated.

Random knobs retain the existing equation exactly:

```text
effective random key = geometry_seed XOR (roll * 0x9E37_79B9_7F4A_7C15 mod 2^64)
eff[i] = rand_knob(effective random key, spec.params[i])
```

Changing geometry seed leaves `roll` untouched and deterministically recomputes `eff`. Changing `roll` leaves geometry seed untouched. `g` changes only the randomization flag. Saved `pvals` remain the user's manual values; never persist generated `eff` over them. Extend existing history to `Vec<(u64, u64, Vec<f32>, bool)>`, ordered `(geometry_seed, roll, pvals, randomize)`, bounded at 512 as now. `b` restores all four; use the same reseed invalidation when needed, without pushing the popped state again. Seed edits enter history only on a changed committed seed.

Before lazy endpoint `render_frame` calls, write resolved `eff` into `LIVE_PARAMS` even for registered modes when endpoints are required. The current `!registered_native_params` condition alone leaves registered fallback subprocesses using startup values. Use `if !registered_native_params || needs_endpoints` after computing `needs_endpoints`. Maintain `LIVE_ROLL` with the worker's current `roll` before subprocess launch. `render_frame_t` already calls `live_params_to_command`, so `Command::env` carries the current values without mutating the process environment.

The footer must place `seed:<geometry_seed>` near the start in both pane states, before verbose help can be truncated. Seed prompt replaces footer help, keeping dimensions and the current seed visible. Use `previous_status`'s existing diff machinery. Frame trace inputs carry current seed, palette, `randomize`, `roll`, and effective knobs; add `target_seed` to its existing JSON extension to make endpoint resets inspectable.

Worker seed and history live for one animation session, matching the current worker-local nonce/history lifetime. Returning with q restores the demo's existing parent seed; no seed is persisted to options, and no return-state IPC is introduced by this task. Document this explicitly in help/test expectations. The existing bool `morph_session` return remains solely the interrupt result.

## 3. Instance lifetimes, storage, and read/write order

| Instance | Lifetime / owner | Reads and writes |
|---|---|---|
| `MODE` | Static singleton, immutable | Registry borrows it; no time or simulation state |
| `GEOMETRY` | One cached entry per render thread | Key checks dimensions, seed, 8 geometry knobs; palette/time/reveal changes reuse paths |
| `Stroke` arrays | Until cache replacement/thread exit | Ordered points and cumulative distances immutable after build |
| `ModeFrame` | One render call | Borrows grid, palette, parameter slice; writes only grid |
| `IterateFrameRenderer` | Animation session, recreated for dimensions | Grid retained; reseed writes seed/palette/RNG; render resets RNG and clears grid |
| `geometry_seed`, `target_seed`, `roll`, prompt/history | Animation worker loop | Only render thread mutates; reader thread sends events only |
| `LIVE_PARAMS` / `LIVE_ROLL` | Current thread | Resolved before legacy rendering or explicit child environment construction |
| Picker `query`, `sel` | One picker invocation | Query rebuilds filtered indices; selection clamps to inclusive cancel index |

Seed event order is terminal poll -> JSON `Control::Input` -> bounded pipe queue -> child receiver -> worker event handling -> seed/palette/cache writes -> next-loop effective knobs -> renderer/endpoint generation -> encoded grid/footer -> framed stdout -> terminal acceptance -> `Control::Displayed`. The existing relay can drop old whole input records when its 32-entry pending queue saturates; do not claim lossless arbitrary floods. Partial records and displayed acknowledgements retain existing handling. Tests type bounded sequences and wait for confirmed screen transitions.

Existing acknowledgement waiting can defer control application behind a stalled terminal. `perf/0_E2E.md` records a failed 1,005 ms knob application against a 250 ms gate. These fixes do not authorize replacing the relay or increasing that bound. Run and report that case accurately; preserve a failure and its inputs if it still occurs.

Cache uniqueness is exact equality of resolved geometry inputs. No wall time, process ID, global RNG, accumulated frame deltas, palette, or render order enters geometry. A frame at `(seed,time,knobs,width,height,palette)` is reproducible after arbitrary intervening frames. Screen resize is allowed to alter LOD and geometry layout deterministically.

## 4. Source grounding

Reference source was read only beneath `/Users/chrishafley/projects/hafley-rxjs/packages/gothic`. Do not load its imported `report-shell` source or edit gothic. The code here ports equations/behavior, with terminal LOD and cyclic replay explicitly specified above; it does not claim pixel-identical SVG output.

| Verified source | Relevant existing symbols / behavior |
|---|---|
| `gothic/src/ui/4_Algo.tsx:11-61` | `AlgoSvg`, `AlgoCells`, `pathLength={1}`, positional path identity, `useDrawIn` invocation |
| `gothic/src/app.css:77-102,127-151` | Normalized draw-in, cubic easing, capped stagger; ghost, ink, and pen border layers |
| `gothic/src/pages/3_border.tsx:111-118` | `BorderBody`'s `scrub`, `getPointAtLength(t*m.len)` pen position |
| `gothic/src/algos/3_guilloche.ts:32-89` | `guillochePath`, derivative-based Hermite controls, `generate`, coprime winding, bands and strand LOD |
| `gothic/src/lib/2a_architecture.ts:4-17` | `ogive`, `lancet`, exact cubic controls and spring/rise fractions |
| `gothic/src/algos/4_architecture.ts:22-50` | `generate`'s recursive `branch`, casings, seed-dependent split, crown light positioning |
| `gothic/src/lib/legacy/2_arches.js:89-115` | `twoCentred`, `sampleHead`: alternate circular arch family inspected; use the cubic architecture lancet for this mode |
| `gothic/src/lib/legacy/4_frames.js:132-146,246-365` | `corners.cusp`, `rails.double`, `plan`, `compose`, mirrored corner geometry and delays |
| `gothic/src/pages/7_arches.tsx`, `8_frames.tsx`, `10_guilloche.tsx`, `11_architecture.tsx` | Notebook entry points; frame composition and independent architectural studies |
| `src/registry.rs:60-118` | `Param`, `AnimKind`, `ModeFrame`, `Mode` contract |
| `src/modes/_57_reef.rs:52-105,1270-1314` | Parameter precedence, file-owned mode, deterministic frame helper/snapshots |
| `src/morph.rs:208-271,1211-1248,1293-1928` | Retained renderer, subprocess frame env, supervisor entry, worker state/events |
| `src/opts.rs:54-91,343-371,425-542,729-973` | Thread-local inputs, deterministic knobs, picker, demo and animation launch |
| `src/_1_playback.rs:31-44,102-115,184-268,385-442,631-701` | Exit/control enums, immediate quit, input serialization, worker reader, partial output regression |
| `scripts/0_generate_modes.sh` | Numeric generated registry order and `--check` mode |
| `scripts/12_test_e2e.py:206-365,415-470` | `DemoCase.run`, `workflow`, `knob_and_roll`, suite roster, CLI cases |
| `scripts/16_headless_e2e.py:18-104` | `HeadlessCase(base.DemoCase)` and separate suite roster |
| `perf/0_E2E.md` | GUI/headless distinction, watchdog constraints, evidence and existing failures |

## 5. Deterministic tests and acceptance cases

### Art tests, colocated in `_59_gothic_trace.rs`

Use a single shared frame helper modeled on `_57_reef.rs`'s `frame`: fixed `make_palette(seed)`, `StdRng::seed_from_u64(seed)`, explicit `param_values`, empty args, and a blank grid. Flatten through existing `render::grid_to_plain(&grid).join("\n")`. Snapshot paths belong under `src/modes/snapshots/` with the module's normal insta prefix. Proposed names:

1. `gothic_trace_lancet_seed42_t0` at 80x24, seed 42, defaults: recognizable guide geometry.
2. `gothic_trace_lancet_seed42_t1` at 80x24, time 1: partial strokes and pen tips.
3. `gothic_trace_lancet_seed42_t4_5` at 80x24, time 4.5: completed default drawing during hold.
4. `gothic_trace_lancet_seed42_t6` at 80x24, time 6: wrapped short traces.
5. `gothic_trace_rosette_seed42_t4_5` at 80x24, `GT_SCENE=1`: full woven rosette/frame.
6. `gothic_trace_rosette_seed43_t4_5` with seed 43: inspect phase and rotation variation.
7. `gothic_trace_tiny_gallery`: one snapshot concatenating 1x1, 2x3, 7x4, 20x8 renders; no indexing panic. Separately assert empty dimensions return empty grids.

One deterministic non-snapshot test covers cache correctness by comparing full `Grid` values: A(seed42,t1) -> B(seed43,t6) -> A again must match; t1 and t4.5 must differ; changing draw/speed must not change built geometry; named palette must change colors without changing glyph geometry at full hold. Use deterministic geometry key/stroke dumps where necessary, not only a pointer assertion.

One mathematical test snapshots exact rounded outputs for a synthetic unequal-length polyline: at fraction .5 its tip must lie halfway by distance, not at the middle vertex; clipping/wrap keeps the final and initial intervals without drawing across disconnected strokes. Check cubic endpoints, closed guilloche endpoints, easing endpoints, and a time+cycle match with tolerance appropriate to f32. Test explicit NaN/infinity params fall back to defaults.

### Picker tests in the existing `opts.rs` test module

`demo_picker_wraps_through_cancel`: snapshot the complete selection timeline for match counts 0, 1, and 3, calling the helper with `None` between every key to model the next render loop. For three matches: `0 -> Up -> 3 -> redraw -> 3 -> Down -> 0`; also `2 -> Down -> 3 -> Down -> 0 -> Up -> 3`. For zero matches every movement remains zero. Include query narrowing after cancel (`sel=3`, matches=1 => cancel index 1), then typing resets to 0. Verify filtered indices with a literal roster and mixed-case query through `demo_filter_modes`, and Enter uses `filtered.get(sel)` only for `sel<filtered.len()`.

### Seed tests in the existing `morph.rs` test module

`animation_seed_controls`: table/inline snapshot of input sequences and `(SeedInput, edit)` after each event. Cover Press/Release, pane open/closed, Enter with injected random=7, collision current=7 =>8, 0 wrapping down to `u64::MAX`, max wrapping up to0, `e123<Enter>`, max decimal value, overflow, empty input, backspace, e-cancel, and digits being consumed instead of changing strategies. Quit events must pass through the helper.

`animation_reseed_renderer`: compare a reused renderer after `reseed(123,theme)` with a fresh renderer at seed123/time0 for gothic-trace and one existing registered mode (`illuminarium`, already used by nearby retained-renderer tests). Compare full grid and palette, auto and `deep` named theme. Render at later time, reseed, and prove old grid/time content does not remain.

`animation_geometry_seed_and_roll`: use `mode_spec("gothic-trace")`, explicit manual values, and seeds produced by the real `animation_seed_input` helper. Snapshot `(seed,roll,randomize,eff)` for `(42,0,false)`, `(123,0,false)`, `(123,0,true)`, `(123,1,true)`, `(124,1,true)`, and a return to `(123,1,true)`. Obtain `eff` with existing `effective_pvals`; do not duplicate its hash. Require exact restoration on the repeated tuple and retained manual values when randomization is off. Worker mutation/history, endpoint, resize, and clock assertions belong in the actual-worker E2E below, so a second test implementation of the player transitions cannot mask a production wiring error.

Extend `_1_playback.rs::tests::controls_are_collected_without_abandoning_frames_and_disconnected_workers_stop` with a short ordered stream `e`, `1`, `2`, `3`, Enter encoded as `Control::Input`. Assert exact decoded order and partial-write byte preservation. Keep its existing backpressure/disconnection assertions. Do not infer real terminal responsiveness from this test.

### Real terminal E2E

Add two bounded functional cases, `seed-search` and `seed-state`, to the existing harness. Their mode is supplied by `--mode` like existing cases. Each gets its own application process, evidence directory, and unchanged 15-second watchdog. Concrete edit points:

- `scripts/12_test_e2e.py`: add both case choices in `main`, both `suite` rosters, `DemoCase.run`'s 160x40 functional-size branch, functional/no-cadence-gate classification, and new `async def seed_search(self)` / `async def seed_state(self)` methods. The functional set is exactly `('workflow','seed-search','seed-state')`; only bad/max cases use the large cadence gate. Use existing `key`, `screen_until`, `wait`, `frames`, `capture`, and `checkpoint` methods. Retain startup input assertions, PNG motion evidence, profile, q return, exit, and termios checks.
- Before selecting the filtered mode in `DemoCase.run`, for `seed-search`: type the complete mode query; wait; send Up and assert the selected marker is on cancel; send Down and assert the marker is on the matching mode; Enter must select that mode. Check the selected marker/highlight, not mere presence of the name.
- `seed_search`: seed42 initially; pane open; send `e123\r`, wait for traced seed123 and visible `seed:123`; Enter randomizes with seed !=123 and <10000, roll unchanged; `g` then `e123\r`, record deterministic random knobs at seed123/roll0; Right rerolls while seed remains123; `e124\r` changes seed with that roll unchanged. Set back to123 and require exact restoration of the previous seed123/roll1 effective values. Close pane then Up, reopen and verify Up selects a knob without reseeding. Enter valid typed seed uses no intermediate strategy changes. End using existing q/termios workflow.
- Keep random seed assertions tied to frame inputs; the sampled number is deliberately nondeterministic, while committed seed123 plus roll is exact. Require advancing frame time when playing after each seed restart. Seed edits while paused keep time0 until resumed. Capture visible art before/after, plus frame input evidence.
- `seed_state`: pause and wait for stable time; `e123\r` must produce time0/seed123/target124, then `e124\r`, then `b` must restore seed123 and the original roll/manual knobs. Native `n` must produce seed124/target125/time0. Resize to180x45 and require seed124 in frame inputs/footer with grid146x44. Send `1` for dissolve, capture its phase0 endpoint grid, then `e125\r` and require seed125/target126/time0 plus changed captured art. Send `i` and require native seed125; resume and require increasing time. Randomization stays disabled for this case so the initial ghost remains visible. End using existing q/termios workflow; assert returned demo seed42 per the session-lifetime decision.
- `scripts/16_headless_e2e.py::run`: add both cases to its all-cases roster; the existing `HeadlessCase` inherits the shared workflow. Its cell snapshots prove application/terminal output only.
- `scripts/12_test_e2e_test.py`: add checks that both new cases appear as `not_run` when a preceding case trips the guard; retain existing evidence/watchdog tests. `perf/0_E2E.md`: document both new cases and the animation-session lifetime of seed changes.

Use the existing 15-second per-case watchdog, owned RSS, terminal RSS, artifact size, observer heartbeat, focus checks, and independent cleanup. Do not raise limits. Include both new cases in all-case rosters/report placeholders. Stop after a safety trip, mark remaining cases `not_run`, preserve exact keys/inputs/artifacts, and analyze evidence before another stress attempt.

## 6. Numbered implementation sequence and commands

Commands below are for Luna's later implementation. None was executed during planning.

1. Read `AGENTS.md`, `git status --short`, and this plan. Verify available `_59` filename with `rg --files src/modes -g '_[0-9]*_*.rs'`. Reserve `_58` even when absent. If `_59` has appeared meanwhile, use the next available unreserved number and update mode/test names consistently. Do not load skills/archives or delegate for this plan handoff.
2. Implement the picker helper/clamp correction and focused deterministic picker test. Run `cargo test demo_picker_wraps_through_cancel`. Commit only that fix/test as a milestone.
3. Implement the worker seed prompt, current seed/target state, renderer reseed, history, footer/log updates, and resolved child env ordering. Preserve existing art implementations and transport. Add focused seed and control-transport tests. Run `cargo test animation_seed`, `cargo test animation_reseed_renderer`, `cargo test animation_geometry_seed_and_roll`, and `cargo test controls_are_collected_without_abandoning_frames_and_disconnected_workers_stop`. Commit the control fix/tests as a milestone after checks.
4. Add `_59_gothic_trace.rs`, implement its geometry and reveal, add fixed-seed tests. Run `scripts/0_generate_modes.sh`, then `scripts/0_generate_modes.sh --check`. Review generated changes: only the new module and registration, plus preserve any concurrently added existing mode registrations. Do not hand-edit `src/modes/mod.rs`; no art dispatch branches in main/cli/opts/registry/morph.
5. Run `cargo test gothic_trace`. Inspect every new `.snap.new` as plain ASCII and verify the prescribed partial/full/tracing views. Use `cargo insta review` for individually reviewed files, then rerun `cargo test gothic_trace`. Never bulk-accept unreviewed or unrelated snapshots. Commit new mode, generated registry, and reviewed snapshots as a milestone.
6. Add `seed-search` and `seed-state` to the shared E2E harness and its documentation. Run `python3 scripts/12_test_e2e_test.py` and `python3 scripts/5_probe_guard_test.py`; run `cargo fmt --check`, `scripts/0_generate_modes.sh --check`, and `cargo test`. Fix failures only within this scope; report pre-existing failures with evidence. Commit harness work when checked.
7. Run the real GUI functional checks with iTerm2 running, Python API/capture permissions enabled, and the case window focused:

   ```sh
   scripts/13_e2e.sh --case seed-search --mode gothic-trace
   scripts/13_e2e.sh --case seed-state --mode gothic-trace
   scripts/13_e2e.sh --case workflow --mode gothic-trace
   scripts/13_e2e.sh
   ```

   The final command exercises the full existing/default gem-aetherium-2 suite plus both new cases. It builds the release binary and wraps live cases in the existing watchdog. Default runs remain required to catch controls regressions outside the new art. No unmanaged `cargo run ... demo` stress probes.
8. Optional additional headless evidence, reported separately:

   ```sh
   scripts/13_e2e.sh --headless --case seed-search --mode gothic-trace
   ```

   Headless execution does not satisfy the GUI runs in step 7. Do not run `scripts/2_test_quit.py` bare: it launches a 2000x1000 workload; the mandated guarded E2E suite already covers cancellation/terminal restoration. If that supplemental legacy test becomes necessary, use an external watchdog with the existing limits and report its distinct scope.
9. Review staged diffs by path, commit remaining in-scope validation/documentation changes, and return actual test/snapshot/E2E statuses and artifact locations. No broad refactor, existing mode edits, or push requested. A failed or unexecuted terminal case cannot be described as a full-suite pass.

## 7. Planning validation and handoff status

Completed: read worktree `AGENTS.md`; inspected scoped reference sources, actual mode interface, current highest mode filename, retained renderer, seed/nonce paths, picker loop, supervisor pipe/event handling, generator, snapshot conventions, E2E runner and safety contract. Verified source paths and existing symbol names against files. `gothic/src/algos/9_architecture.ts` and legacy `4_frames.ts` do not exist; the plan uses the inspected `4_architecture.ts` and `legacy/4_frames.js` instead. No external imported library source was read.

Not run in this planning turn: all Rust builds/tests, new snapshots, snapshot review, Python tests, registry generation, GUI `backpressure`, `workflow`, `bad-400x200`, `max-400x200`, proposed `seed-search` and `seed-state`, and all headless variants. Existing failures described in `perf/0_E2E.md` were read as historical evidence and were not reproduced or resolved here.

Commit only `plans/0_gothic_art_and_demo_fixes.md`. Parent launches Luna after this plan commit. Stop after reporting status, commit SHA, file, validation, and next action.
