//! Palette state and frame rendering, independent of the terminal so key
//! handling and layout are unit tested.

use super::PaletteEntry;
use crate::ui::style::{self, Style};

/// A key, reduced to what the palette understands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Input {
    Char(char),
    Backspace,
    ClearQuery,
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
    Enter,
    Escape,
    Interrupt,
    Ignore,
}

/// What the caller should do after a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Continue,
    /// Run the selected match.
    Run,
    /// Leave the palette without running anything.
    Back,
    /// Ctrl+C.
    Interrupt,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PaletteState {
    pub query: String,
    /// Index into the current matches.
    pub selected: usize,
    /// First visible match.
    pub offset: usize,
}

impl PaletteState {
    /// Applies `input` given `matches` results and `page` visible rows.
    pub(crate) fn handle(&mut self, input: Input, matches: usize, page: usize) -> Outcome {
        let page = page.max(1);
        let last = matches.saturating_sub(1);
        match input {
            Input::Char(character) => {
                if !character.is_control() && self.query.chars().count() < super::score::QUERY_LIMIT
                {
                    self.query.push(character);
                    self.selected = 0;
                    self.offset = 0;
                }
            }
            Input::Backspace => {
                if self.query.pop().is_some() {
                    self.selected = 0;
                    self.offset = 0;
                }
            }
            Input::ClearQuery => {
                self.query.clear();
                self.selected = 0;
                self.offset = 0;
            }
            Input::Up if matches > 0 => {
                self.selected = if self.selected == 0 {
                    last
                } else {
                    self.selected - 1
                };
            }
            Input::Down if matches > 0 => {
                self.selected = if self.selected >= last {
                    0
                } else {
                    self.selected + 1
                };
            }
            Input::PageUp => self.selected = self.selected.saturating_sub(page),
            Input::PageDown => self.selected = (self.selected + page).min(last),
            Input::Home => self.selected = 0,
            Input::End => self.selected = last,
            Input::Enter if matches > 0 => return Outcome::Run,
            Input::Escape => return Outcome::Back,
            Input::Interrupt => return Outcome::Interrupt,
            _ => {}
        }
        self.scroll(matches, page);
        Outcome::Continue
    }

    /// Keeps the selection inside the match list and the visible window.
    pub(crate) fn scroll(&mut self, matches: usize, page: usize) {
        let page = page.max(1);
        self.selected = self.selected.min(matches.saturating_sub(1));
        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset + page {
            self.offset = self.selected + 1 - page;
        }
        self.offset = self.offset.min(matches.saturating_sub(page));
    }
}

/// `text` cut to `width` characters, with an ellipsis when cut.
fn fit(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    let mut cut: String = text.chars().take(width.saturating_sub(1)).collect();
    cut.push('…');
    cut
}

/// The lines of one frame: the query, up to `rows` matches and a footer.
/// Every line fits in `width` columns so redrawing never wraps.
pub(crate) fn frame(
    state: &PaletteState,
    entries: &[PaletteEntry],
    matches: &[usize],
    rows: usize,
    width: usize,
    style: Style,
) -> Vec<String> {
    let width = width.max(20) - 1;
    let caret = if style.is_plain() { "_" } else { "▌" };
    let query = fit(&state.query, width.saturating_sub(22));
    let mut lines = vec![format!(
        "  {} {}{}",
        style.bold("Search commands ›", style::BRIGHT),
        style.paint(&query, style::BRIGHT),
        style.paint(caret, style::ACCENT)
    )];
    let name_width = entries
        .iter()
        .map(|entry| entry.name.chars().count())
        .max()
        .unwrap_or(0)
        .min(28)
        + 2;
    if matches.is_empty() {
        lines.push(format!(
            "  {}",
            style.paint(
                &fit(
                    &format!("No command matches \"{}\"", state.query),
                    width - 2
                ),
                style::MUTED
            )
        ));
    }
    for (position, index) in matches.iter().enumerate().skip(state.offset).take(rows) {
        let Some(entry) = entries.get(*index) else {
            continue;
        };
        let selected = position == state.selected;
        let name = fit(&entry.name, name_width - 2);
        let about = fit(&entry.about, width.saturating_sub(name_width + 4));
        let marker = if selected { "❯" } else { " " };
        let name = format!("{name:<name_width$}");
        lines.push(if selected {
            format!(
                "  {} {}{}",
                style.bold(marker, style::PASS),
                style.bold(&name, style::BRIGHT),
                style.paint(&about, style::BRIGHT)
            )
        } else {
            format!(
                "  {} {}{}",
                marker,
                style.paint(&name, style::BRIGHT),
                style.paint(&about, style::MUTED)
            )
        });
    }
    let footer = format!(
        "↑↓ move · Enter run · Esc back · {} of {} commands",
        matches.len(),
        entries.len()
    );
    lines.push(format!(
        "  {}",
        style.paint(&fit(&footer, width - 2), style::MUTED)
    ));
    lines
}
