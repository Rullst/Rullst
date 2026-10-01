//! Traffic Shield monitor lifecycle, load classification and the
//! backpressure middleware.

#[cfg(feature = "orm")]
use super::db_probe;
use super::{
    MONITORS_IDLE, MONITORS_RUNNING, MONITORS_SHUT_DOWN, TrafficShield, TrafficShieldConfig,
    TrafficShieldError, TrafficShieldMonitors, duration_millis_u64, shield_log,
};
use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use std::sync::atomic::{AtomicU8, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

impl TrafficShieldMonitors {
    fn abort_tasks(&self) {
        self.state.store(MONITORS_SHUT_DOWN, Ordering::Release);
        self.shutdown.notify_waiters();
        let mut tasks = match self.tasks.lock() {
            Ok(tasks) => tasks,
            Err(poisoned) => poisoned.into_inner(),
        };
        for task in tasks.drain(..) {
            task.abort();
        }
    }
}

impl Drop for TrafficShieldMonitors {
    fn drop(&mut self) {
        self.abort_tasks();
    }
}

impl TrafficShield {
    /// Creates a new inactive `TrafficShield`.
    ///
    /// Call [`TrafficShield::start`] from inside a Tokio runtime, attach the
    /// shield to [`crate::Server`], or use [`backpressure_middleware`], which
    /// starts it lazily on its first request. Keeping construction side-effect
    /// free makes this API safe in synchronous setup code and tests.
    pub fn new(config: TrafficShieldConfig) -> Self {
        Self {
            config,
            event_loop_lag_ms: Arc::new(AtomicU64::new(0)),
            db_latency_ms: Arc::new(AtomicU64::new(0)),
            active_requests: Arc::new(AtomicUsize::new(0)),
            monitors: Arc::new(TrafficShieldMonitors {
                state: AtomicU8::new(MONITORS_IDLE),
                shutdown: Arc::new(tokio::sync::Notify::new()),
                tasks: Mutex::new(Vec::new()),
                shed_log: shield_log::LogThrottle::new(),
                unavailable_log: shield_log::LogThrottle::new(),
            }),
        }
    }

    /// Starts event-loop and optional database monitoring.
    ///
    /// The operation is idempotent while running and returns a typed error when
    /// called outside Tokio or after [`TrafficShield::shutdown`]. All monitor
    /// tasks are cancelled on explicit shutdown or when the final shield clone
    /// is dropped.
    ///
    /// # Errors
    /// Returns [`TrafficShieldError::RuntimeUnavailable`] without spawning when
    /// no Tokio runtime is active, or [`TrafficShieldError::AlreadyShutDown`]
    /// after the lifecycle has ended.
    #[cfg_attr(mutants, mutants::skip)]
    pub fn start(&self) -> Result<(), TrafficShieldError> {
        match self.monitors.state.load(Ordering::Acquire) {
            MONITORS_RUNNING => return Ok(()),
            MONITORS_SHUT_DOWN => return Err(TrafficShieldError::AlreadyShutDown),
            _ => {}
        }
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| TrafficShieldError::RuntimeUnavailable)?;

        match self.monitors.state.compare_exchange(
            MONITORS_IDLE,
            MONITORS_RUNNING,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => {}
            Err(MONITORS_RUNNING) => return Ok(()),
            Err(_) => return Err(TrafficShieldError::AlreadyShutDown),
        }

        let mut tasks = match self.monitors.tasks.lock() {
            Ok(tasks) => tasks,
            Err(poisoned) => poisoned.into_inner(),
        };
        if self.monitors.state.load(Ordering::Acquire) != MONITORS_RUNNING {
            return Err(TrafficShieldError::AlreadyShutDown);
        }

        let lag_ms = self.event_loop_lag_ms.clone();
        let lag_shutdown = Arc::clone(&self.monitors.shutdown);
        tasks.push(runtime.spawn(async move {
            let interval = Duration::from_millis(100);
            loop {
                let start = Instant::now();
                tokio::select! {
                    _ = tokio::time::sleep(interval) => {}
                    _ = lag_shutdown.notified() => break,
                }
                let elapsed = start.elapsed();
                let lag = elapsed.saturating_sub(interval);
                lag_ms.store(duration_millis_u64(lag), Ordering::Relaxed);
            }
        }));

        #[cfg(feature = "orm")]
        if self.config.enable_db_probe {
            let db_lat_ms = self.db_latency_ms.clone();
            let db_shutdown = Arc::clone(&self.monitors.shutdown);
            let deadline = self.config.max_db_latency;
            tasks.push(runtime.spawn(async move {
                let interval = Duration::from_millis(1000);
                loop {
                    tokio::select! {
                        _ = tokio::time::sleep(interval) => {}
                        _ = db_shutdown.notified() => break,
                    }
                    if let Some(pool) = crate::db::safe_pool() {
                        // A probe that outlives the shedding threshold is
                        // already critical; abandon it so a hung connection
                        // cannot leave a stale healthy latency behind.
                        let probe = sqlx::query("SELECT 1").execute(pool);
                        let latency = db_probe::measure(deadline, probe).await;
                        db_lat_ms.store(latency, Ordering::Relaxed);
                    } else {
                        db_lat_ms.store(0, Ordering::Relaxed);
                    }
                }
            }));
        }

        Ok(())
    }

    /// Stops all background monitors and prevents this shared shield lifecycle
    /// from being restarted. Calling this method more than once is harmless.
    pub fn shutdown(&self) {
        self.monitors.abort_tasks();
    }

    /// Returns whether monitor tasks have been started and not shut down.
    pub fn is_running(&self) -> bool {
        self.monitors.state.load(Ordering::Acquire) == MONITORS_RUNNING
    }

    /// Returns the most recently measured Tokio event-loop lag as a `Duration`.
    /// Updated every 100ms by the background monitor task.
    pub fn event_loop_lag(&self) -> Duration {
        Duration::from_millis(self.event_loop_lag_ms.load(Ordering::Relaxed))
    }

    /// Returns the most recently measured database probe round-trip latency as a `Duration`.
    /// Returns `Duration::ZERO` if `enable_db_probe` is `false`, the `orm`
    /// feature is disabled, or the pool is uninitialized. A probe abandoned at
    /// `max_db_latency` reports at least that latency.
    pub fn db_latency(&self) -> Duration {
        Duration::from_millis(self.db_latency_ms.load(Ordering::Relaxed))
    }

    /// Returns the current count of in-flight HTTP requests being tracked by this shield.
    pub fn active_requests(&self) -> usize {
        self.active_requests.load(Ordering::Relaxed)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TrafficPressure {
    Normal,
    Moderate,
    Critical,
}

pub(super) fn classify_traffic_pressure(
    config: &TrafficShieldConfig,
    lag: Duration,
    db_latency: Duration,
    active_requests: usize,
) -> TrafficPressure {
    let critical = lag >= config.max_event_loop_lag
        || (config.enable_db_probe && db_latency >= config.max_db_latency)
        || active_requests >= config.max_active_requests;
    if critical {
        return TrafficPressure::Critical;
    }

    let moderate = lag >= config.max_event_loop_lag / 2
        || (config.enable_db_probe && db_latency >= config.max_db_latency / 2)
        || active_requests >= config.max_active_requests / 2;
    if moderate {
        TrafficPressure::Moderate
    } else {
        TrafficPressure::Normal
    }
}

struct ActiveRequestGuard<'a>(&'a AtomicUsize);

impl<'a> Drop for ActiveRequestGuard<'a> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Router-level protection middleware that tracks load timing and drops requests under critical saturation.
///
/// Rejections are reported on stderr at most once per second per shield,
/// with the number of rejections suppressed since the previous line, so
/// overload never turns into one blocking log write per request.
#[cfg_attr(mutants, mutants::skip)]
pub async fn backpressure_middleware(shield: TrafficShield, req: Request, next: Next) -> Response {
    if let Err(error) = shield.start() {
        if let Some(suppressed) = shield
            .monitors
            .unavailable_log
            .admit(Instant::now(), shield_log::LOG_INTERVAL)
        {
            crate::server::console::stderr_line(format_args!(
                "Traffic Shield is unavailable: {error} ({suppressed} similar rejections suppressed)"
            ));
        }
        let mut response = Response::new(axum::body::Body::from(
            "Traffic Shield monitoring is unavailable.",
        ));
        *response.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
        return response;
    }

    let active = shield.active_requests.fetch_add(1, Ordering::SeqCst);
    let _guard = ActiveRequestGuard(&shield.active_requests);

    let lag = shield.event_loop_lag();
    let db_lat = shield.db_latency();

    let pressure = classify_traffic_pressure(&shield.config, lag, db_lat, active);

    if pressure == TrafficPressure::Critical {
        if let Some(suppressed) = shield
            .monitors
            .shed_log
            .admit(Instant::now(), shield_log::LOG_INTERVAL)
        {
            crate::server::console::stderr_line(format_args!(
                "⚠️ [Rullst Backpressure] Load shedding active! CPU lag: {:?}, DB latency: {:?}, Active requests: {} ({} more requests shed since the previous report)",
                lag, db_lat, active, suppressed
            ));
        }

        match Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .header(axum::http::header::RETRY_AFTER, "5")
            .header(
                axum::http::header::CONTENT_TYPE,
                "text/plain; charset=utf-8",
            )
            .body(axum::body::Body::from(
                "Service Temporarily Saturated. Please try again soon.",
            )) {
            Ok(res) => return res,
            Err(_) => {
                let mut res = Response::new(axum::body::Body::empty());
                *res.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
                return res;
            }
        }
    }

    if pressure == TrafficPressure::Moderate {
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    next.run(req).await
}
