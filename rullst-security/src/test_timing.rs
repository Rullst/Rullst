//! Wall-clock budgets for the linear-time regression tests.

use std::time::Duration;

/// Scales `budget` by `RULLST_TEST_TIME_SCALE` (1 to 100, default 1).
///
/// The sanitizer workflow sets the variable because instrumentation slows
/// every memory access; ordinary test runs keep the unscaled budget.
pub(crate) fn scaled(budget: Duration) -> Duration {
    let scale = std::env::var("RULLST_TEST_TIME_SCALE")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1)
        .clamp(1, 100);
    budget * scale
}
