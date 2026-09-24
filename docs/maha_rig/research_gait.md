# Mahoraga rig: gait and proportion research

Target: a large athlete, part decathlete and part Olympic lifter, scaled up uniformly. He walks calmly toward the camera.
Code audited: `examples/3_maha_rig.rs` (`RIG`, `shapes()`, `KEYS`) and `examples/maha_skin.html` (`SPEC`, `walkRots`, `footPath`, `ridingHipY`, `solveLeg`).

## 0. Unit conventions used below

| symbol | meaning | value in rig |
|---|---|---|
| u | rig unit | |
| H | stature, sole to vertex | **measured 12.3 u** (sole -0.05, skull top = crown joint 11.4 + r 0.84 = 12.24). The brief said 11.5. |
| h | hip-joint height above the sole. Alexander and Donelan call this "leg length". | 5.55 u (pelvis 5.8 - 0.3, sole -0.05) |
| L_ik | `legLength()` = THIGH + SHIN | 5.1 u. Note `strideRatio` is relative to L_ik, not h: `strideRatio = 1.088 * (stride/h)` |
| cm to u | normalise by leg length, human h ≈ 0.93 m (Donelan 2001 subjects) | 1 cm ≈ 0.060 u |
| gait % | 0 = right heel strike. `phase` 0 in the viewer is the same event. | |

The extra 0.8 u of stature comes almost entirely from the head (section 1). Below the chin, the rig is consistent with **H ≈ 10.5 to 10.9**. The targets below use **H = 11.5**, which assumes a normal-sized head. All targets scale linearly with H.

---

## 1. Segment proportions

Sources:
- Drillis & Contini 1966, as tabulated in Winter, *Biomechanics and Motor Control of Human Movement*, 4th ed. 2009, Fig. 4.1 / Table 4.1 (the "fraction of H" column below).
- ANSUR II 2012 US Army anthropometry, male means, for the breadths and girths Winter omits (biacromial ≈ 0.23H, bideltoid ≈ 0.28H, head breadth ≈ 0.089H).
- Heavy-athlete adjustments: Carter, *Physical Structure of Olympic Athletes* (Montreal Olympic Games Anthropological Project, 1984), and Keogh et al. 2007, *J Sports Sci* 25:1365 (powerlifters). Weightlifters show relatively broad shoulders and chest, large girths, and slightly short limbs relative to trunk. Decathletes are close to population proportions but tall with long legs. The blend is population proportions, about +8% on breadths, and girths at roughly the 95th percentile.
- Hip-joint-centre spacing: Bell, Pedersen & Brand 1990, *Hum Mov Sci* 9:3 (HJC ≈ 0.14 × inter-ASIS width medial of each ASIS), which gives about 0.09 to 0.10H.

Girth targets assume a 1.90 m, about 110 kg athlete. Girths (circumferences) are converted to radius as C/2π.

### 1a. Lengths and heights (rig viewer, including `shoulderDrop` 0.25)

| segment | fraction of H | target @ H=11.5 | current rig | current / target | implied H | verdict |
|---|---|---|---|---|---|---|
| head, chin to vertex | 0.130 | 1.50 | 2.79 (jaw cone bottom 9.45 to skull 12.24) | **1.86** | 21.5 | **head far too big** |
| head breadth | ≈0.089 (ANSUR) | 1.02 | 1.68 (crown r 0.84) | **1.65** | | too big |
| neck, chin to shoulder | 0.052 | 0.60 | 0.80 | 1.33 | | ok (traps hide it) |
| shoulder (acromion) height | 0.818 | 9.41 | 8.70 | 0.92 | 10.6 | ok vs trunk |
| trunk, hip joint to shoulder joint | 0.288 | 3.31 | 3.15 | 0.95 | 10.9 | ok |
| upper arm | 0.186 | 2.14 | 2.35 | 1.10 | 12.6 | slightly long |
| forearm | 0.146 | 1.68 | 2.20 | **1.31** | 15.1 | **long** |
| hand (wrist to fingertip) | 0.108 | 1.24 | 1.28 (0.8 + fist r 0.48) | 1.03 | | ok |
| fingertip height | 0.377 | 4.34 | 2.82 | 0.65 | | **fingertips hang at the knee (2.9): ape arms** |
| hip-joint height h | 0.530 | 6.10 | 5.55 | **0.91** | 10.5 | legs 9% short |
| thigh | 0.245 | 2.82 | 2.60 | 0.92 | | short |
| shank | 0.246 | 2.83 | 2.50 | **0.88** | | short |
| ankle height | 0.039 | 0.45 | 0.45 | 1.00 | | ok |
| foot length | 0.152 | 1.75 | ≈1.57 (heel -0.45 to toe 0.8 + 0.32) | 0.90 | | slightly short |

### 1b. Breadths and girths

| measure | population | heavy-athlete target | target @ H=11.5 | current rig | current / target |
|---|---|---|---|---|---|
| biacromial | 0.23H (ANSUR); Winter "shoulder width" 0.259H | ≈0.25H | 2.9 | | |
| glenohumeral joint-centre spacing | ≈ biacromial - 2×2 cm ≈ 0.20H | ≈0.21H | **±1.2** | `r_shoulder` x = ±2.3 | **1.9** |
| bideltoid (outer delts) | 0.28H | ≈0.30 to 0.31H | 3.5 | 6.45 (mesh at y=9) | **1.84** |
| chest breadth | 0.19H | ≈0.21H | 2.4 | ≈4.2 (chest r 1.25 + pec cones to x 2.1) | **1.75** |
| waist breadth | 0.16H | 0.16H | 1.84 | 1.9 (spine r 0.95) | 1.03 |
| hip breadth (bitrochanteric) | 0.191H (Winter) | ≈0.20H | 2.3 | 3.3 (hip x 0.8 + r 0.85) | **1.43** |
| hip-joint-centre spacing | ≈0.09 to 0.10H | 0.10H | **±0.58** | `r_hip` x = ±0.8 | **1.38** |
| neck radius | C ≈ 0.23H | r 0.037H | 0.42 | `neck` r 0.6 | 1.4 |
| upper-arm radius, mid | C ≈ 0.21 to 0.24H (40 to 45 cm) | r 0.034 to 0.038H | 0.39 to 0.44 | ≈0.71 effective (`r_elbow` 0.55 capsule + biceps 0.64 @ z+0.21 + triceps 0.52 @ z-0.18 gives 1.3 × 1.55 section) | **1.7** |
| forearm radius, max | C ≈ 0.175H | r 0.028H | 0.32 | 0.60 (forearm cone) | **1.9** |
| wrist radius | C ≈ 0.10H | r 0.016H | 0.18 | `r_wrist` 0.44 (forearm core) | 2.4 |
| upper thigh / glute radius | C ≈ 0.38H | r 0.06H | 0.69 | `r_hip` 0.85 | 1.23 |
| mid-thigh radius | C ≈ 0.34H (64 cm) | r 0.054H | 0.62 | `r_knee` 0.62 | **1.00** |
| calf radius, max | C ≈ 0.22H (42 cm) | r 0.035H | 0.40 | `r_ankle` 0.45, constant along the shank | 1.1. No calf belly and no ankle taper; the ankle should be about 0.22. |

**Reading:** the lower body is a real heavy athlete (thigh girth exact, legs about 9% short). The upper body and head are a roughly 1.7 to 1.9× caricature:
- shoulders 1.9× too wide
- arm girth 1.7 to 1.9×
- head 1.8×
- forearms 1.3× too long

That is the gorilla / bodybuilder read the brief wants to avoid.

The viewer knob ranges cannot reach these targets:
- `shoulderWidth` min is 0.85; the target is 0.52.
- `headSize` min is 0.8; the target is about 0.55.
- `armLen` min is 0.85; the target is about 0.84.
- `legLen` 1.09 is in range.

Suggested `RIG` edits at H = 11.5, if anatomy wins:

| joint | offset now | offset target | radius now | radius target |
|---|---|---|---|---|
| r/l_shoulder | (±2.3, 0.2, 0) | (±1.25, 0.35, 0) | 0.7 | 0.45 |
| r/l_elbow | y -2.35 | y -2.14 | 0.55 | 0.40, plus biceps/triceps cones ×0.6 |
| r/l_wrist | y -2.2 | y -1.68 | 0.44 | 0.25, forearm cone 0.34→0.22 |
| r/l_hip | (±0.8, -0.3, 0) | (±0.6, -0.3, 0) | 0.85 | 0.70 |
| r/l_knee | y -2.6 | y -2.82 | 0.62 | 0.62 |
| r/l_ankle | y -2.5 | y -2.83 | 0.45 | 0.40, tapering to 0.22 |
| r/l_toe | (0, -0.3, 0.8) | (0, -0.3, 0.95) | 0.32 | 0.32 |
| head / crown | 0.74 / 0.84 | scale the head group ×0.55 | | |
| `shapes()` delt cone / cap | 0.72 / 0.55 | 0.45 / 0.35 | | |
| `shapes()` pec cone | to x 1.4, r 0.8 | to x 0.85, r 0.5 | | |

Pelvis `y` rises from 5.8 to about 6.4 with the longer legs.

---

## 2. Walking gait kinematics (normal adult; slow/calm values in the last column)

Sources:
- Perry & Burnfield, *Gait Analysis: Normal and Pathological Function*, 2nd ed. 2010
- Winter, *The Biomechanics and Motor Control of Human Gait*, 2nd ed. 1991
- Kadaba, Ramakrishnan & Wootten 1990, *J Orthop Res* 8:383
- Lelas et al. 2003, *Gait Posture* 17:106 (how peaks scale with speed)
- Murray, Drought & Kory 1964, *JBJS* 46A:335
- Saunders, Inman & Eberhart 1953, *JBJS* 35A:543

| quantity | normal range | peaks (% cycle) | slow / calm |
|---|---|---|---|
| **hip** flex/ext (thigh vs pelvis) | +30° IC, -10° peak ext, +35° peak flex. Range ≈40 to 45°. Thigh vs vertical ≈ this minus ≈10° pelvic tilt: +20 / -18 / +25. | ext peak 50 to 55%; flex peak ≈85% | range ≈35°, extension only -5° |
| **knee** flexion | 5° IC; **15 to 20° loading peak**; ≈5° at midstance; ≈40° at toe-off; **≈60° swing peak**; ≈2° before IC | LR peak 12 to 15%; min ≈40%; toe-off 60%; swing peak 72 to 73% | LR peak 10 to 15°; swing peak 55 to 60° |
| **ankle** (+dorsi) | 0° IC; -5 to -7° at foot-flat; +10° peak dorsi; -15 to -20° peak plantar; ≈0° in swing | foot-flat 7 to 10%; dorsi peak 45 to 48%; plantar peak 62 to 65%; neutral by 75 to 85% | plantar peak ≈ -12° |
| foot-to-floor angle | toe-up 15 to 25° at IC (heel rocker); heel-off ≈35 to 45%; ≈35 to 50° at toe-off (forefoot rocker) | | toe-up 10 to 15° at IC |
| **pelvic rotation** (transverse) | ±3 to 5°; the leading-leg side is forward at IC | extremes at IC (0%, 50%) | ±2 to 3° |
| **pelvic obliquity** (swing side drops) | ±3 to 5° | **stance side high at ≈15 to 20%** (just after contralateral toe-off); low at 65 to 70%; neutral ≈40% / 90% | ±2 to 3° |
| pelvic tilt (sagittal) | ±1 to 2° about ≈10° anterior; **twice per stride** | anterior peaks ≈45%, 95% (terminal stance) | smaller; can be ignored visually |
| **thoracic rotation** | counter to the pelvis at normal speed, ±3 to 6° in space. Thorax-pelvis relative phase is ≈110° at fast speeds but **≈25° (nearly in phase) at slow speeds** (van Emmerik & Wagenaar 1996, *J Biomech* 29:1175). Stokes, Andersson & Forssberg 1989, *J Biomech* 22:43. | | ±2 to 4°, small counter-rotation |
| trunk lateral lean | ±1 to 3° toward the stance leg | ≈20 to 30% | |
| **arm swing**, shoulder | total 20 to 35°, **biased backward** (more extension than flexion). Murray, Sepic & Barnard 1967, *Phys Ther* 47:272; Collins, Adamczyk & Kuo 2009, *Proc R Soc B* 276:3679. | forward peak ≈ contralateral IC | total 15 to 20°. Below ≈0.8 m/s the arms switch to step frequency, 2:1 (Wagenaar & van Emmerik 2000, *J Biomech* 33:853). |
| arm swing, elbow | ≈10 to 15° at back swing → ≈30 to 40° at forward swing (approx.) | follows the shoulder | less |
| **head** | held within about ±2 to 3° in yaw and roll relative to space; pitch counters vertical bob (Pozzo, Berthoz & Lefort 1990, *Exp Brain Res* 82:97) | | same |
| **lateral COM** p-p | **7.0 cm at 0.7 m/s → 3.9 cm at 1.6 m/s** (Orendurff et al. 2004, *JRRD* 41:829). It gets larger when slower. | toward the stance foot, peaks at midstance ≈30% / 80% | ≈6 to 7 cm = 0.36 to 0.42 u |
| **vertical COM** p-p | **2.7 cm at 0.7 m/s → 4.8 cm at 1.6 m/s** (Orendurff 2004). High at midstance, low in double support. | max ≈30 / 80%; min ≈5 / 55% | 2.7 to 3.5 cm = **0.16 to 0.21 u (2.9 to 3.8% of h)** |
| stance / swing | 60 / 40; double support ≈10% × 2 | toe-off at 60 to 62% | 63 to 65 / 35 to 37; double support 26 to 30% total |
| **step width** | **0.13h preferred** (Donelan, Kram & Kuo 2001, *Proc R Soc B* 268:1985; ≈ foot width 0.11h) | | same, or slightly wider |
| cadence, slow | 90 to 100 steps/min (normal 105 to 115); stride about 1.1 to 1.2 m at 1.75 m stature. Oberg, Karsznia & Oberg 1993, *J Rehabil Res Dev* 30:210. | | |

### 2a. Scaling for a giant (dynamic similarity)

Dynamic similarity:
- Froude number: Fr = v²/(g·h).
- Equal Fr gives equal relative stride. Alexander 1976, *Nature* 261:129: **stride/h = 2.3·Fr^0.3**.
- Therefore cadence ∝ 1/√h and speed ∝ √h (Alexander 1989, *Physiol Rev* 69:1199).
- Observers read body size from stride frequency with this inverse-square law (Jokisch & Troje 2003, *J Vision* 3:252). **Cadence is the size cue.**

A calm walk is Fr ≈ 0.08 to 0.12, which is 0.85 to 1.05 m/s for a normal human, just above the 2:1 arm-swing transition.

| Fr | stride/h | `strideRatio` (= ×1.088) |
|---|---|---|
| 0.02 (current rig) | 0.72 | 0.78 |
| 0.08 | 1.08 | 1.17 |
| 0.09 | 1.12 | 1.22 |
| 0.12 | 1.22 | 1.32 |

`speed` (cycles/s) for Fr 0.09 with `strideRatio` 1.15: **speed ≈ 0.89/√h_m** (cadence = 120 × speed).

| real hip height h_m | stature ≈ h/0.53 | `speed` | cadence (steps/min) |
|---|---|---|---|
| 1.0 m | 1.9 m | 0.89 | 107 |
| 1.5 m | 2.8 m | 0.73 | 87 |
| 2.0 m | 3.8 m | 0.63 | 76 |
| 2.5 m | 4.7 m | **0.56** | 67 |
| 3.0 m | 5.7 m | 0.51 | 62 |
| 4.0 m | 7.5 m | 0.44 | 53 |

The current settings (`speed` 0.72, `strideRatio` 0.78) are a **Fr ≈ 0.02 shuffle**. Solving both constraints together gives h ≈ 0.75 m, so the gait reads as a small person shuffling, not a giant. The fix is a longer stride and a lower cadence together.

---

## 3. Masculine vs feminine gait markers

| marker | men | women | source |
|---|---|---|---|
| pelvic obliquity range | smaller | **larger** | Smith, Lelas & Kerrigan 2002, *J Women's Health Gend Based Med* 11:453 |
| vertical COM excursion | **larger** | smaller | Smith et al. 2002 |
| lateral sway location | **upper body / shoulders sway**, "straddle-legged, elbows held away from the body" | little upper-body lateral motion, **hip rotation and hip sway** | Troje 2002, *J Vision* 2:371; Mather & Murdoch 1994, *Proc R Soc B* 258:273 |
| rotation emphasis | shoulder rotation ≥ pelvic rotation ("shoulder swagger") | pelvic rotation > shoulder rotation ("hip sway") | Johnson & Tassinary 2005, *Psychol Sci* 16:890 |
| arm carriage | abducted, elbows out, little crossing | close to the body, more crossing toward the midline | Troje 2002 |
| step width | wider (straddle) | narrower; feet near or crossing the midline | Troje 2002 |
| cadence / stride | lower cadence, longer absolute stride | higher cadence | Cho, Park & Kwon 2004, *Clin Biomech* 19:145 |

**Calm, heavy, masculine targets:**
- Pelvic obliquity ±2 to 2.5°, with the trunk not following it.
- Pelvic rotation ±3°, with thorax rotation larger at ±5 to 6° in space.
- Whole-body lateral shift that carries the shoulders over the stance foot (shoulders move at least as much as the hips).
- Vertical bob present, about 3 to 4% of h.
- Arms abducted 12 to 15°, with crossing ≤3°.
- Step width 0.18 to 0.2h.

---

## 4. Heavy / large-mass walking

| effect | direction | evidence | note for Mahoraga |
|---|---|---|---|
| step width | wider | Spyropoulos et al. 1991, *Arch Phys Med Rehabil* 72:1065 (obese men); Browning & Kram 2007, *Med Sci Sports Exerc* 39:1632 | Thigh girth alone forces it. Knee-centre gap must be ≥ 2 × thigh r at the knee (1.24 u). Target **0.18 to 0.20h ≈ 1.0 to 1.1 u**. |
| stance / double support | longer | Spyropoulos 1991; load carriage: Kinoshita 1985, *Ergonomics* 28:1347 | duty 0.64 to 0.65 |
| stride (relative) | 5 to 10% shorter at the same Fr; lower speed | Spyropoulos 1991; Browning & Kram 2007 | `strideRatio` ≈1.15, not 1.25 |
| cadence | lower in absolute terms because the body is big (pendulum, §2a). Mass itself does not enter the Froude number. | Alexander 1989 | set from the §2a table |
| pelvic obliquity | smaller (wide stance, abductor load) | inferred from the step-width data; masculine data in §3 | ±2 to 2.5° |
| trunk counter-rotation | **not larger in angle**. A heavy thorax and arms carry more inertia, so less angle cancels the same leg angular momentum (Bruijn et al. 2008, *Gait Posture* 27:455). The brief's "more counter-rotation" has no support. | | thorax ±5 to 6° |
| arm swing | smaller angle (heavy arms), carried abducted to clear the lats and deltoids | inferred; Collins 2009 for arm-swing mechanics | shoulder total ≈18°, abduction 12 to 15° |
| trunk lean | slightly forward, 2 to 3° | load carriage literature | `lean` 2 |

Most "heavy person" data comes from obesity and load-carriage studies. For a muscular (not adipose) body, only the geometric effects carry over directly: step width from thigh girth, and arm abduction from lat and deltoid girth.

---

## 5. Mapping to `SPEC` knobs

The "now" column is the default. Units follow `SPEC`.

| knob | now | recommend | reasoning / source |
|---|---|---|---|
| `speed` | 0.72 | **0.55** (≈4.8 m giant); use the §2a table for other sizes | Froude scaling; Jokisch & Troje 2003 |
| `strideRatio` | 0.78 | **1.15** | Alexander 1976 at Fr ≈0.08 to 0.09, -5% for heavy |
| `duty` | 0.62 | **0.64** | slow walk 63 to 65%; heavy walkers get longer stance |
| `liftRatio` | 0.11 | **0.09**, only after the lift-skew fix (S5); peak should land ≈30% into swing | Winter 1992, *Phys Ther* 72:45 (foot trajectory, toe clearance 1 to 2 cm at mid-swing) |
| `reach` | 0.97 | **0.998** (knee ≈7°), and replace it with a stance knee profile (S2) | 0.97 locks the stance knee at **28°** all through stance; real midstance is ≈5° |
| `autoHeight` | 1 | 1; fix the blend target (S7) | |
| `footOut` | 0.10 | **-0.25** (foot x = ±0.55, step width 1.1 u = 0.2h). With hips moved to ±0.6: **-0.05** | Now the step width is 1.8 u = 0.32h, 2.5× Donelan's 0.13h, with the feet outside the hips |
| `toeLift` | 12 | redefine as heel-strike toe-up angle, **12°** at IC (S4) | heel rocker, Perry |
| `toeRoll` | 14 | **30°** at toe-off, pivoting on the ball (S3) | forefoot rocker; produces the 35 to 40° toe-off knee |
| `weightShift` | 0.08 | **0.18** (p-p 0.36 u ≈ 6 cm human) | Orendurff 2004, slow speed. Whole-body shift moves the shoulders too, so it reads as masculine, not as hip sway. |
| `hipDrop` | 1.25 | **2.25**, after the phase fix (S6) | men ±3 to 4° normal; calm/heavy lower; Smith 2002 |
| `counterLean` | 0.3 | **1.3** (net trunk tilt ≈+0.7° over the stance leg) | the thorax stays upright or leans to stance; shoulders carry the sway (Troje, Mather & Murdoch) |
| `bob` | 0 | 0 (geometry supplies it) | |
| `pelvisTwist` | 2.5 | **3** | ±3° slow walk (Murray 1964; Stokes 1989) |
| `torsoTwist` | 13 | **6** (thorax ±6° in space, > pelvis) | 13 is 2 to 4× the slow-walk thorax amplitude; masculine = shoulder > pelvis |
| `spineShare` | 0.7 | 0.6 | no data; lumbar rotation is small anatomically, so shift more into the thoracic spine |
| `lean` | 0.5 | **2** | heavy / loaded gait |
| `headStab` | 0.8 | **0.6** with `torsoTwist` 6, giving head ±2.4° in space | Pozzo 1990 |
| `armSwing` | 11 | **9** (±9°), plus the missing `armBias` | Murray 1967 slow; heavy arms |
| `armCross` | 5.5 | **3** | masculine; abducted arms |
| `armSpread` | 9.5 | **13** | lat and deltoid clearance; "elbows out" (Troje) |
| `elbowRest` | 5.5 | **12** | resting tone of massive elbow flexors; Murray 1967 minimum ≈10 to 15° |
| `elbowSwing` | 32.5 | **20** (forward peak ≈32°) | slow walk |
| `wristFlex` / `wristLag` | 8 / 10 | keep | no data |

### Missing knobs

| knob | why | default |
|---|---|---|
| `armBias` | Arm swing is biased backward (Collins 2009; Murray 1967). `walkRots` swings symmetrically about vertical. Add it to the shoulder pitch: `armBias + swing*f`. | -6° |
| `kneeLoad`, `kneeMid` | stance knee profile: 15 to 20° peak at ≈13%, ≈5° at midstance (Perry). Replaces the constant `reach`. | 15°, 5° |
| `heelStrike` | toe-up at IC, rolling to flat by ≈8% | 12° |
| `liftSkew` | shape exponent for swing lift | 0.6 |
| `dropPhase` | pelvic-obliquity peak timing, or hard-code it (S6) | 0.18 |
| `shoulderSway` | trunk lateral lean toward the stance leg, independent of `hipDrop`, in phase with `weightShift` | 1.5° |
| `trunkPhase` | Optional. Thorax-pelvis relative phase: 180° now; the slow-walk truth is nearer 25 to 90° (van Emmerik 1996). | 180 |
| head roll stabilisation | the neck cancels yaw only. Also cancel pelvis + spine roll so the head stays level. | on |

### Structural problems in `maha_skin.html`

The leg IK was re-simulated in 2D (sagittal plane) with the same equations. Results below; "LR" is the loading-response knee peak.

| config | knee IC | LR peak | midstance | toe-off | swing peak | bob p-p |
|---|---|---|---|---|---|---|
| **current defaults** | 28° | 28° (none) | 28° | 28° | 61° @81% | 0.16 u |
| current code, stride 1.25, reach 0.995 | 11° | **38°** @15% | 11° | 11° | 56° @82% | 0.43 u |
| **S1+S2+S3+S5 fixed**, stride 1.0 to 1.2, duty 0.64 to 0.65 | **5°** | **14 to 17° @13%** | **5°** | **37 to 41°** | **60 to 64° @76 to 77%** | 0.28 to 0.39 u |
| textbook (Perry / Winter) | 5° | 15 to 20° @12 to 15% | 5° | 40° | 60° @73% | 0.16 to 0.21 u (slow) |

**S1. `ridingHipY` constrains the hip with the swing leg during early swing.** Right after toe-off the swing foot is still far behind with little lift, so it pulls the hip down. With a real stride this bends the *stance* knee to 34 to 46° at ≈15%.
Fix: use stance legs only, plus the swing leg during its last 25% (v > 0.75) so the body drops into the next heel strike. Without that last-25% term the swing foot misses the landing by 0.2 to 0.3 u.

**S2. Constant `reach` equals constant stance knee flexion.** Knee flexion ≈ √(8(1 − reach)) rad: 0.97 → 28°, 0.99 → 16°, 0.995 → 11°, 0.998 → 7°.
Fix: per stance fraction s = u/duty, compute `reach(s) = sqrt(lt² + ls² + 2·lt·ls·cos k(s)) / L`, where k(s) is:
- 5° → `kneeLoad` at s = 0.2 (sine ramp),
- cosine back to `kneeMid` by s = 0.5,
- flat after that.

Use it inside `ridingHipY`.

**S3. `toeRoll` pivots the foot about the ankle** (the `heel` term in `solveLeg`). The toe sinks into the floor, the ankle never rises, and there is no pre-swing knee flexion (toe-off knee = stance knee).
Fix: in `footPath`'s stance branch, pivot on the ball:
- `peel = toeRoll·max(0, (s − 0.55)/0.45)²`
- `lift = BALL·sin(peel)`
- `z += BALL·(1 − cos peel)`, with BALL ≈ 0.8 u (the ankle-to-toe offset)

The swing branch then starts from that raised pose: lift `y0·(1 − v) + …`. `ridingHipY` already adds `f.lift`, so the trough rises and the knee reaches ≈40° at toe-off by itself.

**S4. The foot lands flat.** The swing `heel = toeLift·sin(π·swing)` is 0 at v = 1.
Fix: in swing, `heel = heelStrike·smooth(v)` (plus toe clearance). In stance for s < 0.15, `heel = heelStrike·(1 − s/0.15)`. Pivot about the heel point (0, -0.45, -0.3) in ankle space, which also lifts the ankle ≈0.07 u at IC and flattens the COM trough.

**S5. The swing lift `sin(π v)` peaks at mid-swing.** Real ankle lift peaks early, when the knee is at 60° (≈30% into swing), then flattens toward landing.
Fix: `sin(π · v^0.6)`.

**S6. The hip-drop phase is ≈13% late.** `walkRots` uses `w = cos(2π(ph − duty/2))`, which peaks at 31% (midstance). Obliquity peaks just after contralateral toe-off, at ≈15 to 20%. Keep `w` for `weightShift` (ML COM does peak at midstance). For `roll` use `cos(2π(ph − (K.duty − 0.45)))`, which peaks at ≈0.19 for duty 0.64. A drop timed at midstance reads as a sashay; timed at weight acceptance it reads as load.

**S7. `autoHeight < 1` blends toward `fixedHip`, the cycle maximum.** Legs then cannot reach the floor in double support and the IK clamps, so the feet float.
Fix: blend toward the cycle minimum of `ridingHipY`.

**S8. `ridingHipY` ignores pelvis roll, yaw and `weightShift`.** Obliquity raises the stance hip joint by 0.8·sin(hipDrop) ≈ 0.03 u. With a near-straight knee the IK clamps and the stance foot hovers.
Fix: compute the height for the stance *hip joint*, then place the pelvis centre from it through the pelvis rotation. This also makes obliquity lower the pelvis centre at midstance (Saunders' second determinant), which flattens the bob.

**Residual:** even with S1 to S5, the bob is 5 to 7% of h, against 2.9 to 3.8% measured in slow walking. S4 and S8 remove roughly another 0.1 u. Beyond that, the only lever left is lowering the midstance peak, and near-straight knees make that costly: every 0.05 u of lowering adds ≈10° of midstance knee flexion. Accept about 4 to 5%: a heavy masculine walk shows more bob (Smith 2002).

**Rust `KEYS` (`3_maha_rig.rs`):**
- `bob` ±0.25/0.2 → p-p 0.45 u (8% h): halve it.
- Pelvis twist ±8 → 3.
- Chest ±10 → 6.
- Shoulder swing ±30 → ±10 with a backward bias.

The knee values (-4 at plant, -24 trailing, -50 at passing) are fine.

---

## Top 5 changes, by visual impact

1. **Proportions.** Shoulders ±2.3 → ±1.25 (1.9× too wide). Arm and forearm girth down ≈40%. Forearm 2.2 → 1.7. Head ×0.55. Legs +9%. Knob ranges must widen to allow this. This single change decides between "big athlete" and "gorilla".
2. **Stride and cadence.** `strideRatio` 0.78 → 1.15 and `speed` 0.72 → 0.55. The current gait is a Fr 0.02 shuffle that reads small; this makes it read as a calm giant.
3. **Leg IK realism (S1 to S3, S5, and S2's `reach` 0.97 → knee profile).** This removes the constant 28° crouch and gives a heel-strike → load → straight → push-off → 60° swing knee.
4. **Stance and sway.** `footOut` 0.1 → -0.25 (feet under the hips, not outside). `weightShift` 0.08 → 0.18 and `counterLean` 0.3 → 1.3, so the shoulders carry the sway. `hipDrop` 2.25 with the phase fix (S6). This is the masculine lateral read.
5. **Upper-body rhythm.** `torsoTwist` 13 → 6 with `pelvisTwist` 3. Arms: swing 9° with a -6° backward bias, `armSpread` 13, `armCross` 3, `elbowRest` 12, `elbowSwing` 20, `headStab` 0.6.

Sources (URLs):
- [Orendurff et al. 2004](https://pubmed.ncbi.nlm.nih.gov/15685471/)
- [Donelan, Kram & Kuo 2001 (PDF)](https://spot.colorado.edu/~kram/DKKwidthPRSL2001.pdf)
- [Smith, Lelas & Kerrigan 2002](https://journals.sagepub.com/doi/abs/10.1089/15246090260137626)
- [van Emmerik & Wagenaar 1996](https://pubmed.ncbi.nlm.nih.gov/8872274/)
- [Troje 2002](https://jov.arvojournals.org/article.aspx?articleid=2192503)
- [Jokisch & Troje 2003](https://jov.arvojournals.org/article.aspx?articleid=2121515)
- [Alexander 1976](https://www.nature.com/articles/261129a0)
- [Johnson & Tassinary 2005](https://doi.org/10.1111/j.1467-9280.2005.01633.x)
- [Bruijn et al. 2008](https://www.sciencedirect.com/science/article/abs/pii/S096663620700135X)

Figures from Winter, Perry, Murray, Carter, ANSUR and Keogh are cited from the standard texts and were not re-fetched. Treat values marked "approx." or "≈" as ±20%.
