// src/ui/spinner.rs — Neon animated terminal spinner.

use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

pub fn with_spinner<F, T>(msg: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    let msg = msg.to_string();
    let is_running = Arc::new(AtomicBool::new(true));
    let is_running_clone = is_running.clone();

    let handle = thread::spawn(move || {
        let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let mut i = 0;
        let colors = [
            colored::Color::Cyan,
            colored::Color::Magenta,
            colored::Color::BrightCyan,
            colored::Color::Blue,
        ];

        let re = regex::Regex::new(r"Application|migrations|Omni").unwrap();

        while is_running_clone.load(Ordering::SeqCst) {
            use colored::Colorize;
            let frame = frames[i % frames.len()];
            let color = colors[(i / 2) % colors.len()];

            let mut animated_msg = String::new();

            let mut found_target = None;
            if let Some(mat) = re.find(&msg) {
                found_target = Some((mat.as_str(), mat.start()));
            }

            if let Some((target, pos)) = found_target {
                animated_msg.push_str(&msg[..pos].bold().to_string());

                let custom_colors = if target == "Omni" {
                    vec![
                        colored::Color::Red,
                        colored::Color::TrueColor {
                            r: 255,
                            g: 165,
                            b: 0,
                        },
                        colored::Color::BrightRed,
                        colored::Color::Yellow,
                    ]
                } else {
                    colors.to_vec()
                };

                for (j, ch) in target.chars().enumerate() {
                    let is_upper = ((i + j) % 4) < 2;
                    let wave_char = if is_upper {
                        ch.to_ascii_uppercase()
                    } else {
                        ch.to_ascii_lowercase()
                    };
                    let c_idx = (i + j) % custom_colors.len();
                    animated_msg.push_str(
                        &wave_char
                            .to_string()
                            .color(custom_colors[c_idx])
                            .bold()
                            .to_string(),
                    );
                }
                animated_msg.push_str(&msg[pos + target.len()..].bold().to_string());
            } else {
                animated_msg = msg.bold().to_string();
            }

            print!("\r\x1B[K{} {}", frame.color(color).bold(), animated_msg);
            let _ = std::io::stdout().flush();

            thread::sleep(Duration::from_millis(80));
            i += 1;
        }
        print!("\r\x1B[K");
        let _ = std::io::stdout().flush();
    });

    let result = f();

    is_running.store(false, Ordering::SeqCst);
    let _ = handle.join();

    result
}
