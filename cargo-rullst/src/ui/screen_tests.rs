use super::console::{clear, paint};
use super::*;
use crate::ui::palette::ColorDepth;

fn screen(selection: Selection) -> Screen {
    Screen {
        title: "New Rullst app".to_string(),
        crumb: "Step 2 of 4 · Blueprint".to_string(),
        body: vec![Line::new().push(Tone::Label, "Body line")],
        question: "Which starter?".to_string(),
        choices: vec![
            Choice {
                detail: vec![
                    Line::new().push(Tone::Directory, "app/"),
                    Line::new().push(Tone::Muted, "└── Cargo.toml"),
                ],
                ..Choice::new("Blank", "minimal page")
            },
            Choice::new("SaaS", "billing"),
            Choice::new("ERP Pocket", ""),
        ],
        selection,
        can_go_back: true,
        echo: Some("Blueprint".to_string()),
    }
}

fn texts(lines: &[Line]) -> Vec<String> {
    lines.iter().map(Line::text).collect()
}

#[test]
fn frame_lays_out_title_body_aligned_choices_detail_and_hints() {
    let single = screen(Selection::One { initial: 0 });
    let lines = texts(&frame(&single, &Cursor::new(&single), 100, 40));
    assert_eq!(
        lines,
        [
            "◆ New Rullst app  Step 2 of 4 · Blueprint",
            "",
            "  Body line",
            "",
            "  Which starter?",
            "  ❯ Blank       minimal page",
            "    SaaS        billing",
            "    ERP Pocket",
            "",
            "    app/",
            "    └── Cargo.toml",
            "",
            "  ↑↓ move · Enter select · Esc back · Ctrl+C quit",
        ]
    );

    let mut cursor = Cursor::new(&single);
    cursor.index = 1;
    let moved = texts(&frame(&single, &cursor, 100, 40));
    assert!(moved.contains(&"  ❯ SaaS        billing".to_string()));
    assert!(!moved.iter().any(|line| line.contains("Cargo.toml")));
}

#[test]
fn checkboxes_show_state_and_their_own_hints() {
    let many = screen(Selection::Many {
        checked: vec![false, true],
    });
    let lines = texts(&frame(&many, &Cursor::new(&many), 100, 40));
    assert!(lines.contains(&"  ❯ [ ] Blank       minimal page".to_string()));
    assert!(lines.contains(&"    [x] SaaS        billing".to_string()));
    assert!(lines.contains(&"    [ ] ERP Pocket".to_string()));
    assert_eq!(
        lines.last().map(String::as_str),
        Some("  ↑↓ move · Space toggle · Enter confirm · Esc back · Ctrl+C quit")
    );
}

#[test]
fn small_terminals_shrink_the_detail_then_the_body_and_cut_wide_lines() {
    let mut tall = screen(Selection::One { initial: 0 });
    tall.choices[0].detail = (0..30)
        .map(|index| Line::new().push(Tone::Muted, format!("file {index}")))
        .collect();
    let lines = frame(&tall, &Cursor::new(&tall), 100, 16);
    assert_eq!(lines.len(), 15, "{:#?}", texts(&lines));
    let text = texts(&lines);
    assert!(text.contains(&"    …".to_string()));
    assert!(text.contains(&"  ❯ Blank       minimal page".to_string()));

    let tiny = frame(&tall, &Cursor::new(&tall), 100, 8);
    assert!(tiny.len() <= 9, "{:#?}", texts(&tiny));
    assert!(!texts(&tiny).contains(&"  Body line".to_string()));

    let narrow = frame(&tall, &Cursor::new(&tall), 24, 40);
    for line in &narrow {
        assert!(line.text().chars().count() <= 23, "{}", line.text());
    }
    assert!(texts(&narrow).iter().any(|line| line.ends_with('…')));
}

#[test]
fn keys_move_wrap_jump_toggle_and_answer() {
    let single = screen(Selection::One { initial: 9 });
    let mut cursor = Cursor::new(&single);
    assert_eq!(cursor.index, 2, "initial clamps to the last entry");
    assert_eq!(
        handle_key(&single, &mut cursor, Key::Down),
        Outcome::Continue
    );
    assert_eq!(cursor.index, 0, "down wraps");
    handle_key(&single, &mut cursor, Key::Up);
    assert_eq!(cursor.index, 2, "up wraps");
    handle_key(&single, &mut cursor, Key::Digit(2));
    assert_eq!(cursor.index, 1);
    handle_key(&single, &mut cursor, Key::Digit(0));
    handle_key(&single, &mut cursor, Key::Digit(9));
    assert_eq!(cursor.index, 1, "out-of-range digits are ignored");
    handle_key(&single, &mut cursor, Key::Home);
    assert_eq!(cursor.index, 0);
    handle_key(&single, &mut cursor, Key::End);
    assert_eq!(cursor.index, 2);
    assert_eq!(
        handle_key(&single, &mut cursor, Key::Space),
        Outcome::Continue
    );
    assert_eq!(
        handle_key(&single, &mut cursor, Key::Enter),
        Outcome::Done(Answer::One(2))
    );
    assert_eq!(
        handle_key(&single, &mut cursor, Key::Back),
        Outcome::Done(Answer::Back)
    );
    assert_eq!(
        handle_key(&single, &mut cursor, Key::Interrupt),
        Outcome::Interrupted
    );

    let first = Screen {
        can_go_back: false,
        ..single
    };
    assert_eq!(
        handle_key(&first, &mut cursor, Key::Back),
        Outcome::Continue
    );

    let many = screen(Selection::Many {
        checked: vec![true],
    });
    let mut cursor = Cursor::new(&many);
    assert_eq!(cursor.checked, [true, false, false]);
    handle_key(&many, &mut cursor, Key::Space);
    handle_key(&many, &mut cursor, Key::End);
    handle_key(&many, &mut cursor, Key::Space);
    assert_eq!(
        handle_key(&many, &mut cursor, Key::Enter),
        Outcome::Done(Answer::Many(vec![2]))
    );
}

#[test]
fn answers_echo_their_labels_and_back_echoes_nothing() {
    let single = screen(Selection::One { initial: 0 });
    let echo = |answer| echo_line(&single, &answer).map(|line| line.text());
    assert_eq!(echo(Answer::One(1)).as_deref(), Some("✔ Blueprint · SaaS"));
    assert_eq!(
        echo(Answer::Many(vec![0, 2])).as_deref(),
        Some("✔ Blueprint · Blank, ERP Pocket")
    );
    assert_eq!(
        echo(Answer::Many(Vec::new())).as_deref(),
        Some("✔ Blueprint · none")
    );
    assert_eq!(echo(Answer::Back), None);
    let silent = Screen {
        echo: None,
        ..single.clone()
    };
    assert_eq!(echo_line(&silent, &Answer::One(0)), None);
}

#[test]
fn painting_follows_the_colour_depth_and_brand_gradient() {
    let line = Line::new()
        .push(Tone::Brand, "RULLST")
        .push(Tone::Plain, " ")
        .push(Tone::Accent, "ok");
    assert_eq!(paint(&line, ColorDepth::None), "RULLST ok");
    let truecolor = paint(&line, ColorDepth::TrueColor);
    assert!(truecolor.starts_with("\x1b[1m\x1b[38;2;40;120;255mR"));
    assert!(truecolor.contains("\x1b[38;2;255;130;25mT"));
    assert!(truecolor.ends_with("\x1b[1m\x1b[38;2;30;205;110mok\x1b[0m"));
    let ansi = paint(&line, ColorDepth::Ansi256);
    assert!(ansi.contains("\x1b[38;5;33mR") && !ansi.contains("38;2;"));
}

#[test]
fn clearing_moves_to_the_first_line_of_the_previous_frame() {
    let mut out = Vec::new();
    clear(&mut out, 0).unwrap();
    assert!(out.is_empty());
    clear(&mut out, 1).unwrap();
    assert_eq!(out, b"\r\x1b[J");
    out.clear();
    clear(&mut out, 5).unwrap();
    assert_eq!(out, b"\r\x1b[4A\x1b[J");
}
