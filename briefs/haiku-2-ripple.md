# haiku-2-ripple

Concentric hyperbolic ripples expanding and contracting from a point source, with traveling particles that trace the wave field. Subject: a liquid surface disturbance viewed from above, animated as outward-and-back interference patterns.

## What moves and why

The ripple centers are fixed. Each concentric circle animates in radius via a sinusoidal wave driven by time `t`, creating the illusion of a propagating disturbance. Particles (secondary glyphs) follow level curves of the wave field with phase offsets, making the wave structure legible through motion. Seeds control: which concentric circle widths are active (frequency content), the hue of the primary and secondary palettes, and particle density.

## Glyph families

- Primary circles: `. * o O #` (wave peaks and troughs)
- Particles: `~ + x` (following the wave)
- Accents: `- = _` (cross-current flows)
- Background: ` ` (blank)
- Structural guides: `. , ; :` (faint grid)

## Knobs (min 6)

1. SPEED: wave propagation speed (0.3 to 3.0, default 1.0)
2. HUE: palette hue offset in HSL (0 to 360, default 200)
3. FREQ: primary ripple frequency, higher = more rings (2 to 12, default 5)
4. DECAY: how fast amplitude fades from center (0.1 to 1.0, default 0.5)
5. PDENSE: particle density multiplier (0.1 to 1.2, default 1.0)
6. WFORM: waveform type 0=sine 1=square 2=sawtooth (0 to 2, default 0)

## Positional order

`ascii-renderer 42 haiku-2-ripple moss [speed] [hue] [freq] [decay] [particle_density] [waveform]`

## Render commands

Static (t=0):
```
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 7 haiku-2-ripple moss | sed 's/\x1b\[[0-9;]*m//g'
```

Animated (t=12):
```
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=12 ./target/release/ascii-renderer 7 haiku-2-ripple moss | sed 's/\x1b\[[0-9;]*m//g'
```

## Perf receipt

Knob sweep at 2000x1000, worst case FREQ=12 at 29.4 fps (all knobs above 29 fps):

| knob at max | frames | fps | avg ms |
| --- | ---: | ---: | ---: |
| baseline | 31 | 30.7 | 32.53 |
| FREQ=12 | 30 | 29.4 | 34.05 |
| PDENSE=1.2 | 30 | 29.6 | 33.81 |
| HUE=360 | 31 | 30.1 | 33.27 |
| SPEED=3 | 31 | 30.4 | 32.91 |
| DECAY=1 | 31 | 30.8 | 32.42 |
| WFORM=2 | 42 | 41.5 | 24.08 |

Hotspot at FREQ=12: waves layer 80.2% of frame, particles 14.4%
