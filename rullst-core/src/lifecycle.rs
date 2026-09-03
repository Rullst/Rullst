//! Bounded process lifecycle, readiness, and graceful request draining.

use axum::{
    Router,
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::{Next, from_fn_with_state},
    response::{IntoResponse, Response},
};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::Notify;

const PHASE_STARTING: u8 = 0;
const PHASE_READY: u8 = 1;
const PHASE_DRAINING: u8 = 2;
const PHASE_STOPPED: u8 = 3;
const MAX_REQUIRED_COMPONENTS: usize = 32;
const MAX_COMPONENT_NAME_BYTES: usize = 64;
const MAX_DRAIN_WAIT: Duration = Duration::from_secs(600);

/// Monotonic phase of one Rullst application process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ApplicationPhase {
    /// Startup is still in progress.
    Starting,
    /// Startup finished; readiness still depends on every required component.
    Ready,
    /// New application requests are rejected while accepted requests finish.
    Draining,
    /// The process lifecycle has ended.
    Stopped,
}

impl ApplicationPhase {
    fn from_u8(value: u8) -> Self {
        match value {
            PHASE_READY => Self::Ready,
            PHASE_DRAINING => Self::Draining,
            PHASE_STOPPED => Self::Stopped,
            _ => Self::Starting,
        }
    }
}

/// Typed lifecycle construction, transition, and drain failures.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum ApplicationLifecycleError {
    /// A readiness component label is empty, too long, or ambiguous.
    #[error("invalid readiness component label `{0}`")]
    InvalidComponentLabel(String),
    /// A readiness component was registered more than once.
    #[error("duplicate readiness component label `{0}`")]
    DuplicateComponent(String),
    /// The readiness component is not part of the immutable registry.
    #[error("unknown readiness component label `{0}`")]
    UnknownComponent(String),
    /// Too many readiness components were requested.
    #[error("readiness accepts at most {MAX_REQUIRED_COMPONENTS} required components")]
    TooManyComponents,
    /// A transition would move the lifecycle backwards.
    #[error("cannot transition application lifecycle from {from:?} to {to:?}")]
    InvalidTransition {
        /// Current phase.
        from: ApplicationPhase,
        /// Requested phase.
        to: ApplicationPhase,
    },
    /// The readiness registry lock was poisoned and is denied rather than trusted.
    #[error("readiness component state is unavailable")]
    StateUnavailable,
    /// A drain wait must have a bounded, non-zero deadline.
    #[error("drain wait must be greater than zero and at most 600 seconds")]
    InvalidDrainWait,
    /// Accepted requests remained active until the configured deadline.
    #[error("graceful drain timed out with {in_flight} request(s) still active")]
    DrainTimedOut {
        /// Number of requests still executing when the deadline elapsed.
        in_flight: usize,
    },
    /// The in-flight counter cannot safely accept another request.
    #[error("in-flight request capacity is exhausted")]
    RequestCapacityExhausted,
    /// Startup, a dependency gate, or draining currently denies admission.
    #[error("application phase {phase:?} is not accepting requests")]
    RequestNotAdmitted {
        /// Phase observed when admission was denied.
        phase: ApplicationPhase,
    },
}

struct LifecycleInner {
    phase: AtomicU8,
    in_flight: AtomicUsize,
    components: RwLock<BTreeMap<String, bool>>,
    drained: Notify,
}

/// Shared process-local readiness and graceful-drain coordinator.
///
/// Required component labels are immutable and intentionally absent from the
/// public health payload. Applications update them only after performing their
/// own authenticated dependency checks. This type does not discover services,
/// coordinate replicas, or authorize domain requests.
#[derive(Clone)]
pub struct ApplicationLifecycle {
    inner: Arc<LifecycleInner>,
}

impl fmt::Debug for ApplicationLifecycle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let snapshot = self.snapshot();
        formatter
            .debug_struct("ApplicationLifecycle")
            .field("phase", &snapshot.phase)
            .field("required_components", &snapshot.required_components)
            .field("unready_components", &snapshot.unready_components)
            .field("in_flight_requests", &snapshot.in_flight_requests)
            .finish_non_exhaustive()
    }
}

impl Default for ApplicationLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

impl ApplicationLifecycle {
    /// Creates a lifecycle in the `Starting` phase without required components.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(LifecycleInner {
                phase: AtomicU8::new(PHASE_STARTING),
                in_flight: AtomicUsize::new(0),
                components: RwLock::new(BTreeMap::new()),
                drained: Notify::new(),
            }),
        }
    }

    /// Creates a lifecycle with an immutable bounded set of required components.
    pub fn with_required_components<I, S>(components: I) -> Result<Self, ApplicationLifecycleError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut required = BTreeMap::new();
        for component in components {
            if required.len() >= MAX_REQUIRED_COMPONENTS {
                return Err(ApplicationLifecycleError::TooManyComponents);
            }
            let component = component.into();
            validate_component_label(&component)?;
            if required.insert(component.clone(), false).is_some() {
                return Err(ApplicationLifecycleError::DuplicateComponent(component));
            }
        }

        Ok(Self {
            inner: Arc::new(LifecycleInner {
                phase: AtomicU8::new(PHASE_STARTING),
                in_flight: AtomicUsize::new(0),
                components: RwLock::new(required),
                drained: Notify::new(),
            }),
        })
    }

    /// Returns the current monotonic phase.
    pub fn phase(&self) -> ApplicationPhase {
        ApplicationPhase::from_u8(self.inner.phase.load(Ordering::Acquire))
    }

    /// Marks startup complete without overriding required component states.
    pub fn mark_ready(&self) -> Result<(), ApplicationLifecycleError> {
        match self.inner.phase.compare_exchange(
            PHASE_STARTING,
            PHASE_READY,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) | Err(PHASE_READY) => Ok(()),
            Err(value) => Err(ApplicationLifecycleError::InvalidTransition {
                from: ApplicationPhase::from_u8(value),
                to: ApplicationPhase::Ready,
            }),
        }
    }

    /// Updates one pre-registered dependency readiness bit.
    pub fn set_component_ready(
        &self,
        component: impl Into<String>,
        ready: bool,
    ) -> Result<(), ApplicationLifecycleError> {
        let component = component.into();
        validate_component_label(&component)?;
        let mut components = self
            .inner
            .components
            .write()
            .map_err(|_| ApplicationLifecycleError::StateUnavailable)?;
        let state = components
            .get_mut(&component)
            .ok_or_else(|| ApplicationLifecycleError::UnknownComponent(component.clone()))?;
        *state = ready;
        Ok(())
    }

    /// Begins a monotonic drain and rejects all subsequent application requests.
    pub fn begin_draining(&self) -> Result<(), ApplicationLifecycleError> {
        loop {
            let current = self.inner.phase.load(Ordering::Acquire);
            match current {
                PHASE_DRAINING => return Ok(()),
                PHASE_STOPPED => {
                    return Err(ApplicationLifecycleError::InvalidTransition {
                        from: ApplicationPhase::Stopped,
                        to: ApplicationPhase::Draining,
                    });
                }
                PHASE_STARTING | PHASE_READY => {
                    if self
                        .inner
                        .phase
                        .compare_exchange(
                            current,
                            PHASE_DRAINING,
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        )
                        .is_ok()
                    {
                        if self.in_flight_requests() == 0 {
                            self.inner.drained.notify_waiters();
                        }
                        return Ok(());
                    }
                }
                _ => continue,
            }
        }
    }

    /// Marks the lifecycle stopped. This terminal transition is idempotent.
    pub fn mark_stopped(&self) {
        self.inner.phase.store(PHASE_STOPPED, Ordering::Release);
        self.inner.drained.notify_waiters();
    }

    /// Returns a secret-minimized readiness snapshot.
    pub fn snapshot(&self) -> ReadinessSnapshot {
        let phase = self.phase();
        let in_flight_requests = self.in_flight_requests();
        let component_counts = self.inner.components.read().ok().map(|components| {
            let unready = components.values().filter(|ready| !**ready).count();
            (components.len(), unready)
        });

        let (required_components, unready_components, state_available) = match component_counts {
            Some((required, unready)) => (required, unready, true),
            None => (0, 0, false),
        };
        let ready = phase == ApplicationPhase::Ready && state_available && unready_components == 0;

        ReadinessSnapshot {
            phase,
            ready,
            state_available,
            required_components,
            unready_components,
            in_flight_requests,
        }
    }

    /// Returns the number of application requests admitted and still executing.
    pub fn in_flight_requests(&self) -> usize {
        self.inner.in_flight.load(Ordering::Acquire)
    }

    /// Waits for all requests admitted before draining to finish.
    pub async fn wait_for_drain(&self, timeout: Duration) -> Result<(), ApplicationLifecycleError> {
        if timeout.is_zero() || timeout > MAX_DRAIN_WAIT {
            return Err(ApplicationLifecycleError::InvalidDrainWait);
        }
        let phase = self.phase();
        if !matches!(
            phase,
            ApplicationPhase::Draining | ApplicationPhase::Stopped
        ) {
            return Err(ApplicationLifecycleError::InvalidTransition {
                from: phase,
                to: ApplicationPhase::Draining,
            });
        }

        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let notified = self.inner.drained.notified();
            tokio::pin!(notified);
            // Register before observing the counter so a final request cannot
            // reach zero between the observation and waiter registration.
            notified.as_mut().enable();
            if self.in_flight_requests() == 0 {
                return Ok(());
            }
            if tokio::time::timeout_at(deadline, notified).await.is_err() {
                // A terminal notification racing the deadline still wins when
                // the authoritative counter proves the drain completed.
                if self.in_flight_requests() == 0 {
                    return Ok(());
                }
                return Err(ApplicationLifecycleError::DrainTimedOut {
                    in_flight: self.in_flight_requests(),
                });
            }
        }
    }

    fn try_admit(&self) -> Result<ApplicationRequestGuard, ApplicationLifecycleError> {
        loop {
            if !self.snapshot().ready {
                return Err(ApplicationLifecycleError::RequestNotAdmitted {
                    phase: self.phase(),
                });
            }
            let active = self.inner.in_flight.load(Ordering::Acquire);
            let next = active
                .checked_add(1)
                .ok_or(ApplicationLifecycleError::RequestCapacityExhausted)?;
            if self
                .inner
                .in_flight
                .compare_exchange(active, next, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                continue;
            }
            if self.snapshot().ready {
                return Ok(ApplicationRequestGuard {
                    inner: Arc::clone(&self.inner),
                });
            }
            release_request(&self.inner);
            return Err(ApplicationLifecycleError::RequestNotAdmitted {
                phase: self.phase(),
            });
        }
    }
}

/// Bounded readiness counters safe for public orchestration probes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct ReadinessSnapshot {
    /// Current process phase.
    pub phase: ApplicationPhase,
    /// Whether the process accepts application requests.
    pub ready: bool,
    /// Whether component state could be read without lock corruption.
    pub state_available: bool,
    /// Number of immutable required component slots.
    pub required_components: usize,
    /// Number of required components currently marked unavailable.
    pub unready_components: usize,
    /// Number of admitted requests still executing.
    pub in_flight_requests: usize,
}

struct ApplicationRequestGuard {
    inner: Arc<LifecycleInner>,
}

impl Drop for ApplicationRequestGuard {
    fn drop(&mut self) {
        release_request(&self.inner);
    }
}

fn release_request(inner: &LifecycleInner) {
    let previous = inner.in_flight.fetch_sub(1, Ordering::AcqRel);
    if previous == 1 && inner.phase.load(Ordering::Acquire) >= PHASE_DRAINING {
        inner.drained.notify_waiters();
    }
}

fn validate_component_label(component: &str) -> Result<(), ApplicationLifecycleError> {
    if component.is_empty()
        || component.len() > MAX_COMPONENT_NAME_BYTES
        || !component
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(ApplicationLifecycleError::InvalidComponentLabel(
            component.to_string(),
        ));
    }
    Ok(())
}

/// Applies lifecycle admission to application routes.
///
/// Exact `GET`/`HEAD /health` and `/ready` probes bypass admission so an
/// orchestrator can observe startup and drain state. All other requests receive
/// a bounded `503` once startup/dependency readiness is false or draining starts.
pub fn apply_lifecycle(router: Router, lifecycle: ApplicationLifecycle) -> Router {
    router.layer(from_fn_with_state(lifecycle, lifecycle_middleware))
}

async fn lifecycle_middleware(
    State(lifecycle): State<ApplicationLifecycle>,
    request: Request,
    next: Next,
) -> Response {
    let is_probe = matches!(
        request.method(),
        &axum::http::Method::GET | &axum::http::Method::HEAD
    ) && matches!(request.uri().path(), "/health" | "/ready");
    if is_probe {
        return next.run(request).await;
    }

    match lifecycle.try_admit() {
        Ok(_guard) => next.run(request).await,
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            [
                (header::CACHE_CONTROL, "no-store"),
                (header::RETRY_AFTER, "1"),
                (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
            ],
            "Service is not ready to accept application requests.",
        )
            .into_response(),
    }
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
