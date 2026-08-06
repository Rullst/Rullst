// src/ui/components.rs — Re-exports all UI components for backwards compatibility.
// Actual implementations live in the sub-modules below.

pub use super::dashboard::{execute_command, show_interactive_dashboard};
pub use super::help::{get_help_groups, show_help_reference};
pub use super::spinner::with_spinner;
pub use super::update_check::{
    check_update_available, print_update_banner, trigger_background_update_check,
};
