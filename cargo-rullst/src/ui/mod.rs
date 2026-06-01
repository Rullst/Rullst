// src/ui/mod.rs — Visual layer of the Rullst CLI.
// Everything terminal-related lives here: banners, spinners, dashboards, menus.

pub mod components;

pub use components::{
    check_update_available,
    print_update_banner,
    show_help_reference,
    show_interactive_dashboard,
    trigger_background_update_check,
    with_spinner,
};
