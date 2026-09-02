//! Animated, terminal-native branding for the interactive command interface.

use std::io::{IsTerminal, Write};

const LOGO: [&str; 6] = [
    r#"  ██████╗ ██╗   ██╗██╗     ██╗     ███████╗████████╗"#,
    r#"  ██╔══██╗██║   ██║██║     ██║     ██╔════╝╚══██╔══╝"#,
    r#"  ██████╔╝██║   ██║██║     ██║     ███████╗   ██║   "#,
    r#"  ██╔══██╗██║   ██║██║     ██║     ╚════██║   ██║   "#,
    r#"  ██║  ██║╚██████╔╝███████╗███████╗███████║   ██║   "#,
    r#"  ╚═╝  ╚═╝ ╚═════╝ ╚══════╝╚══════╝╚══════╝   ╚═╝   "#,
];

// Saturated ANSI-256 colors remain predictable on terminals that do not
// implement 24-bit RGB faithfully. The opening pauses on complete blue, green,
// and orange gradients before settling on the stable three-color signature.
const BLUE_GRADIENT: [u8; 6] = [69, 63, 33, 27, 21, 20];
const GREEN_GRADIENT: [u8; 6] = [118, 82, 46, 40, 34, 28];
const ORANGE_GRADIENT: [u8; 6] = [215, 214, 208, 202, 166, 130];
const FINAL_SIGNATURE: [u8; 6] = [33, 27, 46, 34, 215, 166];
const PULSE_COLORS: [u8; 6] = FINAL_SIGNATURE;
const ANIMATION_FRAMES: [usize; 24] = [
    0, 0, 0, 1, 2, 3, 4, 5, 6, 6, 6, 7, 8, 9, 10, 11, 12, 12, 12, 13, 14, 15, 16, 16,
];

pub(super) fn print_neon_logo() -> std::io::Result<()> {
    let mut stdout = std::io::stdout();
    let animation = if visual_effects_enabled() {
        &ANIMATION_FRAMES[..]
    } else {
        &[16][..]
    };
    writeln!(stdout)?;
    for (index, frame) in animation.iter().copied().enumerate() {
        if index > 0 {
            write!(stdout, "\x1B[{}A", LOGO.len())?;
        }
        for line in LOGO {
            writeln!(stdout, "\r\x1B[2K{}", color_wave(line, frame))?;
        }
        stdout.flush()?;
        if index + 1 < animation.len() {
            std::thread::sleep(std::time::Duration::from_millis(110));
        }
    }
    writeln!(
        stdout,
        "\n  {}  {}",
        menu_icon("◆", (255, 60, 190)),
        paint_256(
            "RULLST v12 // SECURE • FAST • EXPLICIT • AI-NATIVE • WEB-FIRST",
            51
        )
    )?;
    writeln!(
        stdout,
        "  {}\n",
        paint_256(
            "THE FULL-STACK RUST TOOLKIT, BUILT WITHOUT RUNTIME MAGIC",
            46
        )
    )?;
    stdout.flush()
}

fn color_wave(line: &str, frame: usize) -> String {
    if !colors_enabled() {
        return line.to_string();
    }
    line.chars()
        .enumerate()
        .map(|(column, character)| {
            if character.is_whitespace() {
                character.to_string()
            } else {
                let letter = match column {
                    0..=8 => 0,
                    9..=18 => 1,
                    19..=26 => 2,
                    27..=34 => 3,
                    35..=42 => 4,
                    _ => 5,
                };
                paint_256(&character.to_string(), logo_color(frame, letter))
            }
        })
        .collect()
}

fn logo_color(frame: usize, letter: usize) -> u8 {
    if frame >= 16 {
        FINAL_SIGNATURE[letter]
    } else if frame <= 6 {
        if letter < frame {
            GREEN_GRADIENT[letter]
        } else {
            BLUE_GRADIENT[letter]
        }
    } else if frame <= 12 {
        if letter < frame - 6 {
            ORANGE_GRADIENT[letter]
        } else {
            GREEN_GRADIENT[letter]
        }
    } else {
        let blue_count = (frame - 12).min(2);
        let green_count = frame.saturating_sub(14).min(2);
        if letter < blue_count || (2..2 + green_count).contains(&letter) {
            FINAL_SIGNATURE[letter]
        } else {
            ORANGE_GRADIENT[letter]
        }
    }
}

pub(super) fn play_launch_pulse() -> std::io::Result<()> {
    let mut stdout = std::io::stdout();
    if !visual_effects_enabled() {
        writeln!(
            stdout,
            "  {} COMMAND INTERFACE READY",
            menu_icon("◆", (65, 255, 170))
        )?;
        return stdout.flush();
    }
    for (index, frame) in ["◇", "◈", "◆", "◈", "◇", "◆"].into_iter().enumerate() {
        let color = PULSE_COLORS[index % PULSE_COLORS.len()];
        write!(
            stdout,
            "\r  {} {}",
            paint_256(frame, color),
            paint_256("RULLST // COMMAND INTERFACE", color)
        )?;
        stdout.flush()?;
        std::thread::sleep(std::time::Duration::from_millis(130));
    }
    writeln!(
        stdout,
        "\r  {} {}                    ",
        menu_icon("◆", (65, 255, 170)),
        paint_256("COMMAND INTERFACE READY", 51)
    )?;
    stdout.flush()
}

pub(super) fn menu_icon(symbol: &str, color: (u8, u8, u8)) -> String {
    if colors_enabled() {
        paint_256(symbol, nearest_ansi_256(color))
    } else {
        symbol.to_string()
    }
}

fn paint_256(value: &str, color: u8) -> String {
    if colors_enabled() {
        format!("\x1b[38;5;{color}m{value}\x1b[0m")
    } else {
        value.to_string()
    }
}

const fn nearest_ansi_256((red, green, blue): (u8, u8, u8)) -> u8 {
    if green > red.saturating_add(25) && green > blue.saturating_add(10) {
        46
    } else if red > 220 && green > 120 && blue < 120 {
        208
    } else if red > 180 && blue > 130 {
        201
    } else if blue > red.saturating_add(25) && green > 150 {
        51
    } else if blue > red.saturating_add(25) {
        39
    } else if red > 200 {
        197
    } else {
        220
    }
}

fn colors_enabled() -> bool {
    std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}

fn visual_effects_enabled() -> bool {
    colors_enabled()
        && !std::env::var("RULLST_REDUCED_MOTION")
            .ok()
            .is_some_and(|value| {
                matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes")
            })
}

#[cfg(test)]
mod tests {
    #[test]
    fn logo_visits_each_full_gradient_then_settles_on_signature() {
        let blue = (0..6)
            .map(|letter| super::logo_color(0, letter))
            .collect::<Vec<_>>();
        let green = (0..6)
            .map(|letter| super::logo_color(6, letter))
            .collect::<Vec<_>>();
        let orange = (0..6)
            .map(|letter| super::logo_color(12, letter))
            .collect::<Vec<_>>();
        let signature = (0..6)
            .map(|letter| super::logo_color(16, letter))
            .collect::<Vec<_>>();

        assert_eq!(blue, super::BLUE_GRADIENT);
        assert_eq!(green, super::GREEN_GRADIENT);
        assert_eq!(orange, super::ORANGE_GRADIENT);
        assert_eq!(signature, super::FINAL_SIGNATURE);
    }
}
