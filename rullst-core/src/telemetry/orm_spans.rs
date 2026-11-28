//! ORM spans on demand in the subscriber that [`super::init_telemetry`]
//! installs.
//!
//! The `rullst.orm.*` spans (target `rullst_orm`) are created only once
//! something in the process records them: the development dashboard endpoint
//! or the production monitor's recording layer is mounted, or, in the
//! `telemetry` build, at start-up, because its OpenTelemetry layer exports
//! every span. Until then [`OrmSpansOnDemand`] disables their call sites, so
//! an ORM call creates no span, formats no span fields and is not re-entered
//! on every poll of SQLx's streams. An event logged during such a call is
//! written without the ORM span.
//!
//! The layer decides nothing else: events, other spans and the ORM spans
//! once recorded go through the `EnvFilter` (`RUST_LOG`, or
//! [`super::DEFAULT_LOG_FILTER`]) as before. A `RUST_LOG` with a span
//! directive (`target[span{field=value}]=level`) keeps every ORM span,
//! because such a directive can select events by the span they run in.

use std::sync::atomic::{AtomicBool, Ordering};
use tracing_core::subscriber::Interest;
use tracing_core::{Metadata, Subscriber};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::{Context, Layer};

/// Target of the ORM's spans and events.
const ORM_TARGET: &str = "rullst_orm";
/// Name prefix of the ORM's spans (`rullst.orm.query`,
/// `rullst.orm.transaction`, `rullst.orm.transaction.begin`). The call sites
/// of `tracing::enabled!` are named `enabled <file>:<line>`, so the check of
/// the development dashboard and the monitor still sees what `RUST_LOG`
/// keeps.
const ORM_SPAN_PREFIX: &str = "rullst.orm.";

/// Disables the ORM's span call sites until `recorded` is set.
pub(crate) struct OrmSpansOnDemand {
    /// `None` when `RUST_LOG` has a span directive: every ORM span is kept.
    recorded: Option<&'static AtomicBool>,
}

impl OrmSpansOnDemand {
    /// The layer for a subscriber filtered by `filter`.
    pub(crate) fn new(filter: &EnvFilter, recorded: &'static AtomicBool) -> Self {
        Self {
            recorded: (!has_span_directives(filter)).then_some(recorded),
        }
    }

    /// The flag that `metadata` waits for, if it is an ORM span.
    fn waits_for(&self, metadata: &Metadata<'_>) -> Option<&'static AtomicBool> {
        self.recorded.filter(|_| is_orm_span(metadata))
    }
}

fn is_orm_span(metadata: &Metadata<'_>) -> bool {
    metadata.is_span()
        && metadata.target() == ORM_TARGET
        && metadata.name().starts_with(ORM_SPAN_PREFIX)
}

/// Whether `filter` has a directive that selects by span or span field: only
/// those are written with `[...]`, in `RUST_LOG` and in the filter's
/// `Display` output.
fn has_span_directives(filter: &EnvFilter) -> bool {
    filter.to_string().contains('[')
}

impl<S: Subscriber> Layer<S> for OrmSpansOnDemand {
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        if self.waits_for(metadata).is_some() {
            // Asked again at every span, so that the first recorder turns
            // the spans on.
            Interest::sometimes()
        } else {
            Interest::always()
        }
    }

    fn enabled(&self, metadata: &Metadata<'_>, _: Context<'_, S>) -> bool {
        self.waits_for(metadata)
            .is_none_or(|recorded| recorded.load(Ordering::Relaxed))
    }
}

#[cfg(test)]
#[path = "orm_spans_tests.rs"]
mod tests;
