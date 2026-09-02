//! Authenticated Nexus administrator identity.

use std::{fmt, sync::Arc};

pub(crate) const NEXUS_ADMIN_ROLE: &str = "NexusAdmin";

/// Administrator capability inserted only by a validated Nexus access policy.
#[derive(Clone)]
#[non_exhaustive]
pub struct NexusPrincipal {
    actor_id: Arc<str>,
}

impl NexusPrincipal {
    pub(crate) fn authenticated(actor_id: impl Into<String>) -> Self {
        Self {
            actor_id: Arc::from(actor_id.into()),
        }
    }

    /// Returns the bounded identity established by the selected Nexus policy.
    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }
}

impl fmt::Debug for NexusPrincipal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NexusPrincipal")
            .field("actor_id_bytes", &self.actor_id.len())
            .finish()
    }
}

impl rullst_auth::HasRole for NexusPrincipal {
    fn has_role(&self, role: &str) -> bool {
        role == NEXUS_ADMIN_ROLE
    }
}
