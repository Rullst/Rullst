// src/generators/desktop/mod.rs — Omni desktop & mobile packaging public API.

mod android_release;
mod runner;
mod scaffold;
mod signing;

pub use android_release::{
    AndroidReleaseEvidence, AndroidReleaseOptions, build_android_release,
    build_android_release_with_options,
};
pub(crate) use android_release::{command as release_command, run as run_release};
pub use runner::run_omni_app;
pub use scaffold::{OmniScaffoldOptions, scaffold_omni_system, scaffold_omni_system_with_options};
