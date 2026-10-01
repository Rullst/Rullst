// src/ui/mod.rs — Visual layer of the Rullst CLI.
// Everything terminal-related lives here: banners, spinners, dashboards, menus.

pub mod components;
pub mod dash_tui;
pub mod dashboard;
mod dashboard_brand;
pub mod help;
mod home;
mod opening_marker;
mod palette;
pub(crate) mod screen;
pub mod spinner;
mod terminal;
pub mod update_check;

pub use dashboard::{execute_command, show_interactive_dashboard};
pub use help::{get_help_groups, show_help_reference};
pub use spinner::with_spinner;
pub use update_check::{
    check_update_available, print_update_banner, trigger_background_update_check,
};
