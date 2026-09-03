#![allow(warnings)]

mod _0_profile;

#[macro_use]
mod registry;

mod arboretum;
mod astrolabe;
mod automata;
mod avant;
mod biomes;
mod borders;
mod cli;
mod cli_basic;
mod cli_catalog;
mod cli_city;
mod cli_fa;
mod cli_forest;
mod cli_scenes;
mod color;
mod content;
mod fills;
mod gridio;
mod ink;
mod layout;
mod markdown;
mod modes;
mod modes_creatures;
mod modes_geo;
mod modes_sky;
mod modes_tree;
mod mondrian;
mod morph;
mod opts;
mod pp;
mod render;
mod sauron;
mod scene;
mod sprites;
mod tree_draw;
mod types;
mod walker;
mod warps;
mod mahoraga2;
mod mahoraga3;
mod mahoraga4;
mod mahoraga5;
mod lifetree;
mod lifetree2;
mod lifetree3;
mod lifetree4;
mod lifetree5;
mod lifetree6;
mod braid;
mod braid2;
mod chladni;
mod pendwave;
mod polytope;
mod poincare;
mod opus_1_quasicrystal;
mod opus_2_quasicrystal;
mod sonnet_1_spirograph;
mod sonnet_2_clifford;
mod haiku_1_torus;
mod haiku_2_ripple;
mod fable_1_trees;
mod fable_1_forest;
#[cfg(test)]
mod perf_sweep;

use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::io::{self, IsTerminal, Read as _};

use arboretum::*;
use astrolabe::*;
use automata::*;
use avant::*;
use biomes::*;
use sauron::*;
use borders::*;
use cli::*;
use cli_basic::*;
use cli_catalog::*;
use cli_city::*;
use cli_fa::*;
use cli_forest::*;
use cli_scenes::*;
use color::*;
use content::*;
use fills::*;
use gridio::*;
use ink::*;
use layout::*;
use markdown::*;
use modes_creatures::*;
use modes_geo::*;
use modes_sky::*;
use modes_tree::*;
use mondrian::*;
use morph::*;
use opts::*;
use registry::*;
use pp::*;
use render::*;
use scene::*;
use sprites::*;
use tree_draw::*;
use types::*;
use walker::*;
use warps::*;

fn main() {
    if let Err(error) = _0_profile::init() {
        eprintln!("profiling initialization failed: {error}");
    }
    cli::run();
}

#[cfg(test)]
mod tests {
    use super::*;
    use unicode_width::UnicodeWidthStr;

    fn assert_uniform_display_width(grid: &Grid, expected: usize) {
        let lines = grid_to_plain(grid);
        for (i, line) in lines.iter().enumerate() {
            let w = UnicodeWidthStr::width(line.as_str());
            assert_eq!(
                w, expected,
                "row {} has display width {} (expected {}): {:?}",
                i, w, expected, line,
            );
        }
    }

    fn make_grid(width: usize, height: usize, seed: u64) -> (Grid, StdRng, [Color; 5]) {
        let grid = vec![vec![Cell::blank(); width]; height];
        let rng = StdRng::seed_from_u64(seed);
        let palette = make_palette(seed);
        (grid, rng, palette)
    }

    fn grid_to_string(grid: &Grid) -> String {
        grid_to_plain(grid).join("\n")
    }

    #[test]
    fn ease_in_out_shape() {
        assert!((ease_in_out(0.0) - 0.0).abs() < 1e-6);
        assert!((ease_in_out(1.0) - 1.0).abs() < 1e-6);
        assert!((ease_in_out(0.5) - 0.5).abs() < 1e-6); // symmetric
        // middle is faster than the ends: derivative (delta over equal step) bigger at 0.5
        let d_end = ease_in_out(0.1) - ease_in_out(0.0);
        let d_mid = ease_in_out(0.55) - ease_in_out(0.45);
        assert!(d_mid > d_end, "middle should advance faster than the ends");
        // monotonic non-decreasing
        let mut prev = -1.0;
        for i in 0..=20 {
            let v = ease_in_out(i as f32 / 20.0);
            assert!(v >= prev - 1e-6, "must be monotonic");
            prev = v;
        }
    }

    #[test]
    fn warps_are_deterministic_and_animate() {
        let (mut g, mut rng, pal) = make_grid(80, 24, 4);
        draw_nebula(&mut g, 80, 24, 4, &pal, &mut rng, 0.0);
        for warp in [warp_drift, warp_swirl, warp_ripple, warp_breathe, warp_wind] {
            let a = warp(&g, 1.0, 2.0);
            assert_eq!(grid_to_string(&a), grid_to_string(&warp(&g, 1.0, 2.0)), "deterministic");
            let b = warp(&g, 4.5, 2.0);
            assert_ne!(grid_to_string(&a), grid_to_string(&b), "time should animate the warp");
        }
    }

    #[test]
    fn voronoi_flow_deterministic_and_moves() {
        let pal = make_palette(3);
        let f0 = voronoi_flow_frame(80, 24, 3, 0.0, &pal);
        let f0b = voronoi_flow_frame(80, 24, 3, 0.0, &pal);
        let f1 = voronoi_flow_frame(80, 24, 3, 5.0, &pal);
        assert_eq!(grid_to_string(&f0), grid_to_string(&f0b), "same args -> same frame");
        assert_ne!(grid_to_string(&f0), grid_to_string(&f1), "time should move the cells");
    }

    #[test]
    fn voronoi_flow_snapshot() {
        let pal = make_palette(3);
        insta::assert_snapshot!("voronoi_flow_t2", grid_to_string(&voronoi_flow_frame(80, 24, 3, 2.0, &pal)));
    }

    #[test]
    fn elevator_animates_and_deterministic() {
        let (mut a, mut ra, pal) = make_grid(80, 24, 9);
        draw_elevator(&mut a, 80, 24, 9, &pal, &mut ra, 0.0, 3, 1.0, 1.0);
        let (mut b, mut rb, _) = make_grid(80, 24, 9);
        draw_elevator(&mut b, 80, 24, 9, &pal, &mut rb, 0.0, 3, 1.0, 1.0);
        assert_eq!(grid_to_string(&a), grid_to_string(&b), "same args -> same frame");
        let (mut c, mut rc, _) = make_grid(80, 24, 9);
        draw_elevator(&mut c, 80, 24, 9, &pal, &mut rc, 2.0, 3, 1.0, 1.0);
        assert_ne!(grid_to_string(&a), grid_to_string(&c), "T should move the cabs");
    }

    #[test]
    fn ferris_animates_and_deterministic() {
        let (mut a, mut ra, pal) = make_grid(80, 24, 5);
        draw_ferris(&mut a, 80, 24, 5, &pal, &mut ra, 1.3, 8, 10, 1.0);
        let (mut b, mut rb, _) = make_grid(80, 24, 5);
        draw_ferris(&mut b, 80, 24, 5, &pal, &mut rb, 1.3, 8, 10, 1.0);
        assert_eq!(grid_to_string(&a), grid_to_string(&b), "same args -> same frame");
        let (mut c, mut rc, _) = make_grid(80, 24, 5);
        draw_ferris(&mut c, 80, 24, 5, &pal, &mut rc, 2.9, 8, 10, 1.0);
        assert_ne!(grid_to_string(&a), grid_to_string(&c), "T should turn the wheel");
    }

    #[test]
    fn warp_wind_moves_and_zero_amp_identity() {
        let (mut g, mut rng, pal) = make_grid(80, 24, 1);
        draw_phyllotaxis(&mut g, 80, 24, 1, &pal, &mut rng, 0.0);
        // amplitude 0 -> no displacement -> identity
        assert_eq!(grid_to_string(&warp_wind(&g, 5.0, 0.0)), grid_to_string(&g));
        // deterministic, and different times differ
        let a = warp_wind(&g, 0.3, 5.0);
        assert_eq!(grid_to_string(&a), grid_to_string(&warp_wind(&g, 0.3, 5.0)));
        assert_ne!(grid_to_string(&a), grid_to_string(&warp_wind(&g, 2.1, 5.0)));
    }

    #[test]
    fn grid_serialize_roundtrip() {
        let (mut grid, mut rng, palette) = make_grid(20, 6, 42);
        draw_phyllotaxis(&mut grid, 20, 6, 42, &palette, &mut rng, 0.0);
        let restored = parse_grid(&serialize_grid(&grid));
        assert_eq!(restored.len(), grid.len());
        assert_eq!(restored[0].len(), grid[0].len());
        for y in 0..grid.len() {
            for x in 0..grid[0].len() {
                assert_eq!(restored[y][x].ch, grid[y][x].ch, "ch at {},{}", x, y);
                assert_eq!(restored[y][x].fg, grid[y][x].fg, "fg at {},{}", x, y);
            }
        }
    }

    fn morph_pair() -> MorphState {
        let (mut a, mut ra, pa) = make_grid(80, 24, 1);
        draw_phyllotaxis(&mut a, 80, 24, 1, &pa, &mut ra, 0.0);
        let (mut b, mut rb, pb) = make_grid(80, 24, 7);
        draw_phyllotaxis(&mut b, 80, 24, 7, &pb, &mut rb, 0.0);
        MorphState::new(a, b)
    }

    #[test]
    fn morph_dissolve_mid() {
        insta::assert_snapshot!("morph_dissolve_mid", grid_to_string(&morph_pair().frame(0.5, "dissolve")));
    }

    #[test]
    fn morph_field_mid() {
        insta::assert_snapshot!("morph_field_mid", grid_to_string(&morph_pair().frame(0.5, "field")));
    }

    #[test]
    fn morph_transport_mid() {
        insta::assert_snapshot!("morph_transport_mid", grid_to_string(&morph_pair().frame(0.5, "transport")));
    }

    #[test]
    fn morph_sdf_mid() {
        insta::assert_snapshot!("morph_sdf_mid", grid_to_string(&morph_pair().frame(0.5, "sdf")));
    }

    #[test]
    fn morph_endpoints_recover_inputs() {
        // at t=0 dissolve should be ~grid A, at t=1 ~grid B (char identity).
        let st = morph_pair();
        let f0 = st.frame(0.0, "dissolve");
        let f1 = st.frame(1.0, "dissolve");
        let mut a_match = 0;
        let mut b_match = 0;
        for y in 0..st.h {
            for x in 0..st.w {
                if f0[y][x].ch == st.a[y][x].ch {
                    a_match += 1;
                }
                if f1[y][x].ch == st.b[y][x].ch {
                    b_match += 1;
                }
            }
        }
        let total = st.w * st.h;
        assert_eq!(a_match, total, "t=0 should equal A");
        assert_eq!(b_match, total, "t=1 should equal B");
    }

    #[test]
    fn demo_filter_empty_matches_all() {
        let modes = ["party", "soup", "tree"];
        assert_eq!(demo_filter_modes(&modes, ""), vec![0, 1, 2]);
    }

    #[test]
    fn demo_filter_substring_case_insensitive() {
        let modes = ["forest", "forest++", "eyes++", "FullMetal"];
        assert_eq!(demo_filter_modes(&modes, "forest"), vec![0, 1]);
        assert_eq!(demo_filter_modes(&modes, "++"), vec![1, 2]);
        assert_eq!(demo_filter_modes(&modes, "metal"), vec![3]);
        assert!(demo_filter_modes(&modes, "zzz").is_empty());
    }

    #[test]
    fn eyes_pp_42() {
        let (mut grid, mut rng, palette) = make_grid(80, 24, 42);
        draw_eyes_pp(&mut grid, 80, 24, 42, &palette, &mut rng);
        insta::assert_snapshot!("eyes_pp_42", grid_to_string(&grid));
    }

    #[test]
    fn fme_pp_42() {
        let (mut grid, mut rng, palette) = make_grid(80, 24, 42);
        draw_fme_pp(&mut grid, 80, 24, 42, &palette, &mut rng);
        insta::assert_snapshot!("fme_pp_42", grid_to_string(&grid));
    }

    #[test]
    fn trees_pp_42() {
        let (mut grid, mut rng, palette) = make_grid(80, 24, 42);
        draw_trees_pp(&mut grid, 80, 24, 42, &palette, &mut rng);
        insta::assert_snapshot!("trees_pp_42", grid_to_string(&grid));
    }

    #[test]
    fn forest_pp_42() {
        let (mut grid, mut rng, palette) = make_grid(80, 24, 42);
        draw_forest_pp(&mut grid, 80, 24, 42, &palette, &mut rng);
        insta::assert_snapshot!("forest_pp_42", grid_to_string(&grid));
    }

    #[test]
    fn phyllotaxis_42() {
        let (mut grid, mut rng, palette) = make_grid(80, 24, 42);
        draw_phyllotaxis(&mut grid, 80, 24, 42, &palette, &mut rng, 0.0);
        insta::assert_snapshot!("phyllotaxis_42", grid_to_string(&grid));
    }

    #[test]
    fn moire_42() {
        let (mut grid, mut rng, palette) = make_grid(80, 24, 42);
        draw_moire(&mut grid, 80, 24, 42, &palette, &mut rng, 0.0);
        insta::assert_snapshot!("moire_42", grid_to_string(&grid));
    }

    #[test]
    fn nebula_42() {
        let (mut grid, mut rng, palette) = make_grid(80, 24, 42);
        draw_nebula(&mut grid, 80, 24, 42, &palette, &mut rng, 0.0);
        insta::assert_snapshot!("nebula_42", grid_to_string(&grid));
    }

    #[test]
    fn delta_42() {
        let (mut grid, mut rng, palette) = make_grid(80, 24, 42);
        draw_delta(&mut grid, 80, 24, 42, &palette, &mut rng, 0.0);
        insta::assert_snapshot!("delta_42", grid_to_string(&grid));
    }

    #[test]
    fn circuit_42() {
        let (mut grid, mut rng, palette) = make_grid(80, 24, 42);
        draw_circuit(&mut grid, 80, 24, 42, &palette, &mut rng, 0.0, 14);
        insta::assert_snapshot!("circuit_42", grid_to_string(&grid));
    }

    #[test]
    fn snakes_42() {
        let (mut grid, mut rng, palette) = make_grid(80, 24, 42);
        draw_snakes(&mut grid, 80, 24, 42, &palette, &mut rng, 0.0, 7);
        insta::assert_snapshot!("snakes_42", grid_to_string(&grid));
    }

    #[test]
    fn options_persist_roundtrip() {
        let dir = std::env::temp_dir().join(format!("ascii-opt-test-{}", std::process::id()));
        let path = dir.join("options.tsv");
        let mut map: OptMap = std::collections::HashMap::new();
        {
            let m = map.entry("snakes".to_string()).or_default();
            m.insert("COUNT".to_string(), 30.0);
            m.insert("RBOW".to_string(), 1.0);
            m.insert("HOP".to_string(), 5.0); // out of range -> clamped on load via pvals_for
        }
        save_options_to(&path, &map);
        let back = load_options_from(&path);
        assert_eq!(back.get("snakes").and_then(|m| m.get("COUNT")).copied(), Some(30.0));

        // pvals_for applies saved values, clamped to each param's range.
        let spec = mode_spec("snakes");
        let pv = pvals_for(&spec, "snakes", &back);
        let count_i = spec.params.iter().position(|p| p.key == "COUNT").unwrap();
        let hop_i = spec.params.iter().position(|p| p.key == "HOP").unwrap();
        let turn_i = spec.params.iter().position(|p| p.key == "TURN").unwrap();
        assert_eq!(pv[count_i], 30.0, "saved value applied");
        assert_eq!(pv[hop_i], spec.params[hop_i].max, "out-of-range clamped to max");
        assert_eq!(pv[turn_i], spec.params[turn_i].default, "unset falls back to default");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn randomize_knobs_per_seed() {
        let spec = mode_spec("snakes");
        let zeros = vec![0.0f32; spec.params.len()];
        let a = effective_pvals(&spec, &zeros, 7, true, 0);
        let a2 = effective_pvals(&spec, &zeros, 7, true, 0);
        let b = effective_pvals(&spec, &zeros, 8, true, 0);
        let rolled = effective_pvals(&spec, &zeros, 7, true, 1);
        assert_eq!(a, a2, "randomize is deterministic for a given seed+roll");
        assert_ne!(a, b, "randomize re-rolls when the seed changes");
        assert_ne!(a, rolled, "bumping the roll nonce re-rolls the set");
        for (p, v) in spec.params.iter().zip(a.iter()) {
            assert!(*v >= p.min && *v <= p.max, "{} sampled in range", p.key);
        }
        // toggled off -> passes the tuned values straight through.
        let pv = vec![1.0f32; spec.params.len()];
        assert_eq!(effective_pvals(&spec, &pv, 7, false, 0), pv);
    }

    #[test]
    fn snakes_animate_and_deterministic() {
        let frame = |t: f32| {
            let (mut g, mut r, p) = make_grid(80, 24, 42);
            draw_snakes(&mut g, 80, 24, 42, &p, &mut r, t, 7);
            grid_to_string(&g)
        };
        assert_eq!(frame(1.3), frame(1.3), "same t -> same frame");
        assert_ne!(frame(0.0), frame(4.0), "t should slither the snakes");
    }

    #[test]
    fn hypercube_uses_seed_and_time_deterministically() {
        let frame = |seed: u64, t: f32| {
            let (mut g, mut r, p) = make_grid(80, 24, seed);
            draw_hypercube(&mut g, 80, 24, seed, &p, &mut r, t, 3, 1.0, 2);
            grid_to_string(&g)
        };
        assert_eq!(frame(42, 1.25), frame(42, 1.25));
        assert_ne!(frame(42, 0.0), frame(42, 2.0), "T should rotate the projection");
        assert_ne!(frame(42, 0.0), frame(43, 0.0), "seed should change the projection");
    }

    #[test]
    fn flux_uses_seed_and_time_deterministically() {
        let frame = |seed: u64, t: f32| {
            let (mut g, mut r, p) = make_grid(80, 24, seed);
            draw_flux(&mut g, 80, 24, seed, &p, &mut r, t, 58, 8, 1.0);
            grid_to_string(&g)
        };
        assert_eq!(frame(42, 1.25), frame(42, 1.25));
        assert_ne!(frame(42, 0.0), frame(42, 1.0), "T should advect the particles");
        assert_ne!(frame(42, 0.0), frame(43, 0.0), "seed should change the currents");
    }

    #[test]
    fn fireworks_use_seed_and_time_deterministically() {
        let frame = |seed: u64, t: f32| {
            let (mut g, mut r, p) = make_grid(80, 24, seed);
            draw_fireworks(&mut g, 80, 24, seed, &p, &mut r, t, 6, 22, 1.0);
            grid_to_string(&g)
        };
        assert_eq!(frame(42, 1.25), frame(42, 1.25));
        assert_ne!(frame(42, 0.0), frame(42, 1.0), "T should advance the bursts");
        assert_ne!(frame(42, 0.0), frame(43, 0.0), "seed should change the show");
    }

    #[test]
    fn fa6_uses_seed_and_time_deterministically() {
        let frame = |seed: u64, t: f32| {
            let (mut g, mut r, p) = make_grid(80, 24, seed);
            draw_fa6(&mut g, 80, 24, seed, &p, &mut r, t, 8, 55, 0.8, 0.42);
            grid_to_string(&g)
        };
        assert_eq!(frame(42, 1.25), frame(42, 1.25));
        assert_ne!(frame(42, 0.0), frame(42, 1.5), "T should animate the ritual field");
        assert_ne!(frame(42, 0.0), frame(43, 0.0), "seed should restructure the chambers");
    }

    #[test]
    fn murmuration_uses_seed_and_time_deterministically() {
        let frame = |seed: u64, t: f32| {
            let (mut g, mut r, p) = make_grid(80, 24, seed);
            draw_murmuration(&mut g, 80, 24, seed, &p, &mut r, t, 140, 3, 1.0);
            grid_to_string(&g)
        };
        assert_eq!(frame(42, 1.25), frame(42, 1.25));
        assert_ne!(frame(42, 0.0), frame(42, 2.0), "T should wheel the flocks");
        assert_ne!(frame(42, 0.0), frame(43, 0.0), "seed should reshape the murmuration");
    }

    #[test]
    fn lanterns_use_seed_and_time_deterministically() {
        let frame = |seed: u64, t: f32| {
            let (mut g, mut r, p) = make_grid(80, 24, seed);
            draw_lanterns(&mut g, 80, 24, seed, &p, &mut r, t, 7, 1.0, 1.0);
            grid_to_string(&g)
        };
        assert_eq!(frame(42, 1.25), frame(42, 1.25));
        assert_ne!(frame(42, 0.0), frame(42, 2.0), "T should lift the lanterns");
        assert_ne!(frame(42, 0.0), frame(43, 0.0), "seed should relayout the launch");
    }

    #[test]
    fn tide_uses_seed_and_time_deterministically() {
        let frame = |seed: u64, t: f32| {
            let (mut g, mut r, p) = make_grid(80, 24, seed);
            draw_tide(&mut g, 80, 24, seed, &p, &mut r, t, 2, 1.0, 1.0);
            grid_to_string(&g)
        };
        assert_eq!(frame(42, 1.25), frame(42, 1.25));
        assert_ne!(frame(42, 0.0), frame(42, 2.0), "T should move the surf");
        assert_ne!(frame(42, 0.0), frame(43, 0.0), "seed should reshape the shoreline");
    }

    #[test]
    fn circuit_topology_stable_across_time() {
        // The current pulse only recolors cells; the trace topology (chars) must
        // be identical at every t.
        let (mut g0, mut r0, p0) = make_grid(80, 24, 42);
        draw_circuit(&mut g0, 80, 24, 42, &p0, &mut r0, 0.0, 14);
        let (mut g1, mut r1, p1) = make_grid(80, 24, 42);
        draw_circuit(&mut g1, 80, 24, 42, &p1, &mut r1, 2.5, 14);
        assert_eq!(grid_to_string(&g0), grid_to_string(&g1), "chars stable over t");
    }

    #[test]
    fn stained_42() {
        let (mut grid, mut rng, palette) = make_grid(80, 24, 42);
        draw_stained(&mut grid, 80, 24, 42, &palette, &mut rng);
        insta::assert_snapshot!("stained_42", grid_to_string(&grid));
    }

    #[test]
    fn mondrian_display_width() {
        let (mut grid, mut rng, _) = make_grid(80, 45, 42);
        let blocks = vec![
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 STATUS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("All systems operational.".into()),
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("METRICS".into()),
                    ContentItem::Rule,
                    ContentItem::Bar {
                        label: "cpu".into(),
                        value: 72.0,
                        max: 100.0,
                    },
                    ContentItem::Bar {
                        label: "mem".into(),
                        value: 4.8,
                        max: 8.0,
                    },
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 SKILLS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("typespec ···· 12".into()),
                    ContentItem::Text("ast-grep ···· 5".into()),
                    ContentItem::Text("tree-sit ···· 3".into()),
                ],
                padding: 1,
            },
        ];
        let (_, line_color) = mondrian_colors();
        let text_fg = rgb(20, 20, 20);
        let (fills, _) = mondrian_colors();
        layout_mondrian(
            &mut grid, &blocks, 0, 2, 10, 5, text_fg, line_color, &fills, line_color, &mut rng,
        );
        assert_uniform_display_width(&grid, 80);
    }

    #[test]
    fn mondrian_different_seeds_display_width() {
        for seed in [0, 1, 7, 42, 99, 1234] {
            let (mut grid, mut rng, _) = make_grid(80, 45, seed);
            let blocks = vec![ContentBlock {
                items: vec![
                    ContentItem::Text("「 STATUS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("Online.".into()),
                ],
                padding: 1,
            }];
            let (fills, line_color) = mondrian_colors();
            layout_mondrian(
                &mut grid,
                &blocks,
                0,
                2,
                10,
                5,
                rgb(20, 20, 20),
                line_color,
                &fills,
                line_color,
                &mut rng,
            );
            assert_uniform_display_width(&grid, 80);
        }
    }

    #[test]
    fn default_mode_display_width() {
        let (mut grid, mut rng, palette) = make_grid(80, 45, 42);
        let truchet_color = darken(palette[1], 80);
        let tiles = ['╱', '╲'];
        for y in 0..45 {
            for x in 0..80 {
                grid[y][x] = Cell::new(tiles[rng.random_range(0..2)], truchet_color);
            }
        }
        let cx = 40;
        let cy = 22;
        let lines = ["「 技 」 S K I L L S", "", "  typespec ···· 12"];
        for (i, line) in lines.iter().enumerate() {
            let mut col = 0usize;
            for ch in line.chars() {
                let cw = char_width(ch);
                let gx = cx - 15 + col;
                if gx < 80 {
                    grid[cy - 5 + 1 + i][gx] = Cell::new(ch, palette[4]);
                    if cw == 2 && gx + 1 < 80 {
                        grid[cy - 5 + 1 + i][gx + 1] = Cell::blank();
                    }
                }
                col += cw;
            }
        }
        assert_uniform_display_width(&grid, 80);
    }

    #[test]
    fn bsp_display_width() {
        let (mut grid, mut rng, palette) = make_grid(80, 45, 42);
        let truchet_color = darken(palette[1], 90);
        let tiles = ['╱', '╲'];
        for y in 0..45 {
            for x in 0..80 {
                grid[y][x] = Cell::new(tiles[rng.random_range(0..2)], truchet_color);
            }
        }
        let blocks = vec![
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 STATUS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("All systems operational.".into()),
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("TASKS".into()),
                    ContentItem::Rule,
                    ContentItem::Text("▪ layout engine".into()),
                ],
                padding: 1,
            },
        ];
        layout_bsp(
            &mut grid, &blocks, 1, 12, 5, palette[4], palette[3], &mut rng,
        );
        assert_uniform_display_width(&grid, 80);
    }

    #[test]
    fn wrap_text_fullwidth_chars() {
        let lines = wrap_text("「 X 」", 7);
        assert_eq!(lines, vec!["「 X 」"]);

        let lines = wrap_text("「 X 」 extra", 7);
        assert_eq!(lines, vec!["「 X 」", "extra"]);
    }

    #[test]
    fn wrap_text_ascii_basic() {
        let lines = wrap_text("hello world foo", 11);
        assert_eq!(lines, vec!["hello world", "foo"]);
    }

    #[test]
    fn min_block_width_accounts_for_fullwidth() {
        let block = ContentBlock {
            items: vec![ContentItem::Text("「 SKILLS 」".into())],
            padding: 1,
        };
        assert_eq!(min_block_width(&block), 14);
    }

    #[test]
    fn bsp_split_gap_leaves_cover_canvas() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut root = BspNode::new(0, 0, 80, 45);
        root.split_with_gap(10, 5, 4, 2, &mut rng);
        let leaves = root.leaves();
        assert!(leaves.len() >= 2, "should produce multiple leaves");
        for leaf in &leaves {
            assert!(leaf.x + leaf.w <= 80, "leaf x overflow");
            assert!(leaf.y + leaf.h <= 45, "leaf y overflow");
            assert!(leaf.w >= 10, "leaf too narrow");
            assert!(leaf.h >= 5, "leaf too short");
        }
    }

    #[test]
    fn bsp_split_gap1_backward_compat() {
        let mut rng1 = StdRng::seed_from_u64(99);
        let mut rng2 = StdRng::seed_from_u64(99);
        let mut a = BspNode::new(0, 0, 80, 45);
        let mut b = BspNode::new(0, 0, 80, 45);
        a.split(10, 5, 4, &mut rng1);
        b.split_with_gap(10, 5, 4, 1, &mut rng2);
        let la: Vec<_> = a.leaves().iter().map(|r| (r.x, r.y, r.w, r.h)).collect();
        let lb: Vec<_> = b.leaves().iter().map(|r| (r.x, r.y, r.w, r.h)).collect();
        assert_eq!(la, lb);
    }

    #[test]
    fn mondrian_content_not_wrapped() {
        let (mut grid, mut rng, _) = make_grid(80, 45, 42);
        let blocks = vec![ContentBlock {
            items: vec![ContentItem::Text("「 SKILLS 」".into())],
            padding: 1,
        }];
        let (fills, line_color) = mondrian_colors();
        layout_mondrian(
            &mut grid,
            &blocks,
            0,
            2,
            10,
            5,
            rgb(20, 20, 20),
            line_color,
            &fills,
            line_color,
            &mut rng,
        );
        let lines = grid_to_plain(&grid);
        let skill_rows: Vec<_> = lines.iter().filter(|l| l.contains("SKILLS")).collect();
        assert_eq!(
            skill_rows.len(),
            1,
            "「 SKILLS 」 should appear on exactly one row"
        );
        assert!(
            skill_rows[0].contains("「 SKILLS 」"),
            "full title should be on one line, got: {:?}",
            skill_rows[0]
        );
    }

    #[test]
    fn scene_walk_produces_layers() {
        let mut rng = StdRng::seed_from_u64(42);
        let palette = make_palette(42);
        let mut root = layout::BspNode::new(0, 0, 80, 45);
        root.split_with_gap(12, 6, 4, 2, &mut rng);
        let leaves: Vec<Rect> = root.leaves().into_iter().copied().collect();
        let layers = walk_to_layers(&leaves, (40, 22), &palette, &mut rng);
        assert!(layers.len() > 0, "walker should produce at least one layer");
        assert!(
            layers.len() <= leaves.len() * 4,
            "layers bounded by leaves + scatter"
        );
        for layer in &layers {
            assert!(
                layer.mask.is_some(),
                "every scene-walk layer should be masked"
            );
        }
    }

    #[test]
    fn scene_walk_renders_without_panic() {
        for seed in [0, 1, 7, 42, 99, 1234] {
            let (mut grid, mut rng, palette) = make_grid(80, 45, seed);
            let mut root = layout::BspNode::new(0, 0, 80, 45);
            root.split_with_gap(12, 6, 4, 2, &mut rng);
            let leaves: Vec<Rect> = root.leaves().into_iter().copied().collect();
            let layers = walk_to_layers(&leaves, (40, 22), &palette, &mut rng);
            let scene = Scene { layers };
            let rect = Rect {
                x: 0,
                y: 0,
                w: 80,
                h: 45,
            };
            render_scene(&mut grid, &rect, &scene, &mut rng);
            assert_uniform_display_width(&grid, 80);
        }
    }

    #[test]
    fn scene_walk_deterministic() {
        let run = |seed: u64| {
            let mut rng = StdRng::seed_from_u64(seed);
            let palette = make_palette(seed);
            let mut root = layout::BspNode::new(0, 0, 60, 30);
            root.split_with_gap(10, 5, 4, 2, &mut rng);
            let leaves: Vec<Rect> = root.leaves().into_iter().copied().collect();
            let layers = walk_to_layers(&leaves, (30, 15), &palette, &mut rng);
            let mut grid = vec![vec![Cell::blank(); 60]; 30];
            let rect = Rect {
                x: 0,
                y: 0,
                w: 60,
                h: 30,
            };
            let scene = Scene { layers };
            render_scene(&mut grid, &rect, &scene, &mut rng);
            grid_to_plain(&grid)
        };
        assert_eq!(run(42), run(42));
        assert_ne!(run(42), run(99));
    }

    #[test]
    fn tile_edge_seigaiha_skew_deterministic() {
        // Seigaiha with skew should produce identical output for same seed
        let run = |seed: u64| {
            let (mut grid, mut rng, palette) = make_grid(40, 20, seed);
            let rect = Rect {
                x: 5,
                y: 3,
                w: 25,
                h: 12,
            };
            let params = TileParams {
                variant: TileVariant::Seigaiha,
                density: 1.0,
                stagger_override: -1,
                rhythm_override: 0,
                jitter: 0.0,
                skew: 60,
            };
            fill_tile_ex(
                &mut grid, &rect, &params, palette[1], palette[2], 0.0, None, &mut rng,
            );
            grid_to_plain(&grid)
        };
        assert_eq!(run(42), run(42));
        assert_ne!(run(42), run(99));
    }

    #[test]
    fn tile_edge_skew_bleeds_past_rect() {
        // With skew>0, cells outside the rect should get drawn
        let (mut grid, mut rng, palette) = make_grid(40, 20, 42);
        let rect = Rect {
            x: 10,
            y: 5,
            w: 15,
            h: 8,
        };
        let params = TileParams {
            variant: TileVariant::Seigaiha,
            density: 1.0,
            stagger_override: -1,
            rhythm_override: 0,
            jitter: 0.0,
            skew: 80,
        };
        fill_tile_ex(
            &mut grid, &rect, &params, palette[1], palette[2], 0.0, None, &mut rng,
        );

        // Check that at least some cells outside the rect got drawn
        let mut outside_drawn = 0;
        for y in 0..20 {
            for x in 0..40 {
                let inside =
                    x >= rect.x && x < rect.x + rect.w && y >= rect.y && y < rect.y + rect.h;
                if !inside && grid[y][x].ch != ' ' {
                    outside_drawn += 1;
                }
            }
        }
        assert!(
            outside_drawn > 0,
            "skew=80 should bleed chars outside the rect"
        );
    }

    #[test]
    fn tile_edge_all_variants_no_panic() {
        // Every variant with skew should render without panic
        for vi in 0..TILE_VARIANT_COUNT {
            let variant = tile_variant_from_index(vi);
            for skew in [0, 30, 60, 100] {
                let (mut grid, mut rng, palette) = make_grid(30, 15, 42);
                let rect = Rect {
                    x: 3,
                    y: 2,
                    w: 20,
                    h: 10,
                };
                let params = TileParams {
                    variant,
                    density: 1.0,
                    stagger_override: -1,
                    rhythm_override: 0,
                    jitter: 0.0,
                    skew,
                };
                fill_tile_ex(
                    &mut grid, &rect, &params, palette[1], palette[2], 0.0, None, &mut rng,
                );
            }
        }
    }
}
