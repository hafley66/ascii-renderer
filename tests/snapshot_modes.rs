use std::process::Command;

/// Run the renderer with given args and return output with ANSI codes stripped.
fn render(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_ascii-renderer"))
        .args(args)
        .env("ASCII_GRID_W", "80")
        .env("ASCII_GRID_H", "24")
        .output()
        .expect("failed to run ascii-renderer");
    let raw = String::from_utf8_lossy(&output.stdout);
    strip_ansi(&raw)
}

/// Strip ANSI escape sequences, keeping only visible characters.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip until we hit a letter (end of escape sequence)
            while let Some(&next) = chars.peek() {
                chars.next();
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

// ── Mode snapshots ───────────────────────────────────────────────

#[test]
fn forest_seed_42() {
    insta::assert_snapshot!(render(&["42", "forest", "ember"]));
}

#[test]
fn forest2_seed_42() {
    insta::assert_snapshot!(render(&["42", "forest2", "ember"]));
}

#[test]
fn forest3_seed_42() {
    insta::assert_snapshot!(render(&["42", "forest3", "ember"]));
}

#[test]
fn forest3_seed_77() {
    insta::assert_snapshot!(render(&["77", "forest3", "ocean"]));
}

#[test]
fn forest4_seed_42() {
    insta::assert_snapshot!(render(&["42", "forest4", "ember"]));
}

#[test]
fn forest4_seed_77() {
    insta::assert_snapshot!(render(&["77", "forest4", "ocean"]));
}

#[test]
fn forest2_seed_77() {
    insta::assert_snapshot!(render(&["77", "forest2", "forest"]));
}

#[test]
fn forest5_seed_42() {
    insta::assert_snapshot!(render(&["42", "forest5", "moss"]));
}

#[test]
fn mondrian_seed_42() {
    insta::assert_snapshot!(render(&["42", "mondrian", "ember"]));
}

#[test]
fn mondrian2_seed_42() {
    insta::assert_snapshot!(render(&["42", "mondrian2", "ember"]));
}

#[test]
fn mondrian2_seed_77() {
    insta::assert_snapshot!(render(&["77", "mondrian2", "neon"]));
}

#[test]
fn party_seed_42() {
    insta::assert_snapshot!(render(&[
        "42", "party", "ember", "0", "6", "50", "50", "none", "line", "0"
    ]));
}

#[test]
fn party_seed_99() {
    insta::assert_snapshot!(render(&[
        "99", "party", "midnight", "0", "5", "50", "50", "stars", "vine", "30"
    ]));
}

#[test]
fn soup_seed_42() {
    insta::assert_snapshot!(render(&["42", "soup", "ember"]));
}

#[test]
fn tree_seed_42() {
    insta::assert_snapshot!(render(&["42", "tree", "ember"]));
}

#[test]
fn trees_seed_42() {
    insta::assert_snapshot!(render(&["42", "trees", "ember"]));
}

#[test]
fn aztec_seed_42() {
    insta::assert_snapshot!(render(&["42", "aztec", "ember"]));
}

#[test]
fn flowers_seed_42() {
    insta::assert_snapshot!(render(&["42", "flowers", "ember"]));
}

#[test]
fn fruits_seed_42() {
    insta::assert_snapshot!(render(&["42", "fruits", "ember"]));
}

#[test]
fn masks_seed_42() {
    insta::assert_snapshot!(render(&["42", "masks", "ember"]));
}

#[test]
fn tiles_seed_42() {
    insta::assert_snapshot!(render(&["42", "tiles", "ember"]));
}

#[test]
fn bsp_seed_42() {
    insta::assert_snapshot!(render(&["42", "bsp", "ember"]));
}

#[test]
fn ca_seed_42() {
    insta::assert_snapshot!(render(&["42", "ca", "ember"]));
}

#[test]
fn noise_seed_42() {
    insta::assert_snapshot!(render(&["42", "noise", "ember"]));
}

#[test]
fn shapes_seed_42() {
    insta::assert_snapshot!(render(&["42", "shapes", "ember"]));
}

#[test]
fn world_seed_42() {
    insta::assert_snapshot!(render(&["42", "world", "ember"]));
}

#[test]
fn boles1_seed_42() {
    insta::assert_snapshot!(render(&["42", "boles1", "ember"]));
}

#[test]
fn boles2_seed_42() {
    insta::assert_snapshot!(render(&["42", "boles2", "ember"]));
}

#[test]
fn boles3_seed_42() {
    insta::assert_snapshot!(render(&["42", "boles3", "ember"]));
}

#[test]
fn trunks1_seed_42() {
    insta::assert_snapshot!(render(&["42", "trunks1", "ember"]));
}

#[test]
fn trees1_seed_42() {
    insta::assert_snapshot!(render(&["42", "trees1", "ember"]));
}

#[test]
fn trees2_seed_42() {
    insta::assert_snapshot!(render(&["42", "trees2", "ember"]));
}

#[test]
fn forest6_seed_42() {
    insta::assert_snapshot!(render(&["42", "forest6", "ember"]));
}

#[test]
fn trees4_seed_42() {
    insta::assert_snapshot!(render(&["42", "trees4", "ember"]));
}

#[test]
fn bushes_seed_42() {
    insta::assert_snapshot!(render(&["42", "bushes", "ember"]));
}

#[test]
fn trees8_seed_42() {
    insta::assert_snapshot!(render(&["42", "trees8", "moss"]));
}

#[test]
fn trees9_seed_42() {
    insta::assert_snapshot!(render(&["42", "trees9", "moss"]));
}

#[test]
fn boles4_seed_42() {
    insta::assert_snapshot!(render(&["42", "boles4", "ember"]));
}

#[test]
fn boles5_seed_42() {
    insta::assert_snapshot!(render(&["42", "boles5", "ember"]));
}

#[test]
fn forest8_seed_42() {
    insta::assert_snapshot!(render(&["42", "forest8", "moss"]));
}

#[test]
fn forest9_seed_42() {
    insta::assert_snapshot!(render(&["42", "forest9", "deep"]));
}

#[test]
fn boles6_seed_42() {
    insta::assert_snapshot!(render(&["42", "boles6", "ember"]));
}

#[test]
fn trees10_seed_42() {
    insta::assert_snapshot!(render(&["42", "trees10"]));
}

#[test]
fn fullmetal_eyes2_seed_42() {
    insta::assert_snapshot!(render(&["42", "fullmetal-eyes2", "nerv"]));
}

#[test]
fn kintsugi_seed_42() {
    insta::assert_snapshot!(render(&["42", "kintsugi", "ember"]));
}

#[test]
fn constellation_seed_42() {
    insta::assert_snapshot!(render(&["42", "constellation", "deep"]));
}

#[test]
fn strata_seed_42() {
    insta::assert_snapshot!(render(&["42", "strata", "terracotta"]));
}

#[test]
fn circuit_seed_42() {
    insta::assert_snapshot!(render(&["42", "circuit", "neon"]));
}

#[test]
fn quilt_seed_42() {
    insta::assert_snapshot!(render(&["42", "quilt", "ember"]));
}

#[test]
fn patchwalk_seed_42() {
    insta::assert_snapshot!(render(&["42", "patchwalk", "ember"]));
}

#[test]
fn aurora_seed_42() {
    insta::assert_snapshot!(render(&["42", "aurora", "deep"]));
}

#[test]
fn aura2_seed_42() {
    insta::assert_snapshot!(render(&["42", "aura2", "deep"]));
}

#[test]
fn harbor_seed_42() {
    insta::assert_snapshot!(render(&["42", "harbor", "arctic"]));
}

#[test]
fn labyrinth_seed_42() {
    insta::assert_snapshot!(render(&["42", "labyrinth", "mitla"]));
}

#[test]
fn eyes_seed_42() {
    insta::assert_snapshot!(render(&["42", "eyes", "neon"]));
}

#[test]
fn eyes2_seed_42() {
    insta::assert_snapshot!(render(&["42", "eyes2", "bone"]));
}

#[test]
fn fullmetal_eyes_seed_42() {
    insta::assert_snapshot!(render(&["42", "fullmetal-eyes", "nerv"]));
}

#[test]
fn fullmetal_alchemist_seed_42() {
    insta::assert_snapshot!(render(&["42", "fullmetal-alchemist", "ember"]));
}

#[test]
fn fullmetal_alchemist2_seed_42() {
    insta::assert_snapshot!(render(&["42", "fullmetal-alchemist2", "neon"]));
}

#[test]
fn fa3_seed_42() {
    insta::assert_snapshot!(render(&["42", "fa3", "neon"]));
}

#[test]
fn fa4_seed_42() {
    insta::assert_snapshot!(render(&["42", "fa4", "neon"]));
}

#[test]
fn fa5_seed_42() {
    insta::assert_snapshot!(render(&["42", "fa5", "neon"]));
}

#[test]
fn fa6_seed_42() {
    insta::assert_snapshot!(render(&[
        "42", "fa6", "nerv", "8", "55", "0.8", "42"
    ]));
}

#[test]
fn spiro_seed_42() {
    insta::assert_snapshot!(render(&["42", "spiro", "deep"]));
}

#[test]
fn weave_seed_42() {
    insta::assert_snapshot!(render(&["42", "weave", "ember"]));
}

#[test]
fn gears_seed_42() {
    insta::assert_snapshot!(render(&["42", "gears", "silver"]));
}

#[test]
fn kaleido_seed_42() {
    insta::assert_snapshot!(render(&["42", "kaleido", "sakura"]));
}

#[test]
fn contour_seed_42() {
    insta::assert_snapshot!(render(&["42", "contour", "moss"]));
}

#[test]
fn spiro_tile_seed_42() {
    insta::assert_snapshot!(render(&["42", "spiro-tile", "deep"]));
}

#[test]
fn eyes3_seed_42() {
    insta::assert_snapshot!(render(&["42", "eyes3", "ember"]));
}

#[test]
fn rainfall_seed_42() {
    insta::assert_snapshot!(render(&["42", "rainfall", "silver"]));
}

#[test]
fn meadow_seed_42() {
    insta::assert_snapshot!(render(&["42", "meadow", "moss"]));
}

#[test]
fn watershed_seed_42() {
    insta::assert_snapshot!(render(&["42", "watershed", "moss"]));
}

#[test]
fn solar_system_seed_42() {
    insta::assert_snapshot!(render(&["42", "solar-system", "deep"]));
}

#[test]
fn world2_seed_42() {
    insta::assert_snapshot!(render(&["42", "world2", "ember"]));
}

#[test]
fn metro_seed_42() {
    insta::assert_snapshot!(render(&["42", "metro", "neon"]));
}

#[test]
fn koi_seed_42() {
    insta::assert_snapshot!(render(&["42", "koi", "terracotta"]));
}

#[test]
fn skyline_seed_42() {
    insta::assert_snapshot!(render(&["42", "skyline", "deep"]));
}

#[test]
fn hive_seed_42() {
    insta::assert_snapshot!(render(&["42", "hive", "ember"]));
}

#[test]
fn jelly_seed_42() {
    insta::assert_snapshot!(render(&["42", "jelly", "deep"]));
}

#[test]
fn jelly2_seed_42() {
    insta::assert_snapshot!(render(&["42", "jelly2", "deep"]));
}

#[test]
fn rhizome_seed_42() {
    insta::assert_snapshot!(render(&["42", "rhizome", "ember"]));
}

#[test]
fn effigy_seed_42() {
    insta::assert_snapshot!(render(&["42", "effigy", "ember"]));
}

#[test]
fn dendrite_seed_42() {
    insta::assert_snapshot!(render(&["42", "dendrite", "ember"]));
}

#[test]
fn totem_seed_42() {
    insta::assert_snapshot!(render(&["42", "totem", "ember"]));
}

#[test]
fn chimera_seed_42() {
    insta::assert_snapshot!(render(&["42", "chimera", "ember"]));
}

#[test]
fn hypercube_seed_42() {
    insta::assert_snapshot!(render(&["42", "hypercube", "neon", "3", "1", "2"]));
}

#[test]
fn flux_seed_42() {
    insta::assert_snapshot!(render(&["42", "flux", "arctic", "58", "8", "1"]));
}

#[test]
fn fireworks_seed_42() {
    insta::assert_snapshot!(render(&[
        "42",
        "fireworks",
        "ember",
        "6",
        "22",
        "1",
    ]));
}

#[test]
fn murmuration_seed_42() {
    insta::assert_snapshot!(render(&["42", "murmuration", "ember", "140", "3", "1"]));
}

#[test]
fn lanterns_seed_42() {
    insta::assert_snapshot!(render(&["42", "lanterns", "deep", "7", "1", "1"]));
}

#[test]
fn tide_seed_42() {
    insta::assert_snapshot!(render(&["42", "tide", "arctic", "2", "1", "1"]));
}

#[test]
fn fireflies_seed_42() {
    insta::assert_snapshot!(render(&["42", "fireflies", "ember", "14", "1", "1"]));
}

#[test]
fn ink_seed_42() {
    insta::assert_snapshot!(render(&["42", "ink", "deep", "5", "1", "1"]));
}

#[test]
fn meteors_seed_42() {
    insta::assert_snapshot!(render(&["42", "meteors", "arctic", "90", "1", "1"]));
}

#[test]
fn elevator_seed_42() {
    insta::assert_snapshot!(render(&["42", "elevator", "nerv", "3", "1", "1"]));
}

#[test]
fn ferris_seed_42() {
    insta::assert_snapshot!(render(&["42", "ferris", "deep", "8", "10", "1"]));
}

#[test]
fn arboretum_seed_42() {
    insta::assert_snapshot!(render(&["42", "arboretum", "moss"]));
}

#[test]
fn arboretum_dense_1_layer() {
    insta::assert_snapshot!(render(&["42", "arboretum", "ember", "1", "40"]));
}

#[test]
fn astrolabe_seed_42() {
    insta::assert_snapshot!(render(&["42", "astrolabe", "ember"]));
}

#[test]
fn sauron_seed_42() {
    insta::assert_snapshot!(render(&["42", "sauron", "ember"]));
}

#[test]
fn singularity_seed_42() {
    insta::assert_snapshot!(render(&["42", "singularity", "deep"]));
}

#[test]
fn thunderhead_seed_42() {
    insta::assert_snapshot!(render(&["42", "thunderhead", "arctic"]));
}

#[test]
fn mandelbox_seed_42() {
    insta::assert_snapshot!(render(&["42", "mandelbox", "neon"]));
}

#[test]
fn illuminarium_seed_42() {
    insta::assert_snapshot!(render(&[
        "42",
        "illuminarium",
        "deep",
        "12",
        "7",
        "0.72",
        "9",
        "0.65",
        "0.35",
        "7",
        "90",
        "4",
        "0.72",
    ]));
}

#[test]
fn qwen_cathedral_seed_42() {
    insta::assert_snapshot!(render(&[
        "42",
        "qwen-cathedral",
        "ember",
        "5",
        "2",
        "12",
        "14",
        "0.8",
        "0.6",
        "8",
        "3",
        "0.55",
        "0.7",
        "0.62",
        "4",
    ]));
}

#[test]
fn qwen_cathedral_seed_7_midnight() {
    insta::assert_snapshot!(render(&[
        "7",
        "qwen-cathedral",
        "deep",
        "7",
        "3",
        "16",
        "24",
        "1.2",
        "0.9",
        "12",
        "4",
        "0.8",
        "1.1",
        "0.85",
        "6",
    ]));
}

#[test]
fn gem_aetherium_seed_42() {
    insta::assert_snapshot!(render(&[
        "42",
        "gem-aetherium",
        "cathedral",
        "6",
        "7",
        "4",
        "12",
        "0.75",
        "0.45",
        "0.7",
        "0.8",
        "4",
        "0.85",
        "1.0",
        "3.0",
    ]));
}

#[test]
fn mahoraga2_seed_42() {
    insta::assert_snapshot!(render(&["42", "mahoraga-2", "ember"]));
}

#[test]
fn mahoraga2_fuga_close_focus() {
    insta::assert_snapshot!(render(&["42", "mahoraga-2", "deep", "8", "4", "2.5", "0.9"]));
}

#[test]
fn mahoraga3_seed_42() {
    insta::assert_snapshot!(render(&["42", "mahoraga-3", "ember"]));
}

#[test]
fn mahoraga4_seed_42() {
    insta::assert_snapshot!(render(&["42", "mahoraga-4", "ember"]));
}

#[test]
fn mahoraga4_guard_to_lunge() {
    insta::assert_snapshot!(render(&["42", "mahoraga-4", "deep", "8", "7", "1.5", "0.45", "2", "3", "0.6"]));
}

#[test]
fn mahoraga5_seed_42() {
    insta::assert_snapshot!(render(&["42", "mahoraga-5", "ember"]));
}

#[test]
fn mahoraga5_swing_vs_crouch() {
    insta::assert_snapshot!(render(&["42", "mahoraga-5", "deep", "8", "7", "1.5", "0.45", "5", "3", "0.4", "3"]));
}

#[test]
fn tree_of_life_seed_42() {
    insta::assert_snapshot!(render(&["42", "tree-of-life", "moss"]));
}

#[test]
fn tree_of_life_deep_wide_seam() {
    insta::assert_snapshot!(render(&["42", "tree-of-life", "ember", "10", "3", "1", "60", "0.4"]));
}

#[test]
fn tree_of_life_2_seed_42() {
    insta::assert_snapshot!(render(&["42", "tree-of-life-2", "moss"]));
}

#[test]
fn tree_of_life_2_autumn_veil() {
    insta::assert_snapshot!(render(&["42", "tree-of-life-2", "ember", "9", "3", "1", "60", "0.45", "9", "0.5"]));
}

#[test]
fn tree_of_life_3_seed_42() {
    insta::assert_snapshot!(render(&["42", "tree-of-life-3", "moss"]));
}

#[test]
fn tree_of_life_3_many_eyes() {
    insta::assert_snapshot!(render(&["42", "tree-of-life-3", "ember", "9", "2", "1", "40", "0.5", "4", "0.12", "0.3", "0.5", "1", "60"]));
}

#[test]
fn tree_of_life_4_seed_42() {
    insta::assert_snapshot!(render(&["42", "tree-of-life-4", "moss"]));
}

#[test]
fn tree_of_life_4_deep_drift() {
    insta::assert_snapshot!(render(&["42", "tree-of-life-4", "deep", "9", "0.6", "0.2", "1.4", "120"]));
}

#[test]
fn tree_of_life_5_seed_42() {
    insta::assert_snapshot!(render(&["42", "tree-of-life-5", "moss"]));
}

#[test]
fn tree_of_life_5_klein_spin() {
    insta::assert_snapshot!(render(&["42", "tree-of-life-5", "ember", "8", "0.7", "0.9", "0.15", "0.06", "80"]));
}

#[test]
fn tree_of_life_6_seed_42() {
    insta::assert_snapshot!(render(&["42", "tree-of-life-6", "moss"]));
}

#[test]
fn tree_of_life_6_zoom_flow() {
    insta::assert_snapshot!(render(&["42", "tree-of-life-6", "deep", "8", "0.8", "0.5", "0.8", "90"]));
}

#[test]
fn braid_seed_42() {
    insta::assert_snapshot!(render(&["42", "braid", "moss"]));
}

#[test]
fn braid_wide_ribbons() {
    insta::assert_snapshot!(render(&["42", "braid", "ember", "7", "4", "8", "9", "5", "0.7", "0.5", "0.1", "1.0", "0.9"]));
}

#[test]
fn braid_2_seed_42() {
    insta::assert_snapshot!(render(&["42", "braid-2", "moss"]));
}

#[test]
fn braid_2_thick_slow_twist() {
    insta::assert_snapshot!(render(&["42", "braid-2", "ember", "4", "6", "16", "5", "5", "0.6", "48", "12", "30", "9", "0.1", "0.9"]));
}

#[test]
fn chladni_seed_42() {
    insta::assert_snapshot!(render(&["42", "chladni", "moss"]));
}

#[test]
fn chladni_high_order_thin_sand() {
    insta::assert_snapshot!(render(&["42", "chladni", "ember", "4", "1", "11", "0.03", "0.5", "6", "2", "0", "2"]));
}

#[test]
fn pendulum_wave_seed_42() {
    insta::assert_snapshot!(render(&["42", "pendulum-wave", "moss"]));
}

#[test]
fn pendulum_wave_top_view_trails() {
    insta::assert_snapshot!(render(&["42", "pendulum-wave", "ember", "10", "20", "12", "0.6", "1", "20", "0.05", "2", "30"]));
}

#[test]
fn polytope_seed_42() {
    insta::assert_snapshot!(render(&["42", "polytope", "moss"]));
}

#[test]
fn polytope_600_cell_trails_t12() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_ascii-renderer"))
        .args(["7", "polytope", "ember", "5", "40", "3", "3", "0.5", "3"])
        .env("ASCII_GRID_W", "80")
        .env("ASCII_GRID_H", "24")
        .env("ASCII_T", "12")
        .output()
        .expect("failed to run ascii-renderer");
    insta::assert_snapshot!(strip_ansi(&String::from_utf8_lossy(&output.stdout)));
}

#[test]
fn poincare_seed_42() {
    insta::assert_snapshot!(render(&["42", "poincare", "moss"]));
}

#[test]
fn poincare_half_plane_54() {
    insta::assert_snapshot!(render(&["7", "poincare", "ember", "5", "4", "24", "0.05", "2", "0", "2"]));
}

#[test]
fn opus_1_quasicrystal_seed_42() {
    insta::assert_snapshot!(render(&["42", "opus-1-quasicrystal", "moss"]));
}

#[test]
fn opus_1_quasicrystal_octagonal_dense() {
    insta::assert_snapshot!(render(&["7", "opus-1-quasicrystal", "ember", "1", "40", "4", "7", "0.05", "3", "1", "8", "3", "0.1", "0.4", "0.2"]));
}

/// Same as `render` but with the animation clock set, for modes that move.
fn render_t(args: &[&str], t: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_ascii-renderer"))
        .args(args)
        .env("ASCII_GRID_W", "80")
        .env("ASCII_GRID_H", "24")
        .env("ASCII_T", t)
        .output()
        .expect("failed to run ascii-renderer");
    strip_ansi(&String::from_utf8_lossy(&output.stdout))
}

#[test]
fn opus_2_quasicrystal_seed_42() {
    insta::assert_snapshot!(render(&["42", "opus-2-quasicrystal", "moss"]));
}

#[test]
fn opus_2_quasicrystal_seven_fold_moving() {
    insta::assert_snapshot!(render_t(
        &["7", "opus-2-quasicrystal", "ember", "1", "90", "7", "6", "1.1", "0.6", "0.05", "0.6", "0.03", "1", "0.9", "0.45", "80", "1", "2", "0.35", "3"],
        "45"
    ));
}

#[test]
fn sonnet_1_spirograph_seed_42() {
    insta::assert_snapshot!(render(&["42", "sonnet-1-spirograph", "moss"]));
}

#[test]
fn sonnet_1_spirograph_seed_7_knobs() {
    insta::assert_snapshot!(render(&["7", "sonnet-1-spirograph", "ember", "1.6", "45", "1.2", "0.8", "1.8", "1", "0.3", "1", "0.85", "1", "2.3"]));
}

#[test]
fn sonnet_1_spirograph_seed_42_moving() {
    insta::assert_snapshot!(render_t(&["42", "sonnet-1-spirograph", "moss"], "14"));
}

#[test]
fn sonnet_2_clifford_seed_42() {
    insta::assert_snapshot!(render(&["42", "sonnet-2-clifford", "moss"]));
}

#[test]
fn sonnet_2_clifford_breathing_t20() {
    insta::assert_snapshot!(render_t(&["7", "sonnet-2-clifford", "deep"], "20"));
}

#[test]
fn haiku_1_torus_seed_42() {
    insta::assert_snapshot!(render(&["42", "haiku-1-torus", "moss"]));
}

#[test]
fn haiku_1_torus_rotating_t10() {
    insta::assert_snapshot!(render_t(&["42", "haiku-1-torus", "moss"], "10"));
}

#[test]
fn haiku_2_ripple_seed_42() {
    insta::assert_snapshot!(render(&["42", "haiku-2-ripple", "moss"]));
}

#[test]
fn haiku_2_ripple_animated_t8() {
    insta::assert_snapshot!(render_t(&["42", "haiku-2-ripple", "moss"], "8"));
}

#[test]
fn fable_1_trees_seed_42() {
    insta::assert_snapshot!(render(&["42", "fable-1-trees", "moss"]));
}

#[test]
fn fable_1_trees_seed_7_knobs() {
    insta::assert_snapshot!(render(&["7", "fable-1-trees", "ember", "0.8", "0.6", "1.2", "1.1", "1.0", "0.5"]));
}

#[test]
fn fable_1_forest_seed_42() {
    insta::assert_snapshot!(render(&["42", "fable-1-forest", "moss"]));
}

#[test]
fn fable_1_forest_seed_7_knobs() {
    insta::assert_snapshot!(render(&["7", "fable-1-forest", "deep", "1.4", "4", "2", "1", "40", "3", "0.8", "0.55", "1", "0.4"]));
}

#[test]
fn fable_1_forest_swaying_t12() {
    insta::assert_snapshot!(render_t(&["42", "fable-1-forest", "moss"], "12"));
}

#[test]
fn fable_2_trees_seed_42() {
    insta::assert_snapshot!(render(&["42", "fable-2-trees", "moss"]));
}

#[test]
fn fable_2_trees_seed_7_knobs() {
    insta::assert_snapshot!(render(&["7", "fable-2-trees", "ember", "1.1", "0.6", "1.4", "0.5", "1", "0.5", "1.6", "1"]));
}

#[test]
fn fable_2_forest_seed_42() {
    insta::assert_snapshot!(render(&["42", "fable-2-forest", "moss"]));
}

#[test]
fn fable_2_forest_seed_7_moving_t12() {
    insta::assert_snapshot!(render_t(&["7", "fable-2-forest", "ember", "1.4", "4", "1.0", "1", "0", "2", "0.6", "1", "1", "0.7", "0.3", "0.5"], "12"));
}

#[test]
fn opus_1_trees_seed_42() {
    insta::assert_snapshot!(render(&["42", "opus-1-trees", "moss"]));
}

#[test]
fn opus_1_forest_seed_42() {
    insta::assert_snapshot!(render(&["42", "opus-1-forest", "moss"]));
}

#[test]
fn opus_1_forest_drifting_t18() {
    insta::assert_snapshot!(render_t(&["7", "opus-1-forest", "moss"], "18"));
}

#[test]
fn opus_2_trees_seed_42() {
    insta::assert_snapshot!(render(&["42", "opus-2-trees", "moss"]));
}

#[test]
fn opus_2_forest_seed_42() {
    insta::assert_snapshot!(render(&["42", "opus-2-forest", "moss"]));
}

#[test]
fn opus_2_forest_animated_t14() {
    insta::assert_snapshot!(render_t(&["42", "opus-2-forest", "moss"], "14"));
}

#[test]
fn haiku_1_trees_seed_42() {
    insta::assert_snapshot!(render(&["42", "haiku-1-trees", "moss"]));
}

#[test]
fn haiku_1_forest_seed_42() {
    insta::assert_snapshot!(render(&["42", "haiku-1-forest", "moss"]));
}

#[test]
fn haiku_2_trees_seed_42() {
    insta::assert_snapshot!(render(&["42", "haiku-2-trees", "moss"]));
}

#[test]
fn haiku_2_forest_seed_42() {
    insta::assert_snapshot!(render(&["42", "haiku-2-forest", "moss"]));
}

#[test]
fn haiku_2_forest_animated_t12() {
    insta::assert_snapshot!(render_t(&["42", "haiku-2-forest", "moss"], "12"));
}

#[test]
fn sonnet_2_trees_seed_42() {
    insta::assert_snapshot!(render(&["42", "sonnet-2-trees", "moss"]));
}

#[test]
fn sonnet_2_trees_seed_7_knobs() {
    insta::assert_snapshot!(render(&["7", "sonnet-2-trees", "ember", "1.1", "0.6", "1.4", "0.5", "1", "0.5", "1.6", "1"]));
}

#[test]
fn sonnet_2_forest_seed_42() {
    insta::assert_snapshot!(render(&["42", "sonnet-2-forest", "moss"]));
}

#[test]
fn sonnet_2_forest_seed_7_moving_t18() {
    insta::assert_snapshot!(render_t(&["7", "sonnet-2-forest", "ember"], "18"));
}

#[test]
fn sonnet_1_trees_seed_42() {
    insta::assert_snapshot!(render(&["42", "sonnet-1-trees", "moss"]));
}

#[test]
fn sonnet_1_trees_seed_7_knobs() {
    insta::assert_snapshot!(render(&["7", "sonnet-1-trees", "ember", "1.0", "0.3", "0.7", "0.5", "1", "-0.6"]));
}

#[test]
fn sonnet_1_forest_seed_42() {
    insta::assert_snapshot!(render(&["42", "sonnet-1-forest", "moss"]));
}

#[test]
fn sonnet_1_forest_drifting_t15() {
    insta::assert_snapshot!(render_t(&["7", "sonnet-1-forest", "moss"], "15"));
}
