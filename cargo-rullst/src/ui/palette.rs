//! Brand colour math for the terminal opening: the approved blue → green →
//! orange stops, the flowing animation palette and the 24-bit → xterm-256
//! fallback. Everything here is pure so the rendering stays unit tested.

/// One 24-bit colour.
pub(super) type Rgb = (u8, u8, u8);

/// Brand stops: blue → green → orange.
pub(super) const STOPS: [Rgb; 3] = [(40, 120, 255), (30, 205, 110), (255, 130, 25)];

/// How many colours the attached terminal can show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ColorDepth {
    /// No colour: plain, deterministic text.
    None,
    /// The xterm 256-colour palette.
    Ansi256,
    /// 24-bit RGB (`COLORTERM=truecolor` or `24bit`).
    TrueColor,
}

/// Linear interpolation; channels truncate like the approved prototype.
fn lerp(from: Rgb, to: Rgb, t: f64) -> Rgb {
    let channel = |a: u8, b: u8| (f64::from(a) + (f64::from(b) - f64::from(a)) * t) as u8;
    (
        channel(from.0, to.0),
        channel(from.1, to.1),
        channel(from.2, to.2),
    )
}

/// Linear blue → green → orange for `t` in `[0, 1]` (clamped).
pub(super) fn gradient(t: f64) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        lerp(STOPS[0], STOPS[1], t * 2.0)
    } else {
        lerp(STOPS[1], STOPS[2], (t - 0.5) * 2.0)
    }
}

/// While animating, the cyclic palette slides across the word; phase `0` is
/// exactly the final [`gradient`].
pub(super) fn flowing(t: f64, phase: f64) -> Rgb {
    if phase == 0.0 {
        return gradient(t);
    }
    let u = (t - phase).rem_euclid(1.0);
    if u < 1.0 / 3.0 {
        lerp(STOPS[0], STOPS[1], u * 3.0)
    } else if u < 2.0 / 3.0 {
        lerp(STOPS[1], STOPS[2], (u - 1.0 / 3.0) * 3.0)
    } else {
        lerp(STOPS[2], STOPS[0], (u - 2.0 / 3.0) * 3.0)
    }
}

const CUBE_LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];

/// Nearest level of the 6×6×6 xterm colour cube for one channel.
fn cube_index(value: u8) -> u8 {
    match value {
        0..48 => 0,
        48..115 => 1,
        _ => (value - 35) / 40,
    }
}

fn distance((r1, g1, b1): Rgb, (r2, g2, b2): Rgb) -> u32 {
    let delta = |a: u8, b: u8| u32::from(a.abs_diff(b)).pow(2);
    delta(r1, r2) + delta(g1, g2) + delta(b1, b2)
}

/// Nearest xterm-256 colour (cube 16–231 or grey ramp 232–255) for `rgb`.
/// The 16 system colours are skipped because terminal themes redefine them.
pub(super) fn nearest_xterm_256(rgb: Rgb) -> u8 {
    let (r, g, b) = (cube_index(rgb.0), cube_index(rgb.1), cube_index(rgb.2));
    let cube = (
        CUBE_LEVELS[usize::from(r)],
        CUBE_LEVELS[usize::from(g)],
        CUBE_LEVELS[usize::from(b)],
    );
    let average = (u16::from(rgb.0) + u16::from(rgb.1) + u16::from(rgb.2)) / 3;
    // Grey ramp levels are 8, 18, ..., 238.
    let grey_index = (average.saturating_sub(3) / 10).min(23) as u8;
    let grey_level = 8 + 10 * grey_index;
    if distance(rgb, cube) <= distance(rgb, (grey_level, grey_level, grey_level)) {
        16 + 36 * r + 6 * g + b
    } else {
        232 + grey_index
    }
}

/// The SGR sequence that selects `rgb` as the foreground at `depth`.
pub(super) fn foreground(rgb: Rgb, depth: ColorDepth) -> String {
    match depth {
        ColorDepth::None => String::new(),
        ColorDepth::Ansi256 => format!("\x1b[38;5;{}m", nearest_xterm_256(rgb)),
        ColorDepth::TrueColor => format!("\x1b[38;2;{};{};{}m", rgb.0, rgb.1, rgb.2),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradient_and_flow_match_the_approved_prototype() {
        // Reference values printed by APPROVED_opening_prototype.py.
        for (t, expected) in [
            (0.0, (40, 120, 255)),
            (0.25, (35, 162, 182)),
            (0.5, (30, 205, 110)),
            (0.75, (142, 167, 67)),
            (1.0, (255, 130, 25)),
            (0.1234, (37, 140, 219)),
            (-1.0, (40, 120, 255)),
            (2.0, (255, 130, 25)),
        ] {
            assert_eq!(gradient(t), expected, "gradient({t})");
        }
        for (t, phase, expected) in [
            (0.1, 0.5, (210, 145, 42)),
            (0.9, 0.2, (233, 129, 48)),
            (0.0, 0.999, (39, 120, 254)),
            (0.6, 0.3, (31, 196, 124)),
        ] {
            assert_eq!(flowing(t, phase), expected, "flowing({t}, {phase})");
        }
        assert_eq!(flowing(0.75, 0.0), gradient(0.75));
    }

    #[test]
    fn brand_colours_map_to_the_expected_xterm_256_entries() {
        for (rgb, expected) in [
            ((0, 0, 0), 16),
            ((255, 255, 255), 231),
            ((255, 0, 0), 196),
            ((128, 128, 128), 244),
            ((8, 8, 8), 232),
            ((238, 238, 238), 255),
            ((95, 135, 175), 67),
            (STOPS[0], 33),
            (STOPS[1], 41),
            (STOPS[2], 208),
            ((240, 240, 248), 255),
            ((110, 110, 130), 243),
            ((150, 155, 175), 247),
        ] {
            assert_eq!(nearest_xterm_256(rgb), expected, "{rgb:?}");
        }
    }

    fn xterm_rgb(index: u8) -> Rgb {
        if index >= 232 {
            let level = 8 + 10 * (index - 232);
            return (level, level, level);
        }
        let cube = index - 16;
        (
            CUBE_LEVELS[usize::from(cube / 36)],
            CUBE_LEVELS[usize::from((cube / 6) % 6)],
            CUBE_LEVELS[usize::from(cube % 6)],
        )
    }

    #[test]
    fn xterm_mapping_is_as_close_as_an_exhaustive_palette_search() {
        let palette: Vec<Rgb> = (16..=255).map(xterm_rgb).collect();
        for r in (0..=255u8).step_by(5) {
            for g in (0..=255u8).step_by(5) {
                for b in (0..=255u8).step_by(5) {
                    let rgb = (r, g, b);
                    let best = palette
                        .iter()
                        .map(|candidate| distance(rgb, *candidate))
                        .min()
                        .unwrap_or(u32::MAX);
                    let chosen = distance(rgb, xterm_rgb(nearest_xterm_256(rgb)));
                    assert_eq!(chosen, best, "{rgb:?}");
                }
            }
        }
    }

    #[test]
    fn foreground_sequences_follow_the_colour_depth() {
        assert_eq!(foreground((1, 2, 3), ColorDepth::None), "");
        assert_eq!(
            foreground((1, 2, 3), ColorDepth::TrueColor),
            "\x1b[38;2;1;2;3m"
        );
        assert_eq!(foreground(STOPS[2], ColorDepth::Ansi256), "\x1b[38;5;208m");
    }
}
