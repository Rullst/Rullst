// src/generators/desktop/mod.rs — Omni desktop & mobile packaging public API.

mod runner;
mod scaffold;

pub use runner::run_omni_app;
pub use scaffold::{OmniScaffoldOptions, scaffold_omni_system, scaffold_omni_system_with_options};
