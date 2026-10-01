//! Interactive choice screens shared by `cargo rullst new` and `cargo rullst
//! tour`: a gradient title, body lines, a list whose highlighted entry can
//! show a detail panel (for example a file tree) and key hints.
//!
//! Layout ([`frame`]) and key handling ([`handle_key`]) are pure and unit
//! tested; only [`Terminal::choose`] in [`console`] touches the terminal.

use super::palette::Rgb;

mod console;
pub(crate) use console::Terminal;

const LABEL: Rgb = (150, 155, 175);
const VALUE: Rgb = (240, 240, 248);
const MUTED: Rgb = (110, 110, 130);

/// The semantic colour of a [`Span`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tone {
    /// Uncoloured text.
    Plain,
    /// The bold blue → green → orange brand gradient.
    Brand,
    /// A field name in a summary.
    Label,
    /// A field value.
    Value,
    /// A bold value.
    Strong,
    /// The highlighted entry and confirmations (brand green, bold).
    Accent,
    /// Hints and secondary text.
    Muted,
    /// Directory names in a file tree (brand blue).
    Directory,
    /// Something that needs attention (brand orange).
    Warning,
}

/// A run of text in one tone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Span {
    pub(crate) text: String,
    pub(crate) tone: Tone,
}

/// One terminal line made of coloured spans.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Line {
    pub(crate) spans: Vec<Span>,
}

impl Line {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Appends `text` in `tone`.
    pub(crate) fn push(mut self, tone: Tone, text: impl Into<String>) -> Self {
        let text = text.into();
        if !text.is_empty() {
            self.spans.push(Span { text, tone });
        }
        self
    }

    /// The line without colour.
    pub(crate) fn text(&self) -> String {
        self.spans.iter().map(|span| span.text.as_str()).collect()
    }

    fn width(&self) -> usize {
        self.spans
            .iter()
            .map(|span| span.text.chars().count())
            .sum()
    }

    /// Cuts the line to `width` characters, ending with `…` when cut.
    fn truncated(&self, width: usize) -> Self {
        if self.width() <= width {
            return self.clone();
        }
        let mut budget = width.saturating_sub(1);
        let mut line = Self::new();
        for span in &self.spans {
            if budget == 0 {
                break;
            }
            let text: String = span.text.chars().take(budget).collect();
            budget -= text.chars().count();
            line = line.push(span.tone, text);
        }
        line.push(Tone::Muted, "…")
    }
}

/// One selectable entry.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Choice {
    pub(crate) label: String,
    pub(crate) hint: String,
    /// Shown below the list while this entry is highlighted.
    pub(crate) detail: Vec<Line>,
}

impl Choice {
    pub(crate) fn new(label: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            hint: hint.into(),
            detail: Vec::new(),
        }
    }
}

/// Single choice (Enter picks the highlighted entry) or checkboxes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Selection {
    One { initial: usize },
    Many { checked: Vec<bool> },
}

/// Everything one interactive screen shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Screen {
    pub(crate) title: String,
    /// Progress or context shown after the title, e.g. `Step 2 of 5 · Blueprint`.
    pub(crate) crumb: String,
    pub(crate) body: Vec<Line>,
    pub(crate) question: String,
    pub(crate) choices: Vec<Choice>,
    pub(crate) selection: Selection,
    /// Esc/Backspace/← answer [`Answer::Back`].
    pub(crate) can_go_back: bool,
    /// After an answer, print `✔ <echo> · <answer>`; `None` prints nothing.
    pub(crate) echo: Option<String>,
}

/// What the user decided on a [`Screen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Answer {
    One(usize),
    Many(Vec<usize>),
    Back,
}

/// A key press, independent of the terminal library.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Key {
    Up,
    Down,
    Home,
    End,
    Enter,
    Space,
    Back,
    Interrupt,
    Digit(u8),
    Redraw,
    Other,
}

/// The cursor and checkbox state of a screen being answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Cursor {
    pub(crate) index: usize,
    pub(crate) checked: Vec<bool>,
}

impl Cursor {
    pub(crate) fn new(screen: &Screen) -> Self {
        let last = screen.choices.len().saturating_sub(1);
        match &screen.selection {
            Selection::One { initial } => Self {
                index: (*initial).min(last),
                checked: Vec::new(),
            },
            Selection::Many { checked } => Self {
                index: 0,
                checked: (0..screen.choices.len())
                    .map(|index| checked.get(index).copied().unwrap_or(false))
                    .collect(),
            },
        }
    }
}

/// What a key does to a screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Continue,
    Done(Answer),
    Interrupted,
}

/// Applies `key` to `cursor`. Pure, so every binding is unit tested.
pub(crate) fn handle_key(screen: &Screen, cursor: &mut Cursor, key: Key) -> Outcome {
    let count = screen.choices.len();
    let many = matches!(screen.selection, Selection::Many { .. });
    match key {
        Key::Interrupt => return Outcome::Interrupted,
        Key::Back if screen.can_go_back => return Outcome::Done(Answer::Back),
        Key::Up if count > 0 => cursor.index = (cursor.index + count - 1) % count,
        Key::Down if count > 0 => cursor.index = (cursor.index + 1) % count,
        Key::Home => cursor.index = 0,
        Key::End => cursor.index = count.saturating_sub(1),
        Key::Digit(digit) => {
            let target = usize::from(digit).saturating_sub(1);
            if digit > 0 && target < count {
                cursor.index = target;
            }
        }
        Key::Space if many => {
            if let Some(checked) = cursor.checked.get_mut(cursor.index) {
                *checked = !*checked;
            }
        }
        Key::Enter if many => {
            let picked = (0..count)
                .filter(|index| cursor.checked.get(*index).copied().unwrap_or(false))
                .collect();
            return Outcome::Done(Answer::Many(picked));
        }
        Key::Enter if count > 0 => return Outcome::Done(Answer::One(cursor.index)),
        _ => {}
    }
    Outcome::Continue
}

fn footer(screen: &Screen) -> String {
    let mut hints = vec!["↑↓ move"];
    if matches!(screen.selection, Selection::Many { .. }) {
        hints.push("Space toggle");
        hints.push("Enter confirm");
    } else {
        hints.push("Enter select");
    }
    if screen.can_go_back {
        hints.push("Esc back");
    }
    hints.push("Ctrl+C quit");
    hints.join(" · ")
}

/// The lines of `screen` for a `width` × `height` terminal. The detail panel
/// shrinks first, then the body, so the frame never scrolls the terminal.
pub(crate) fn frame(screen: &Screen, cursor: &Cursor, width: u16, height: u16) -> Vec<Line> {
    let width = usize::from(width.max(20)) - 1;
    let budget = usize::from(height.max(8)) - 1;
    let many = matches!(screen.selection, Selection::Many { .. });

    let mut head = vec![
        Line::new()
            .push(Tone::Accent, "◆ ")
            .push(Tone::Brand, screen.title.clone())
            .push(
                Tone::Muted,
                format!("  {}", screen.crumb).trim_end().to_string(),
            ),
        Line::new(),
    ];
    let mut body: Vec<Line> = screen
        .body
        .iter()
        .map(|line| {
            let mut indented = Line::new().push(Tone::Plain, "  ");
            indented.spans.extend(line.spans.iter().cloned());
            indented
        })
        .collect();
    if !body.is_empty() {
        body.push(Line::new());
    }

    let label_width = screen
        .choices
        .iter()
        .map(|choice| choice.label.chars().count())
        .max()
        .unwrap_or(0);
    let mut list = Vec::new();
    if !screen.question.is_empty() {
        list.push(
            Line::new()
                .push(Tone::Plain, "  ")
                .push(Tone::Strong, screen.question.clone()),
        );
    }
    for (index, choice) in screen.choices.iter().enumerate() {
        let active = index == cursor.index;
        let mut line = Line::new().push(Tone::Accent, if active { "  ❯ " } else { "    " });
        if many {
            let mark = if cursor.checked.get(index).copied().unwrap_or(false) {
                "[x] "
            } else {
                "[ ] "
            };
            line = line.push(if active { Tone::Accent } else { Tone::Muted }, mark);
        }
        let label = format!("{:<label_width$}", choice.label);
        line = line.push(if active { Tone::Accent } else { Tone::Value }, label);
        if !choice.hint.is_empty() {
            line = line.push(Tone::Muted, format!("  {}", choice.hint));
        }
        list.push(line);
    }

    let tail = vec![
        Line::new(),
        Line::new()
            .push(Tone::Plain, "  ")
            .push(Tone::Muted, footer(screen)),
    ];

    let fixed = head.len() + list.len() + tail.len();
    let room = budget.saturating_sub(fixed);
    if body.len() > room {
        let cut = body.len() - room;
        body.drain(..cut);
    }
    let room = room.saturating_sub(body.len());
    let mut detail: Vec<Line> = screen
        .choices
        .get(cursor.index)
        .map(|choice| choice.detail.as_slice())
        .unwrap_or_default()
        .iter()
        .map(|line| {
            let mut indented = Line::new().push(Tone::Plain, "    ");
            indented.spans.extend(line.spans.iter().cloned());
            indented
        })
        .collect();
    if !detail.is_empty() {
        detail.insert(0, Line::new());
        if detail.len() > room {
            detail.truncate(room.saturating_sub(1));
            if room > 1 {
                detail.push(Line::new().push(Tone::Muted, "    …"));
            }
        }
    }

    head.append(&mut body);
    head.append(&mut list);
    head.append(&mut detail);
    head.extend(tail);
    head.iter().map(|line| line.truncated(width)).collect()
}

/// The `✔ <echo> · <answer>` line printed after a screen is answered.
pub(crate) fn echo_line(screen: &Screen, answer: &Answer) -> Option<Line> {
    let echo = screen.echo.as_ref()?;
    let label = |index: &usize| screen.choices.get(*index).map(|choice| choice.label.trim());
    let value = match answer {
        Answer::Back => return None,
        Answer::One(index) => label(index).unwrap_or_default().to_string(),
        Answer::Many(indices) if indices.is_empty() => "none".to_string(),
        Answer::Many(indices) => indices
            .iter()
            .filter_map(label)
            .collect::<Vec<_>>()
            .join(", "),
    };
    Some(
        Line::new()
            .push(Tone::Accent, "✔ ")
            .push(Tone::Strong, echo.clone())
            .push(Tone::Muted, " · ")
            .push(Tone::Value, value),
    )
}

#[cfg(test)]
#[path = "screen_tests.rs"]
mod tests;
