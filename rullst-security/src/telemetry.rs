use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default, Debug)]
pub struct SecurityTelemetry {
    pub sanitizations_count: AtomicU64,
    pub honeypot_traps_count: AtomicU64,
    pub rbac_denials_count: AtomicU64,
}

impl SecurityTelemetry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn inc_sanitizations(&self) {
        self.sanitizations_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_honeypot_traps(&self) {
        self.honeypot_traps_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_rbac_denials(&self) {
        self.rbac_denials_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> TelemetrySnapshot {
        TelemetrySnapshot {
            sanitizations: self.sanitizations_count.load(Ordering::Relaxed),
            honeypot_traps: self.honeypot_traps_count.load(Ordering::Relaxed),
            rbac_denials: self.rbac_denials_count.load(Ordering::Relaxed),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    pub sanitizations: u64,
    pub honeypot_traps: u64,
    pub rbac_denials: u64,
}
