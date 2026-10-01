use super::*;

fn rendered(out: Vec<u8>) -> String {
    String::from_utf8(out).unwrap()
}

#[test]
fn the_plain_opening_names_the_installed_version() {
    assert_eq!(
        PLAIN_SLOGAN,
        format!(
            "RULLST v{} · SECURE, FAST AND AI-NATIVE RUST FRAMEWORK",
            env!("CARGO_PKG_VERSION")
        )
    );
    let mut out = Vec::new();
    print_static(&mut out, ColorDepth::None).unwrap();
    assert_eq!(rendered(out), format!("{PLAIN_SLOGAN}\n\n"));
}

#[test]
fn the_wordmark_is_the_approved_text_without_a_badge() {
    assert_eq!(wordmark_width(), 50);
    assert!(WORDMARK.iter().all(|line| line.chars().count() == 50));
    let plain = wordmark_lines(0.0, ColorDepth::None);
    assert_eq!(plain.len(), 6);
    assert_eq!(plain[0], format!("{MARGIN}{}", WORDMARK[0]));
    assert!(
        plain
            .iter()
            .all(|line| !line.contains('▀') && !line.contains('▄'))
    );
}

/// Channel sums over every coloured cell, printed by the approved prototype
/// (`APPROVED_opening_prototype.py`) for the same phases.
#[test]
fn every_frame_matches_the_approved_prototype_colours() {
    let width = wordmark_width();
    let sums = |phase: f64| {
        let mut totals = (0u32, 0u32, 0u32, 0u32);
        for (row, line) in WORDMARK.iter().enumerate() {
            for (column, character) in line.chars().enumerate() {
                if character != ' ' {
                    let (r, g, b) = cell_color(row, column, width, phase);
                    totals.0 += u32::from(r);
                    totals.1 += u32::from(g);
                    totals.2 += u32::from(b);
                    totals.3 += 1;
                }
            }
        }
        totals
    };
    assert_eq!(sums(0.0), (21655, 34884, 26063, 217));
    assert_eq!(sums(START_PHASE), (21652, 32386, 30584, 217));
    assert_eq!(sums(0.5), (24522, 33406, 25910, 217));
    assert_eq!(sums(1.0 - 10.0 / 42.0), (21277, 33440, 29028, 217));
    assert_eq!(sums(1.0 - 41.0 / 42.0), (22091, 32319, 30271, 217));

    assert_eq!(cell_color(0, 0, width, 0.0), (40, 120, 255));
    assert_eq!(cell_color(0, 25, width, 0.0), (44, 200, 104));
    assert_eq!(cell_color(3, 10, width, 0.0), (34, 162, 182));
    assert_eq!(cell_color(0, 25, width, START_PHASE), (164, 160, 59));
    assert_eq!(cell_color(3, 10, width, 0.5), (200, 127, 83));
}

#[test]
fn colour_depth_selects_truecolor_or_the_nearest_xterm_entry() {
    let truecolor = wordmark_lines(0.0, ColorDepth::TrueColor);
    assert!(truecolor[0].starts_with("  \x1b[38;2;40;120;255m█"));
    assert!(truecolor[0].ends_with(RESET));

    let ansi = wordmark_lines(0.0, ColorDepth::Ansi256);
    assert!(ansi[0].starts_with("  \x1b[38;5;33m█"));
    assert!(!ansi.concat().contains("38;2;"));
    // Runs of one quantized colour share a single escape sequence.
    assert!(ansi[0].matches("\x1b[38;5;").count() < 50);
}

#[test]
fn the_slogan_is_bold_with_brand_keywords_and_a_plain_separator() {
    let version = env!("CARGO_PKG_VERSION");
    assert_eq!(
        slogan_line(ColorDepth::TrueColor),
        format!(
            "   \x1b[1m\x1b[38;2;240;240;248mv{version}\x1b[0m\
             \x1b[38;2;110;110;130m  ·  \x1b[0m\
             \x1b[1m\x1b[38;2;40;120;255mSECURE\x1b[0m\
             \x1b[1m\x1b[38;2;150;155;175m, \x1b[0m\
             \x1b[1m\x1b[38;2;30;205;110mFAST\x1b[0m\
             \x1b[1m\x1b[38;2;150;155;175m AND \x1b[0m\
             \x1b[1m\x1b[38;2;255;130;25mAI-NATIVE\x1b[0m\
             \x1b[1m\x1b[38;2;150;155;175m RUST FRAMEWORK\x1b[0m"
        )
    );
    assert!(slogan_line(ColorDepth::Ansi256).contains("\x1b[1m\x1b[38;5;208mAI-NATIVE"));
}

#[test]
fn the_animation_takes_about_seven_tenths_and_ends_on_the_gradient() {
    let phases: Vec<f64> = animation_phases().collect();
    assert_eq!(phases.len(), 42);
    assert_eq!(phases.last().copied(), Some(0.0));
    assert!((phases[0] - (1.0 - 1.0 / 42.0)).abs() < f64::EPSILON);
    assert!(phases.windows(2).all(|pair| pair[0] > pair[1]));
    let total = FRAME_DELAY * STEPS;
    assert_eq!(total, Duration::from_millis(714));
}

#[test]
fn a_full_animation_draws_every_frame_and_settles_on_the_final_one() {
    let mut out = Vec::new();
    let mut waits = 0;
    let outcome = play(&mut out, ColorDepth::TrueColor, |delay| {
        assert_eq!(delay, FRAME_DELAY);
        waits += 1;
        KeyWait::Elapsed
    })
    .unwrap();
    assert_eq!(outcome, KeyWait::Elapsed);
    assert_eq!(waits, 42);
    let output = rendered(out);
    assert_eq!(output.matches("\x1b[8A").count(), 42);
    let last_frame = output.rsplit("\x1b[8A").next().unwrap();
    for line in frame_lines(0.0, ColorDepth::TrueColor) {
        assert!(last_frame.contains(&format!("\r\x1b[2K{line}\r\n")));
    }
    assert!(output.ends_with("\r\n\r\n"));
}

#[test]
fn any_key_skips_to_the_final_frame_and_ctrl_c_interrupts() {
    for (key, expected) in [
        (KeyWait::Skip, KeyWait::Skip),
        (KeyWait::Interrupt, KeyWait::Interrupt),
    ] {
        let mut out = Vec::new();
        let mut waits = 0;
        let outcome = play(&mut out, ColorDepth::Ansi256, |_| {
            waits += 1;
            if waits == 3 { key } else { KeyWait::Elapsed }
        })
        .unwrap();
        assert_eq!(outcome, expected);
        assert_eq!(waits, 3);
        let output = rendered(out);
        // Two normal frames, then the final frame drawn immediately.
        assert_eq!(output.matches("\x1b[8A").count(), 3);
        let last_frame = output.rsplit("\x1b[8A").next().unwrap();
        for line in frame_lines(0.0, ColorDepth::Ansi256) {
            assert!(last_frame.contains(&line));
        }
    }
}

#[test]
fn static_profiles_never_animate_or_touch_the_daily_stamp() {
    for color in [ColorDepth::None, ColorDepth::Ansi256, ColorDepth::TrueColor] {
        let profile = TerminalProfile {
            color,
            motion: false,
            interactive: true,
        };
        let mut out = Vec::new();
        print_opening(&profile, &mut out).unwrap();
        let output = rendered(out);
        assert!(!output.contains("\x1b[8A"));
        assert!(!output.contains("\x1b[?25l"));
        if color == ColorDepth::None {
            assert_eq!(output, format!("{PLAIN_SLOGAN}\n\n"));
        } else {
            assert_eq!(output.lines().count(), 10);
            assert!(output.contains("SECURE"));
        }
    }
}
