// src/generators/desktop/mod.rs — Omni desktop & mobile packaging public API.

mod runner;
mod scaffold;
mod signing;

pub use runner::run_omni_app;
pub use scaffold::{OmniScaffoldOptions, scaffold_omni_system, scaffold_omni_system_with_options};
pub use signing::build_android_release;
