//! Revalidation of long-lived responses: a Server-Sent Events stream or a
//! WebSocket that outlives the request which authorized it.
//!
//! The route authenticates and authorizes the request once, when the
//! connection opens. A [`Revalidate`] check then runs every revalidation
//! interval ([`DEFAULT_REVALIDATION_INTERVAL`] unless configured, 1 second to
//! 5 minutes) and whenever [`Revalidate::changed`] reports that the answer
//! may have changed; the connection ends when the check answers `false` or
//! does not answer within [`REVALIDATION_DEADLINE`]. A session revoked in
//! another process is therefore noticed within one interval; a signal such
//! as an in-process revocation notice ends the connection at once.
//!
//! Core knows nothing about sessions: `rullst-auth`'s session registry
//! provides a check for its sessions (`SessionRegistry::revalidation`), and
//! [`revalidate_with`] turns any async closure into one.
//!
//! Added in v13.0.0-alpha.6.

use std::future::Future;
use std::time::Duration;
use tokio::time::{Instant, Interval, MissedTickBehavior};

/// How often an open connection is revalidated unless configured otherwise.
pub const DEFAULT_REVALIDATION_INTERVAL: Duration = Duration::from_secs(15);

/// Longest a single check may take; a check still running then ends the
/// connection, as a `false` answer does.
pub const REVALIDATION_DEADLINE: Duration = Duration::from_secs(5);

/// The accepted revalidation intervals: 1 second to 5 minutes.
pub(crate) const REVALIDATION_INTERVAL_RANGE: (Duration, Duration) =
    (Duration::from_secs(1), Duration::from_secs(300));

/// Decides, while a long-lived response stays open, whether its request may
/// still receive it.
pub trait Revalidate: Send + 'static {
    /// Whether the connection may continue, for example whether its session
    /// is still signed in. Failures should answer `false`: the client
    /// reconnects and the route authenticates it again.
    fn still_valid(&mut self) -> impl Future<Output = bool> + Send;

    /// Resolves when the answer of [`Self::still_valid`] may have changed,
    /// so the check runs at once instead of at the next interval. The
    /// default never resolves. The future is dropped and created again
    /// while the connection waits, so it must be cancel-safe (as
    /// `tokio::sync::broadcast::Receiver::recv` is).
    fn changed(&mut self) -> impl Future<Output = ()> + Send {
        std::future::pending()
    }
}

/// A [`Revalidate`] check made from an async closure; see [`revalidate_with`].
pub struct RevalidateWith<F>(F);

/// The check `check` as a [`Revalidate`], with no change signal.
///
/// ```rust
/// use rullst_core::realtime::revalidate_with;
///
/// // A check that always lets the connection continue.
/// let _check = revalidate_with(|| async { true });
/// ```
pub fn revalidate_with<F, Fut>(check: F) -> RevalidateWith<F>
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = bool> + Send,
{
    RevalidateWith(check)
}

impl<F, Fut> Revalidate for RevalidateWith<F>
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = bool> + Send,
{
    fn still_valid(&mut self) -> impl Future<Output = bool> + Send {
        (self.0)()
    }
}

/// The check of a connection opened without revalidation: it never runs.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoRevalidation;

impl Revalidate for NoRevalidation {
    async fn still_valid(&mut self) -> bool {
        true
    }
}

/// The interval clamped into [`REVALIDATION_INTERVAL_RANGE`].
pub(crate) fn clamp_interval(interval: Duration) -> Duration {
    interval.clamp(REVALIDATION_INTERVAL_RANGE.0, REVALIDATION_INTERVAL_RANGE.1)
}

/// The revalidation state of one open connection.
pub(crate) struct Revalidator<R> {
    check: R,
    interval: Interval,
}

impl<R: Revalidate> Revalidator<R> {
    /// Revalidates every `every` (clamped to 1 second–5 minutes), first one
    /// interval after now.
    pub(crate) fn new(check: R, every: Duration) -> Self {
        let every = clamp_interval(every);
        let mut interval = tokio::time::interval_at(Instant::now() + every, every);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        Self { check, interval }
    }

    /// Resolves when a check is due: the interval elapsed or the check's
    /// change signal fired. Cancel-safe; run [`Self::check`] afterwards,
    /// outside any `select!`, so traffic on the connection cannot cancel it.
    pub(crate) async fn due(&mut self) {
        tokio::select! {
            biased;
            () = self.check.changed() => {}
            _ = self.interval.tick() => {}
        }
    }

    /// Whether a check is due right now, without waiting.
    pub(crate) fn due_now(&mut self) -> bool {
        futures_util::FutureExt::now_or_never(self.due()).is_some()
    }

    /// One bounded check: `false` when it answers `false` or not within
    /// [`REVALIDATION_DEADLINE`].
    pub(crate) async fn check(&mut self) -> bool {
        tokio::time::timeout(REVALIDATION_DEADLINE, self.check.still_valid())
            .await
            .unwrap_or(false)
    }
}

/// Waits for the next due check of `revalidator`, or forever without one.
pub(crate) async fn due<R: Revalidate>(revalidator: &mut Option<Revalidator<R>>) {
    match revalidator {
        Some(revalidator) => revalidator.due().await,
        None => std::future::pending().await,
    }
}

#[cfg(test)]
#[path = "revalidation_tests.rs"]
mod tests;
