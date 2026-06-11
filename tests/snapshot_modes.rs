use std::process::Command;

/// Run the renderer with given args and return output with ANSI codes stripped.
fn render(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_ascii-renderer"))
        .args(args)
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
                if next.is_ascii_alphabetic() { break; }
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
    insta::assert_snapshot!(render(&["42", "party", "ember", "0", "6", "50", "50", "none", "line", "0"]));
}

#[test]
fn party_seed_99() {
    insta::assert_snapshot!(render(&["99", "party", "midnight", "0", "5", "50", "50", "stars", "vine", "30"]));
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
fn forest7_seed_42() {
    insta::assert_snapshot!(render(&["42", "forest7", "ember"]));
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
