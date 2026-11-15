//! Long-lived responses of a registered session: Server-Sent Events and
//! WebSockets that end when the session is signed out or revoked.
//!
//! [`SessionRevalidation`] is the [`Revalidate`] check of one session for
//! `rullst_core::realtime::RealtimeSse::channel_revalidated` and
//! `rullst_core::WebSocket::with_revalidation`. It looks the session up at
//! each check, and every revocation through a [`SessionRegistry`] of this
//! process (sign-out, revoking one, the other or every session) notifies the
//! checks of the sessions it ends, so their responses end at once. A
//! revocation in another process is seen at the next check, one revalidation
//! interval later (15 seconds by default).

use super::registry::unix_now;
use super::types::{CurrentSession, SessionRef};
use super::{SessionDatabase, SessionRegistry};
use rullst_core::realtime::Revalidate;
use std::sync::OnceLock;
use tokio::sync::broadcast;

/// Notices kept for a slow check before it reads them; one that falls
/// further behind checks its session at once.
const NOTICE_CAPACITY: usize = 256;

/// A revocation this process made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Revoked {
    /// One session of the account.
    One { user_id: i32, reference: SessionRef },
    /// Every session of the account but `kept`.
    Others { user_id: i32, kept: SessionRef },
    /// Every session of the account.
    All { user_id: i32 },
}

impl Revoked {
    /// Whether this revocation ended `session`.
    pub(super) fn covers(&self, session: &CurrentSession) -> bool {
        match self {
            Self::One { user_id, reference } => {
                *user_id == session.user_id && *reference == session.reference
            }
            Self::Others { user_id, kept } => {
                *user_id == session.user_id && *kept != session.reference
            }
            Self::All { user_id } => *user_id == session.user_id,
        }
    }
}

fn notices() -> &'static broadcast::Sender<Revoked> {
    static NOTICES: OnceLock<broadcast::Sender<Revoked>> = OnceLock::new();
    NOTICES.get_or_init(|| broadcast::channel(NOTICE_CAPACITY).0)
}

/// Tells the checks of this process about a revocation.
pub(super) fn announce(revoked: Revoked) {
    // An error only means that no check is listening.
    let _ = notices().send(revoked);
}

/// The [`Revalidate`] check of one registered session; see
/// [`SessionRegistry::revalidation`].
pub struct SessionRevalidation<DB: SessionDatabase> {
    registry: SessionRegistry<DB>,
    current: CurrentSession,
    notices: broadcast::Receiver<Revoked>,
}

impl<DB: SessionDatabase> SessionRegistry<DB> {
    /// The check that ends a long-lived response of `current` once the
    /// session is signed out, revoked or expired: pass it to
    /// `RealtimeSse::channel_revalidated` or `WebSocket::with_revalidation`.
    /// A revocation through a registry of this process ends the response at
    /// once; one made by another process, at the next check.
    ///
    /// It checks the session record only: an application that revokes
    /// sessions by its session epoch alone should check the epoch in its own
    /// [`Revalidate`] (the generated applications delete the records
    /// whenever they bump the epoch).
    pub fn revalidation(&self, current: &CurrentSession) -> SessionRevalidation<DB> {
        SessionRevalidation {
            registry: self.clone(),
            current: current.clone(),
            notices: notices().subscribe(),
        }
    }

    /// Whether the record of `current` still exists for its account and has
    /// not expired; a failure answers `false`.
    pub(super) async fn is_live(&self, current: &CurrentSession, now: u64) -> bool {
        match DB::find(self.pool(), &current.token_hash).await {
            Ok(Some((token_hash, _, user_id, _, _, expires_at, _, _))) => {
                token_hash == current.token_hash
                    && user_id == current.user_id
                    && u64::try_from(expires_at).is_ok_and(|expires_at| now <= expires_at)
            }
            _ => false,
        }
    }
}

impl<DB: SessionDatabase> Revalidate for SessionRevalidation<DB> {
    async fn still_valid(&mut self) -> bool {
        match unix_now() {
            Ok(now) => self.registry.is_live(&self.current, now).await,
            Err(_) => false,
        }
    }

    async fn changed(&mut self) {
        loop {
            match self.notices.recv().await {
                Ok(revoked) if revoked.covers(&self.current) => return,
                Ok(_) => {}
                // Notices were missed: one of them may have ended it.
                Err(broadcast::error::RecvError::Lagged(_)) => return,
                Err(broadcast::error::RecvError::Closed) => std::future::pending::<()>().await,
            }
        }
    }
}

impl<DB: SessionDatabase> std::fmt::Debug for SessionRevalidation<DB> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SessionRevalidation")
            .field("session", &self.current)
            .finish_non_exhaustive()
    }
}

#[cfg(all(test, feature = "session-registry-sqlite"))]
#[path = "revalidation_tests.rs"]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests;
