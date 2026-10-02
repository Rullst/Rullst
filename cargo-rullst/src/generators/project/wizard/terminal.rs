//! The real terminal behind the wizard: a line editor for the name and the
//! shared choice screens for everything else.

use super::WizardResult;
use super::flow::WizardUi;
use crate::ui::screen::{Answer, Line, Screen, Terminal, Tone};

pub(crate) struct TerminalUi {
    terminal: Terminal,
    theme: dialoguer::theme::ColorfulTheme,
}

impl TerminalUi {
    pub(crate) fn new(terminal: Terminal) -> Self {
        Self {
            terminal,
            theme: dialoguer::theme::ColorfulTheme::default(),
        }
    }
}

impl WizardUi for TerminalUi {
    fn ask_name(&mut self, initial: &str) -> WizardResult<String> {
        let mut input = dialoguer::Input::<String>::with_theme(&self.theme)
            .with_prompt("Project name (letters, digits, _ or -)");
        if !initial.is_empty() {
            input = input.with_initial_text(initial);
        }
        Ok(input.interact_text()?)
    }

    fn reject_name(&mut self, reason: &str) {
        // A failed hint must not abort the wizard; the prompt repeats anyway.
        let _ = self
            .terminal
            .print(&[Line::new().push(Tone::Warning, format!("  {reason}"))]);
    }

    fn choose(&mut self, screen: &Screen) -> WizardResult<Answer> {
        Ok(self.terminal.choose(screen)?)
    }
}
