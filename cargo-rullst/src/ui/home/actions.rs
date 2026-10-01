//! Home menu entries as data. Inside a project the quick actions come first;
//! outside one, project creation leads. Every entry maps to an existing CLI
//! command or submenu.

use super::Home;
use colored::Colorize;

/// What a home entry does when chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::ui) enum HomeAction {
    /// Runs `<program> <command>` directly.
    Command(&'static str),
    Scaffold,
    Database,
    Deploy,
    /// The complete project-operations submenu.
    ProjectOperations,
    /// The fuzzy palette over every command.
    Palette,
    NewProject,
    Help,
    Exit,
}

/// One menu row: an aligned title, a dimmed hint and its action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::ui) struct HomeEntry {
    pub title: &'static str,
    pub hint: &'static str,
    pub action: HomeAction,
}

impl HomeEntry {
    const fn new(title: &'static str, hint: &'static str, action: HomeAction) -> Self {
        Self {
            title,
            hint,
            action,
        }
    }

    pub(in crate::ui) fn label(&self) -> String {
        format!("{}{}", self.title, self.hint.dimmed())
    }
}

const PROJECT_ENTRIES: [HomeEntry; 11] = [
    HomeEntry::new(
        "🚀  Start Dev Server         ",
        "(cargo rullst dev · hot reload)",
        HomeAction::Command("dev"),
    ),
    HomeEntry::new(
        "📺  Live Dev Dashboard       ",
        "(cargo rullst dash)",
        HomeAction::Command("dash"),
    ),
    HomeEntry::new(
        "🧰  Scaffold Code            ",
        "(make:controller, make:model, LiveView, gRPC...)",
        HomeAction::Scaffold,
    ),
    HomeEntry::new(
        "💾  Database                 ",
        "(Migrate, Rollback, Status, Seed, Studio)",
        HomeAction::Database,
    ),
    HomeEntry::new(
        "🩺  Doctor                   ",
        "(cargo rullst doctor · toolchain health)",
        HomeAction::Command("doctor"),
    ),
    HomeEntry::new(
        "🚢  Deploy                   ",
        "(Guided PaaS or Foundry SSH pipeline)",
        HomeAction::Deploy,
    ),
    HomeEntry::new(
        "📁  All Project Operations   ",
        "(Auth, Desktop, Docker, Nix, Upgrade...)",
        HomeAction::ProjectOperations,
    ),
    HomeEntry::new(
        "✨  Create New Project       ",
        "(cargo rullst new)",
        HomeAction::NewProject,
    ),
    HomeEntry::new(
        "🔎  Search All Commands      ",
        "(type to filter every command)",
        HomeAction::Palette,
    ),
    HomeEntry::new(
        "💡  View Help & Commands     ",
        "(Framework Reference)",
        HomeAction::Help,
    ),
    HomeEntry::new(
        "❌  Exit                     ",
        "(Close interactive menu)",
        HomeAction::Exit,
    ),
];

const OUTSIDE_ENTRIES: [HomeEntry; 5] = [
    HomeEntry::new(
        "✨  Create New Project       ",
        "(Blank/API, Blog, SaaS, LMS, Portfolio or ERP)",
        HomeAction::NewProject,
    ),
    HomeEntry::new(
        "📁  Already have a project?  ",
        "(Dev, Scaffold, DB, Auth, Deploy...)",
        HomeAction::ProjectOperations,
    ),
    HomeEntry::new(
        "🔎  Search All Commands      ",
        "(type to filter every command)",
        HomeAction::Palette,
    ),
    HomeEntry::new(
        "💡  View Help & Commands     ",
        "(Framework Reference)",
        HomeAction::Help,
    ),
    HomeEntry::new(
        "❌  Exit                     ",
        "(Close interactive menu)",
        HomeAction::Exit,
    ),
];

/// The home menu for `home`, in display order.
pub(in crate::ui) fn home_entries(home: &Home) -> &'static [HomeEntry] {
    match home {
        Home::Project(_) => &PROJECT_ENTRIES,
        Home::Outside => &OUTSIDE_ENTRIES,
    }
}
