use crossterm::style::Color;

pub fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb { r, g, b }
}

pub fn hsl_to_rgb(h: f64, s: f64, l: f64) -> Color {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r1, g1, b1) = match h as u32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Color::Rgb {
        r: ((r1 + m) * 255.0) as u8,
        g: ((g1 + m) * 255.0) as u8,
        b: ((b1 + m) * 255.0) as u8,
    }
}

/// Named themes. Each is [background, primary, secondary, accent, text].
pub fn named_theme(name: &str) -> Option<[Color; 5]> {
    Some(match name {
        // --- warm ---
        "ember" => [
            rgb(25, 8, 2),       // near-black warm
            rgb(204, 85, 0),     // burnt orange
            rgb(140, 40, 50),    // dried blood
            rgb(255, 160, 50),   // amber glow
            rgb(240, 220, 200),  // warm white
        ],
        "terracotta" => [
            rgb(30, 15, 10),     // dark earth
            rgb(180, 100, 60),   // clay
            rgb(90, 130, 80),    // sage
            rgb(220, 170, 100),  // sand
            rgb(235, 225, 210),  // parchment
        ],
        "sakura" => [
            rgb(20, 10, 18),     // dark plum
            rgb(200, 120, 160),  // cherry blossom
            rgb(100, 80, 90),    // bark
            rgb(255, 180, 200),  // petal pink
            rgb(240, 235, 240),  // soft white
        ],

        // --- cool ---
        "arctic" => [
            rgb(5, 10, 20),      // deep night
            rgb(100, 160, 220),  // ice blue
            rgb(60, 80, 120),    // steel
            rgb(180, 230, 255),  // frost
            rgb(230, 240, 250),  // snow
        ],
        "deep" => [
            rgb(2, 5, 15),       // abyss
            rgb(30, 80, 160),    // ocean
            rgb(80, 40, 120),    // purple depth
            rgb(50, 200, 180),   // bioluminescent
            rgb(200, 220, 240),  // foam
        ],
        "moss" => [
            rgb(8, 15, 5),       // forest floor
            rgb(80, 140, 60),    // moss green
            rgb(50, 80, 40),     // dark fern
            rgb(160, 200, 80),   // lichen
            rgb(210, 230, 200),  // pale green
        ],

        // --- monochrome ---
        "bone" => [
            rgb(15, 14, 12),     // charcoal
            rgb(180, 170, 155),  // bone
            rgb(120, 115, 105),  // stone
            rgb(220, 210, 190),  // ivory
            rgb(240, 235, 225),  // cream
        ],
        "silver" => [
            rgb(10, 10, 12),     // gunmetal
            rgb(140, 145, 160),  // silver
            rgb(80, 85, 95),     // pewter
            rgb(200, 205, 220),  // bright silver
            rgb(235, 235, 240),  // platinum
        ],

        // --- vivid ---
        "neon" => [
            rgb(5, 0, 10),       // void
            rgb(0, 255, 128),    // neon green
            rgb(255, 0, 128),    // hot pink
            rgb(0, 200, 255),    // cyan
            rgb(255, 255, 255),  // pure white
        ],
        "nerv" => [
            rgb(10, 2, 15),      // eva purple-black
            rgb(200, 50, 20),    // nerv red
            rgb(100, 60, 160),   // eva purple
            rgb(255, 180, 0),    // warning orange
            rgb(220, 220, 230),  // terminal gray
        ],
        "mitla" => [
            rgb(20, 12, 5),      // obsidian earth
            rgb(190, 140, 60),   // gold stone
            rgb(140, 60, 40),    // red clay
            rgb(100, 170, 130),  // jade
            rgb(230, 220, 200),  // limestone
        ],

        _ => return None,
    })
}

/// Seed-deterministic palette: rotate hue based on seed, derive harmonious colors.
/// Returns [background, primary, secondary, accent, text].
pub fn make_palette(seed: u64) -> [Color; 5] {
    let base_hue = (seed % 360) as f64;
    [
        hsl_to_rgb(base_hue, 0.3, 0.15),
        hsl_to_rgb((base_hue + 30.0) % 360.0, 0.6, 0.55),
        hsl_to_rgb((base_hue + 180.0) % 360.0, 0.5, 0.45),
        hsl_to_rgb((base_hue + 60.0) % 360.0, 0.7, 0.65),
        rgb(220, 220, 220),
    ]
}

/// Shift hue by extracting approximate HSL, rotating, converting back.
pub fn shift_hue(color: Color, degrees: f64) -> Color {
    match color {
        Color::Rgb { r, g, b } => {
            let (rf, gf, bf) = (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);
            let max = rf.max(gf).max(bf);
            let min = rf.min(gf).min(bf);
            let l = (max + min) / 2.0;
            if (max - min).abs() < 0.001 {
                return color; // achromatic
            }
            let d = max - min;
            let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
            let h = if (max - rf).abs() < 0.001 {
                ((gf - bf) / d + if gf < bf { 6.0 } else { 0.0 }) * 60.0
            } else if (max - gf).abs() < 0.001 {
                ((bf - rf) / d + 2.0) * 60.0
            } else {
                ((rf - gf) / d + 4.0) * 60.0
            };
            hsl_to_rgb((h + degrees).rem_euclid(360.0), s, l)
        }
        other => other,
    }
}

pub fn lighten(color: Color, amount: u8) -> Color {
    match color {
        Color::Rgb { r, g, b } => Color::Rgb {
            r: r.saturating_add(amount),
            g: g.saturating_add(amount),
            b: b.saturating_add(amount),
        },
        other => other,
    }
}

pub fn darken(color: Color, amount: u8) -> Color {
    match color {
        Color::Rgb { r, g, b } => Color::Rgb {
            r: r.saturating_sub(amount),
            g: g.saturating_sub(amount),
            b: b.saturating_sub(amount),
        },
        other => other,
    }
}
