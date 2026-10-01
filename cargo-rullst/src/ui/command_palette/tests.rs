#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use super::*;

fn find<'a>(entries: &'a [PaletteEntry], name: &str) -> &'a PaletteEntry {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .unwrap_or_else(|| panic!("missing palette entry {name}"))
}

#[test]
fn entries_cover_every_runtime_command_with_their_required_arguments() {
    let entries = entries(&crate::command());
    for name in [
        "new",
        "make:model",
        "doctor",
        "completions",
        "info",
        "deploy:doctor",
        "make:privacy",
        "update check",
        "update project prepare",
    ] {
        find(&entries, name);
    }
    for absent in ["help", "update", "update project"] {
        assert!(entries.iter().all(|entry| entry.name != absent), "{absent}");
    }

    let model = find(&entries, "make:model");
    assert_eq!(model.path, ["make:model"]);
    assert_eq!(model.required.len(), 1);
    assert_eq!(model.required[0].long, None);
    assert!(model.required[0].prompt.contains("Name of the Model"));

    let models = find(&entries, "generate:models");
    let longs: Vec<_> = models
        .required
        .iter()
        .map(|argument| argument.long.as_deref())
        .collect();
    assert_eq!(longs, [Some("--driver"), Some("--url")]);
    assert!(models.aliases.contains(&"make:models-from-db".to_string()));

    let completions = find(&entries, "completions");
    assert_eq!(
        completions.required[0].choices,
        ["bash", "elvish", "fish", "powershell", "zsh"]
    );
    assert!(find(&entries, "new").required.is_empty());
    assert!(
        entries.windows(2).all(|pair| pair[0].name <= pair[1].name),
        "entries are sorted"
    );
}

#[test]
fn typing_filters_and_ranks_the_runtime_commands() {
    let entries = entries(&crate::command());
    let first = |query: &str| entries[matches(query, &entries)[0]].name.clone();
    assert_eq!(first("dev"), "dev");
    assert_eq!(first("doctor"), "doctor");
    assert_eq!(first("mig"), "db:migrate");
    assert_eq!(first("completions"), "completions");
    assert_eq!(first("update ch"), "update check");
    assert_eq!(matches("", &entries).len(), entries.len());
    assert!(matches("zzzzqqqq", &entries).is_empty());
}

#[test]
fn keys_edit_the_query_and_move_a_wrapping_selection() {
    let mut state = PaletteState::default();
    for character in "mig".chars() {
        assert_eq!(
            state.handle(Input::Char(character), 10, 4),
            Outcome::Continue
        );
    }
    assert_eq!(state.query, "mig");
    state.handle(Input::Up, 10, 4);
    assert_eq!(
        (state.selected, state.offset),
        (9, 6),
        "wraps to the last match"
    );
    state.handle(Input::Down, 10, 4);
    assert_eq!((state.selected, state.offset), (0, 0), "wraps to the first");
    state.handle(Input::PageDown, 10, 4);
    assert_eq!((state.selected, state.offset), (4, 1));
    state.handle(Input::End, 10, 4);
    assert_eq!(state.selected, 9);
    state.handle(Input::Home, 10, 4);
    assert_eq!(state.selected, 0);
    state.handle(Input::Backspace, 10, 4);
    assert_eq!(state.query, "mi");
    state.handle(Input::ClearQuery, 10, 4);
    assert_eq!(state.query, "");
    state.handle(Input::Char('\u{7}'), 10, 4);
    assert_eq!(state.query, "", "control characters are ignored");

    assert_eq!(state.handle(Input::Enter, 0, 4), Outcome::Continue);
    assert_eq!(state.handle(Input::Enter, 3, 4), Outcome::Run);
    assert_eq!(state.handle(Input::Escape, 3, 4), Outcome::Back);
    assert_eq!(state.handle(Input::Interrupt, 3, 4), Outcome::Interrupt);
    for _ in 0..200 {
        state.handle(Input::Char('x'), 0, 4);
    }
    assert_eq!(state.query.chars().count(), score::QUERY_LIMIT);
}

#[test]
fn crossterm_keys_map_to_palette_inputs() {
    let key = |code, modifiers| KeyEvent::new(code, modifiers);
    assert_eq!(
        input_for(key(KeyCode::Char('c'), KeyModifiers::CONTROL)),
        Input::Interrupt
    );
    assert_eq!(
        input_for(key(KeyCode::Char('m'), KeyModifiers::NONE)),
        Input::Char('m')
    );
    assert_eq!(
        input_for(key(KeyCode::Char('M'), KeyModifiers::SHIFT)),
        Input::Char('M')
    );
    assert_eq!(
        input_for(key(KeyCode::Esc, KeyModifiers::NONE)),
        Input::Escape
    );
    assert_eq!(
        input_for(key(KeyCode::Tab, KeyModifiers::NONE)),
        Input::Down
    );
    assert_eq!(
        input_for(key(KeyCode::Char('x'), KeyModifiers::CONTROL)),
        Input::Ignore
    );
}

#[test]
fn frames_fit_the_terminal_and_mark_the_selection() {
    let entries = vec![
        PaletteEntry {
            path: vec!["db:migrate".into()],
            name: "db:migrate".into(),
            about: "Runs pending database migrations".into(),
            aliases: Vec::new(),
            required: Vec::new(),
        },
        PaletteEntry {
            path: vec!["dev".into()],
            name: "dev".into(),
            about: "Starts the Rullst development server with a very long description".into(),
            aliases: Vec::new(),
            required: Vec::new(),
        },
    ];
    let state = PaletteState {
        query: "d".into(),
        selected: 1,
        offset: 0,
    };
    let lines = frame(&state, &entries, &[0, 1], 5, 60, Style::PLAIN);
    assert_eq!(
        lines,
        [
            "  Search commands › d_",
            "    db:migrate  Runs pending database migrations",
            "  ❯ dev         Starts the Rullst development server with …",
            "  ↑↓ move · Enter run · Esc back · 2 of 2 commands",
        ]
    );
    for line in &lines {
        assert!(line.chars().count() < 60, "{line}");
    }
    let empty = frame(&PaletteState::default(), &entries, &[], 5, 50, Style::PLAIN);
    assert_eq!(empty[1], "  No command matches \"\"");
}
