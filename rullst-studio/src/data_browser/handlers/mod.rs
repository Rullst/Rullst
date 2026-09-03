//! HTTP handler sub-module registry for the Studio data browser.

mod ai;
mod dashboard;
mod migrations;
mod mutations;
mod security;
mod table;
mod telemetry;

// ─── Public Re-exports ──────────────────────────────────────────────────────

pub use ai::handle_studio_tools_ai;
pub use dashboard::handle_dashboard;
pub use migrations::handle_studio_tools_migrations;
pub(crate) use mutations::{handle_table_delete, handle_table_update};
pub use security::handle_studio_tools_security;
pub use table::handle_table;
pub use telemetry::{
    handle_studio_capital, handle_studio_radar, handle_studio_traces,
    handle_studio_traces_with_store,
};
