# Maha rig: open-source libraries and data survey

Checked on 2026-09-23. Licenses and activity come from the GitHub API (`gh api repos/...`), crates.io, PyPI, Zenodo, and the projects' own license pages. Anything I could not confirm is marked **UNVERIFIED**.

Current rig, for reference: `examples/3_maha_rig.rs` has 22 body joints (pelvis, spine, chest, neck, head, crown, shoulder/elbow/wrist/fist, hip/knee/ankle/toe) plus 12 wing joints. The body is SDF round cones, meshed with `fast-surface-nets`, with hand-built skin weights, trap and delt morphs, and JSON export. `examples/maha_skin.html` is a three.js `SkinnedMesh` with a procedural walk, analytic 2-bone leg IK, and about 50 knobs.

License flags used below:
- **OK**: permissive, or CC0 / CC-BY.
- **COPYLEFT**: GPL or AGPL on the code only.
- **NC**: non-commercial only.
- **NC-ND**: non-commercial and no derivatives. Retargeting counts as a derivative.

---

## 1. Parametric human body models

| Name | What it gives us | License | Lang | Activity | How it plugs in | Effort | Verdict |
|---|---|---|---|---|---|---|---|
| **MakeHuman assets via MPFB2** (makehumancommunity/mpfb2) | Base mesh `base.obj` (19,158 verts, 125 `joint-*` helper groups). 1,259 sparse `.target.gz` morphs. Macro targets cover gender, age, muscle, weight, height and proportions (for example `macrodetails/height/male-young-maxmuscle-*`). Per-limb muscle targets exist: `l-upperarm-muscle-incr`, `l-upperarm-shoulder-muscle-incr`, lower arm, lower leg. Rigs with CC0 weight JSON: `game_engine` (53 bones), `default`, `cmu_mb`, `mixamo`, `rigify`, `openpose`. | **OK**: assets CC0 1.0. Addon code GPLv3. The LICENSE says exports and output are yours, with no claim. | Python (Blender 4.2+ addon). Data is plain text. | Very active. v2.0.17 released 2026-07-22, pushed 2026-09-23. | **Path A**: Blender + MPFB2 → set muscle/weight → add `game_engine` or `cmu_mb` rig → Blender glTF export → Rust `gltf` crate. **Path B**: no Blender. Parse `base.obj` + sum the targets + `weights.*.json` + `rig.*.json` directly in Rust. Targets are lines of `vidx dx dy dz`. Bone heads and tails are centroids of `joint-*` helper vertex groups ("CUBE" strategy). | A: S. B: M. | **Best body source.** Muscular young male with CC0 skeleton and weights. |
| **Anny** (naver/anny) | Differentiable MakeHuman-derived model. Phenotypes, verified in `models/phenotype.py`: `gender, age, muscle, weight, height, proportions`, all in [0,1]. 104-bone compact rig or 163-bone MakeHuman rig. Linear blend skinning (LBS). Triangulated topology. | **OK**: code Apache-2.0, data CC0 (from MPFB2). The optional `smplx` topology is NC. | Python / PyTorch | Active. v0.6.0 on PyPI 2026-08-07, pushed 2026-09-17. | `pip install anny`. Set `muscle=1, weight≈0.6, gender=1, height≈0.7`, then dump vertices, faces, bone rest transforms and skin weights to JSON, then load with `serde_json` in Rust. README shows mesh export through trimesh. The weight and bone attribute names are **UNVERIFIED**. | S–M | **Best scripted bake.** It handles MakeHuman's macro interpolation for us. |
| MakeHuman 1.x (makehumancommunity/makehuman) | Same assets as MPFB2, as a standalone app. Exports FBX, OBJ, DAE, MHX2. | Code AGPL. Assets CC0. | Python / Qt | Last push 2024-08-19 | Export OBJ/FBX, then convert to glTF | S | Superseded by MPFB2 and Anny. |
| MB-Lab (animate1978/MB-Lab) | Blender character generator with muscle/mass sliders | Code GPL3. **Data (meshes) AGPL3.** | Python | **Archived.** Last push 2024-07-21. | Blender → glTF | S | Skip. Archived, and AGPL on the mesh data is murky. |
| SMPL / SMPL-X (vchoutas/smplx) | Industry-standard shape PCA plus 24/55 joints and LBS | **NC.** "non-commercial scientific research, education, or artistic projects". Commercial license via Meshcapade. Note: a fixed "SMPL-Body" (mesh + rig + pose blendshapes, no shape space) is **CC-BY 4.0** per smpl.is.tue.mpg.de/bodylicense.html. | Python | smplx pushed 2024-08-12 | Generate a body with the NC model, then export a fixed SMPL-Body under CC-BY. Legally subtle. | M | Avoid. The PCA shape space is not built for bodybuilders anyway. |
| STAR (ahmedosman/STAR) | Sparse-weight SMPL successor | **NC.** Same MPI license. | Python | Last push 2024-02-02 | same as SMPL | M | Avoid. |
| `smpl-rs` / `gloss` (Meshcapade) | Rust SMPL-family runtime and viewer | Code MIT. **Model files NC** (separate download). | Rust | smpl-rs 0.9.0, released 2026-02-19 | Rust-native LBS, but needs the NC model files | M | Useful only as reference Rust LBS code. |
| SOMA-X (NVlabs/SOMA-X) | One skeleton over several identity backends (MHR, SOMA, Anny, SMPL). Rig controls. USD/NPZ I/O. | **OK**: Apache-2.0. SMPL/MANO backends need user-supplied NC files. | Python (Warp) | Active. Pushed 2026-09-18. | Anny backend → NPZ → Rust | M | Overkill. Use Anny directly. |
| `symbios-avatar` (TheJanusStream) | **Rust crate, glam.** Procedural humanoid from a record: mass, body fat, age axes; capsule graph → B-Mesh → Catmull-Clark. Rigged and skinned. Has `anim::ik::two_bone`, `fabrik`, `plant_feet` (pelvis drop), `Inertializer`, and `anim::gait` driven by phase offset + duty factor. | **OK**: MIT | Rust | 0.10.0 released 2026-09-22. **1 star, single author, "Early"**. No GLB writer yet. | Direct crate dependency. It uses glam 0.32 and we use 0.33, so convert via arrays. | M | **Watch or borrow ideas.** Its gait and `plant_feet` design matches our needs. Too young to depend on. |
| Quaternius Universal Base Characters | 26 rigged CC0 humans (glTF/FBX/OBJ), about 13k tris, on the same rig as the Animation Library | **OK**: CC0 (quaternius.com pack page) | asset | **UNVERIFIED** release date | glTF → `gltf` crate | S | Good test mesh. Stylized, not muscular. |

## 2. Walking motion data

| Name | What it gives us | License | Format | Activity | How it plugs in | Effort | Verdict |
|---|---|---|---|---|---|---|---|
| **100STYLE** (Zenodo 8127870; ianxmason.com/100style) | 100 locomotion styles, each with FW/BW/SW/FR/BR/SR/ID/TR clips. Useful styles: **`Heavyset`**, `Proud`, `BigSteps`, `Neutral`, `Old`, `Stiff`, `SwingShoulders`, `WideLegs`. The skeleton is Hips / Chest / Neck / Head / L+R Collar, Shoulder, Elbow, Wrist, Hip, Knee, Ankle, Toe, which is **nearly 1:1 with our joint table**. | **OK**: CC-BY 4.0 (Zenodo metadata) | BVH (Xsens) | Static dataset (2022-05-04). The `orangeduck/100style-retarget` re-export is CC-BY-4.0, updated 2025-09-16. Its Geno mesh is NC and not needed. | `Heavyset_FW.bvh` → cut one gait cycle → map joints → bake local quaternions per frame into a Rust `const` table (or JSON) → slow down for scale | S–M | **Best walk data.** A "heavyset" forward walk under CC-BY, on a skeleton close to ours. |
| **CMU Graphics Lab Mocap** (mocap.cs.cmu.edu) | Relevant clips: **17_08 and 17_09 "muscular, heavyset person's walk"**, **137_42 "Strong Man Walk"**, 07_04/07_05/08_04/37_01 "slow walk", 08_11 "slow walk/stride", 132_45–50 "Walk Slow", 137_33 "Old Man Walk". | **OK**: "free for all uses", "may be copied, modified, or redistributed without permission" (FAQ) | ASF/AMC native. BVH conversions by Bruce Hahne are hosted at cgspeed (**UNVERIFIED** mirror availability). | Static | Same bake as 100STYLE. The CMU skeleton has extra spine and neck joints to collapse. MPFB2 ships a matching `cmu_mb` rig. | S–M | **Co-best.** The strongest license, and the one clip labelled "muscular heavyset". |
| **Quaternius Universal Animation Library** (free standard pack; mirror J-Ponzo/gltf-universal-animation-library) | **Verified by parsing the glTF:** 46 clips including `Walk_Loop`, `Walk_Formal_Loop`, `Idle_Loop`, plus 1 skin on a Rigify `DEF-*` skeleton (`DEF-hips`, `DEF-spine.001–003`, `DEF-thigh/shin/foot/toe.L/R`, …) with a `Mannequin` mesh. | **OK**: CC0-1.0 (repo LICENSE) | glTF | Mirror snapshot 2025-06-10 | Directly loadable with the `gltf` crate. It is an end-to-end test asset for a Rust skinned-glTF sampler. | S | **Use as the pipeline test fixture.** The walk is generic game-style, not heavy. |
| Mixamo (Adobe) | Many walk variants, including heavy and "zombie" walks, plus auto-rig | Proprietary, free. Per the Mixamo FAQ: royalty-free in commercial projects, but **no redistribution of raw files** as assets or templates. The FAQ page returned 403, so this is summarized from search results: **partly UNVERIFIED**. | FBX | Adobe service | Download FBX → Blender → glTF. Baking into a committed Rust table may count as redistributing raw data. | S | Authoring reference only. Do not commit the data. |
| Bandai Namco Research Motion Dataset | Walks in 15 + 7 styles (active, tired, happy, …) from pro actors | **NC**: CC BY-NC 4.0 (both datasets) | BVH | Last push 2023-07-04 | BVH bake | S | NC. Reference viewing only. |
| LAFAN1 (ubisoft-laforge-animation-dataset) | Long walk and locomotion takes. Used by motion matching. | **NC-ND**: CC BY-NC-ND 4.0 (LICENSE verified). Retargeted curves are a derivative, so they are forbidden. | BVH | Last push 2022-09-10 | none legal for us | – | **Reject.** |
| AMASS | Unified SMPL mocap archive, which includes CMU | **NC** (license page: "non-commercial … artistic projects" only) | SMPL params | – | Use raw CMU instead | – | Reject. Go to CMU directly. |
| PFNN data and demo | Terrain-aware locomotion NN, weights, and data | **NC**: "free for academic and non-commercial purposes" (sreyafrancis/PFNN mirror; original is by Holden/orangeduck) | C++ / Python | Mirror last push 2021-11-19 | Would need a NN runtime in Rust | L | Reject. Overkill for one calm walk. |
| AI4Animation (sebastianstarke) | DeepPhase, local phases, and related code and data | **NC**: "only for research or education purposes"; mocap under CC BY-NC 4.0 | Unity C# / Python | Pushed 2026-04-17 | – | L | Reject. |
| Motion-Matching (orangeduck) | Clean motion matching and inertialization reference implementation | Code **OK** (MIT). Its data is LAFAN1 (**NC-ND**). | C++ / raylib | Pushed 2025-02-06 | Port the *inertialization* snippet only, if we ever blend clips | S (snippet) | Borrow ideas only. |

## 3. Procedural gait and IK

| Name | What it gives us | License | Lang | Activity | Fit | Verdict |
|---|---|---|---|---|---|---|
| Our analytic 2-bone IK (maha_skin.html) | Law-of-cosines knee, pelvis frame | – | JS | – | Already correct for legs. Port it to Rust with glam (about 30 lines). | **Keep.** |
| `k` (openrr/k) | Robotics kinematic chains: Jacobian IK, URDF | **OK**: Apache-2.0 | Rust (nalgebra) | 0.32.0 released 2024-09-10, repo pushed 2026-06-19 | Brings in nalgebra and a robot-arm API alongside glam | Reject. Wrong abstraction. |
| `bevy_mod_inverse_kinematics` (Kurble) | IK with pole targets on Bevy entities | **OK**: MIT OR Apache-2.0 | Rust | 0.12.0 (Bevy 0.19) released 2026-09-01 | Tied to Bevy's ECS | Reject for the renderer. |
| `fabrik` crate | FABRIK | OK (MIT) | Rust | 0.1.0, 2021 | Stale, trivial | Reject. Write FABRIK inline if the spine or wings ever need it. |
| `transform-gizmo` | Interactive 3D gizmo widget | OK (MIT OR Apache-2.0) | Rust | 0.11.0 released 2026-08-17 | An editor UI, not IK | Not relevant. |
| `symbios-avatar` `anim::*` | two_bone, fabrik, plant_feet, gait (phase + duty), Inertializer | OK (MIT) | Rust | See section 1 | Closest Rust analogue to our walk | **Read its source** for the pelvis-drop and duty-factor ideas. |
| three.js `CCDIKSolver` | CCD for SkinnedMesh (`examples/jsm/animation/CCDIKSolver.js`) | OK (MIT) | JS | three r186 | Authoring aid in maha_skin.html | Optional. The 2-bone analytic solver is better for legs. |
| THREE.IK (jsantell) | FABRIK for three.js | OK (MIT) | JS | Last push 2023-03-15 | – | Stale. Skip. |
| fullik (lo-th) | FABRIK 2D/3D for three.js | OK (MIT) | JS | Pushed 2025-05-17 | – | Skip. Not needed. |
| Godot 4.6 IK | `SkeletonIK3D` **deprecated in 4.6**, replaced by `TwoBoneIK3D`, `FABRIK3D`, `CCDIK3D`, `JacobianIK3D`, `SplineIK3D` (SkeletonModifier3D) | OK (MIT) | C++ | Very active | Reference implementations only | Reference. |
| Unity Animation Rigging | Two-bone IK, multi-aim, and similar constraints | **Unity Companion License**: only usable with a Unity engine license. Not open source. | C# | – | – | Reject. |
| Blender Rigify | Auto-rig with IK/FK. The Quaternius UAL is on its `DEF-*` bones. | COPYLEFT (GPL, bundled with Blender). Output is unencumbered. | Python | Ships with Blender | Authoring only (MPFB2 → Rigify → bake → glTF) | Authoring aid. |

## 4. Rust glTF, skinning and playback

| Name | What it gives us | License | Activity | CPU sample into our grid? | Verdict |
|---|---|---|---|---|---|
| **`gltf`** (gltf-rs) | Reads meshes, `Skin` (joints + inverse bind matrices), `JOINTS_0`/`WEIGHTS_0`, and `Animation` channels (TRS samplers). Readers only; no evaluation. | **OK**: MIT OR Apache-2.0 | 1.4.1 released 2024-05-10. Repo pushed 2026-05-11. About 9.7M downloads. | Yes. We write the pieces ourselves: sample (lerp/slerp keyframes), FK, then `skinned = Σ wᵢ · (Mᵢ · IBMᵢ) · v` with glam. About 200 lines. Output is posed triangles to rasterize into the ASCII z-buffer, or joint transforms to drive our existing SDF. | **Use.** A small dependency that fits our glam/CPU style. |
| `easy-gltf` | Simplified glTF scene loading | OK (MIT) | 1.1.5, 2025-03-29 | Static meshes only (no skin or animation, **UNVERIFIED** in detail) | Skip. |
| `ozz-animation-rs` | Deterministic sampling, blending and local-to-model runtime (a port of ozz) | **MPL-2.0** (file-level copyleft, fine as a dependency) | 0.11.0 released 2025-10-14, pushed 2026-09-05 | Yes, but assets must be built with the **C++ ozz offline tools** (no offline support "and no plans") | Skip unless we need blending of many clips. |
| `bevy_animation` / Bevy glTF | Full skinned glTF plus an animation graph | OK (MIT OR Apache-2.0) | Bevy v0.19.1 released 2026-08-13, 0.20-rc | Only by running a Bevy App and reading `GlobalTransform`s. GPU skinning. | Reject. Pulls in an engine for 200 lines of math. |
| `three-d` | wgpu/GL renderer, glTF loading | OK (MIT) | 0.19.0, 2026-04-17 | **No skinning**: issue #307 "Skinning" is open | Reject. |
| `rend3` | Renderer | OK | **Archived**. Last release 0.3.0 in 2022. | – | Reject. |
| `fyrox` | Full engine: skinned meshes, glTF loader (`fyrox-impl/src/resource/gltf`), animation, retargeting | OK (MIT) | v1.0.0 released 2026-03-29 | Engine-bound | Reject. |
| `bvh_anim` / `bvh_anim_parser` | BVH parsers | OK (MIT / MIT) | bvh_anim 0.4.0 (2019, repo pushed 2025-07). bvh_anim_parser 1.0.1 (2024-05). | Used offline in a bake example | Use `bvh_anim_parser` in a dev-only bake example, or parse in Python. Runtime never sees BVH. |
| `smpl-rs` | Rust LBS reference | Code MIT | 0.9.0, 2026-02 | – | Reference only. |

---

## Recommendation (ranked)

### 1. Bake a CC0 MakeHuman muscular male into Rust. Replace the SDF body.
Why: CC0 mesh, skeleton, and weights, plus real muscle, weight, and proportion targets, including the upper-arm/shoulder muscle targets that cover our hand-made delt and trap morphs. The raw data is plain text, so we can bake it without Blender.

Next steps:
1. Run Anny with `gender=1, age≈young, muscle=1.0, weight≈0.6–0.7, proportions=1, height≈0.8`. Use `rig="anny"` (104 bones) or the MakeHuman `game_engine` rig. Check it visually in the three.js viewer.
2. Dump the mesh and rig to `docs/maha_rig/*.json`: vertices, triangles, rest joints (head, tail, roll), skin weights (top 4), plus optional extra muscle targets (`*-upperarm-shoulder-muscle-incr`, `*-upperarm-muscle-incr`, `*-lowerleg-muscle-incr`) as morph deltas. Alternative: MPFB2 in Blender, then File → Export glTF.
3. In Rust, collapse bones onto our joint names. The `game_engine` rig's non-finger bones, verified from `rig.game_engine.json`, are: Root, pelvis, spine_01–03, neck_01, head, clavicle, upperarm, lowerarm, hand, thigh, calf, foot, ball (each _l/_r). Mapping: `pelvis←pelvis`, `spine←spine_01+02`, `chest←spine_03`, `neck←neck_01`, `head←head`, with the clavicle folded into chest or trap, `r_shoulder←upperarm_r`, `r_elbow←lowerarm_r`, `r_wrist←hand_r`, `r_hip←thigh_r`, `r_knee←calf_r`, `r_ankle←foot_r`, `r_toe←ball_r`. Merge finger and twist weights into their parents. Decimate to about 5–10k tris.
4. Replace `shapes()`/`scene_sdf()` meshing with CPU LBS plus a triangle z-buffer rasterizer at grid resolution, using normals for character and shade.

| Keep | Throw away |
|---|---|
| Joint table names and hierarchy (the retarget target), FK `pose()`, `Camera`, `Canvas`, slope-char and wire drawing, JSON export shape, blend-shape plumbing, wing joints and their SDF (the wings are not human, so keep them as SDF or cones blended on top), the three.js viewer and knobs | SDF round-cone body, `fast-surface-nets` meshing of the body, hand-authored skin weights, hand-sculpted trap and delt deltas (MakeHuman targets replace them) |

### 2. Retarget a real heavy walk: 100STYLE `Heavyset_FW` (CC-BY) or CMU 17_08/17_09 (free).
Why: this gives reference curves from a real heavy human instead of about 50 hand-tuned knobs. The 100STYLE skeleton (Hips/Chest/Collar/Shoulder/Elbow/Wrist/Hip/Knee/Ankle/Toe) maps almost 1:1 to ours. CMU explicitly permits modification and redistribution.

Next steps:
1. Download `100STYLE.zip` from Zenodo. Pull `Heavyset_FW`, `Proud_FW`, `BigSteps_FW`, `Neutral_FW`. Also get CMU `17_08`, `17_09`, `137_42` (BVH from cgspeed, or ASF/AMC).
2. Use a dev-only bake example (`bvh_anim_parser`, or a Python script) to find one clean gait cycle from heel strike to heel strike, express rotations relative to T-pose/bind, and retarget to our joint local frames. Write a Rust `const` table of per-joint quaternions over N phases, plus root bob and sway. Add a CC-BY credit line.
3. Two uses. (a) Direct playback: slerp by phase and time-stretch for giant scale. At 3–4× height, cadence should drop by about √scale. (b) Fit our procedural knobs (hip sway, bob, arm swing, counter-lean) to the curves and keep procedural generation.
4. Keep 2-bone IK as a post-pass for foot planting, because retargeted feet slide once proportions change.

| Keep | Throw away |
|---|---|
| Procedural walk structure, 2-bone IK, seeded jitter and variation, knob UI | The hand-authored `KEYS` K0–K4 keyframes in `3_maha_rig.rs`, and guesswork defaults for sway, bob, and swing |

### 3. Build a generic Rust skinned-glTF sampler using the `gltf` crate. Test it with the Quaternius UAL (CC0).
Why: once this exists, any source (the MPFB2 export, UAL `Walk_Loop`/`Walk_Formal_Loop`, a Blender-retargeted CMU walk) is just a file. The UAL glTF is a verified CC0 fixture with skin, mesh, and walk in one file, which makes a good snapshot-test input.

Next steps:
1. Add `gltf = "1.4"`. Write a loader for skin joints, inverse bind matrices, JOINTS_0/WEIGHTS_0, and animation samplers, then keyframe interpolation, FK, and LBS in glam.
2. Add an `insta` snapshot of `Walk_Loop` at fixed phases, rendered to an 80×24 grid.
3. Point it at the MakeHuman glTF from path 1, carrying the path 2 walk (retargeted in Blender, or applied from the Rust table).

| Keep | Throw away |
|---|---|
| Everything from paths 1 and 2. The sampler becomes the common loader. | Custom JSON rig export, once glTF is the interchange format (optional) |

### Not recommended
- SMPL, SMPL-X, STAR, AMASS, Bandai Namco, PFNN, AI4Animation: **NC**.
- LAFAN1: **NC-ND**, so retargeting is forbidden.
- Unity Animation Rigging: Unity-only license.
- Mixamo: fine for authoring, but do not commit the raw data.
- Bevy, Fyrox, three-d, rend3: an engine or no skinning, for what is about 200 lines of CPU math.
- `k`: robotics and nalgebra, wrong fit.
- MB-Lab: archived, and its data is AGPL.
