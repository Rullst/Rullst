use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

static GLOBAL_SECURITY_STORE: OnceLock<SecurityStore> = OnceLock::new();

const MAX_LIVE_EVENTS: usize = 50;
const MAX_TELEMETRY_BANS: usize = 10_000;
const MAX_HONEYPOT_ROUTES: usize = 1_024;
const DEFAULT_HONEYPOT_BAN_TTL: Duration = Duration::from_secs(15 * 60);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveSecurityEvent {
    pub event_type: String,
    pub details: String,
    pub client_ip: String,
    pub timestamp_str: String,
    pub verified_hmac: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BannedIpRecord {
    pub ip: String,
    pub reason: String,
    pub timestamp_str: String,
    pub expires_at_unix_secs: u64,
}

pub struct SecurityStore {
    pub sanitizations_count: AtomicU64,
    pub honeypot_traps_count: AtomicU64,
    pub rbac_denials_count: AtomicU64,
    pub prompt_injections_blocked_count: AtomicU64,
    pub prompts_inspected_count: AtomicU64,
    pub pii_masked_count: AtomicU64,
    pub log_redactions_count: AtomicU64,
    pub zero_trust_mismatches_count: AtomicU64,
    pub schema_violations_count: AtomicU64,
    pub sri_signed_assets_count: AtomicU64,
    pub mfa_verifications_count: AtomicU64,
    pub deception_hits_count: AtomicU64,
    pub cswsh_blocks_count: AtomicU64,
    pub rate_limit_blocks_count: AtomicU64,
    /// Local alerts recorded through the legacy SIEM dispatch facade.
    /// This is not a successful external-delivery counter.
    pub siem_dispatches_count: AtomicU64,
    pub login_jail_bans_count: AtomicU64,
    pub dlp_secrets_masked_count: AtomicU64,
    pub secure_headers_applied_count: AtomicU64,
    pub idor_warnings_count: AtomicU64,
    pub timing_guard_protected_count: AtomicU64,
    pub banned_ips: DashMap<String, BannedIpRecord>,
    pub honeypot_route_hits: DashMap<String, AtomicU64>,
    pub live_events: Mutex<Vec<LiveSecurityEvent>>,
}

impl Default for SecurityStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityStore {
    pub fn new() -> Self {
        Self {
            sanitizations_count: AtomicU64::new(0),
            honeypot_traps_count: AtomicU64::new(0),
            rbac_denials_count: AtomicU64::new(0),
            prompt_injections_blocked_count: AtomicU64::new(0),
            prompts_inspected_count: AtomicU64::new(0),
            pii_masked_count: AtomicU64::new(0),
            log_redactions_count: AtomicU64::new(0),
            zero_trust_mismatches_count: AtomicU64::new(0),
            schema_violations_count: AtomicU64::new(0),
            sri_signed_assets_count: AtomicU64::new(0),
            mfa_verifications_count: AtomicU64::new(0),
            deception_hits_count: AtomicU64::new(0),
            cswsh_blocks_count: AtomicU64::new(0),
            rate_limit_blocks_count: AtomicU64::new(0),
            siem_dispatches_count: AtomicU64::new(0),
            login_jail_bans_count: AtomicU64::new(0),
            dlp_secrets_masked_count: AtomicU64::new(0),
            secure_headers_applied_count: AtomicU64::new(0),
            idor_warnings_count: AtomicU64::new(0),
            timing_guard_protected_count: AtomicU64::new(0),
            banned_ips: DashMap::new(),
            honeypot_route_hits: DashMap::new(),
            live_events: Mutex::new(Vec::new()),
        }
    }

    pub fn global() -> &'static SecurityStore {
        GLOBAL_SECURITY_STORE.get_or_init(SecurityStore::new)
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

    pub fn record_honeypot_trap(&self, ip: &str, path: &str) {
        self.record_honeypot_trap_with_ttl(ip, path, DEFAULT_HONEYPOT_BAN_TTL);
    }

    pub fn record_honeypot_trap_with_ttl(&self, ip: &str, path: &str, ban_ttl: Duration) {
        self.record_honeypot_event(ip, path, Some(ban_ttl));
    }

    /// Records a trap hit without claiming that the peer was added to an enforcement ban list.
    pub fn record_honeypot_observation(&self, ip: &str, path: &str) {
        self.record_honeypot_event(ip, path, None);
    }

    fn record_honeypot_event(&self, ip: &str, path: &str, ban_ttl: Option<Duration>) {
        self.inc_honeypot_traps();
        let now_str = current_timestamp_str();
        let now = unix_timestamp_secs();
        let normalized_ip = normalize_ip(ip);
        self.prune_expired_bans_at(now);

        if let Some(ban_ttl) = ban_ttl
            && normalized_ip != "unknown"
            && (self.banned_ips.len() < MAX_TELEMETRY_BANS
                || self.banned_ips.contains_key(&normalized_ip))
        {
            self.banned_ips.insert(
                normalized_ip.clone(),
                BannedIpRecord {
                    ip: normalized_ip.clone(),
                    reason: format!("Triggered honeypot route {path}"),
                    timestamp_str: now_str.clone(),
                    expires_at_unix_secs: now.saturating_add(ban_ttl.as_secs()),
                },
            );
        }

        if self.honeypot_route_hits.contains_key(path)
            || self.honeypot_route_hits.len() < MAX_HONEYPOT_ROUTES
        {
            self.honeypot_route_hits
                .entry(path.to_string())
                .or_insert_with(|| AtomicU64::new(0))
                .fetch_add(1, Ordering::Relaxed);
        }

        self.push_live_event(LiveSecurityEvent {
            event_type: "HONEYPOT_TRAP_TRIGGERED".to_string(),
            details: format!("Peer {normalized_ip} accessed trap route {path}"),
            client_ip: normalized_ip,
            timestamp_str: now_str,
            verified_hmac: false,
        });
    }

    pub fn record_sanitization(&self, details: &str) {
        self.inc_sanitizations();
        self.push_live_event(LiveSecurityEvent {
            event_type: "XSS_PAYLOAD_NEUTRALIZED".to_string(),
            details: details.to_string(),
            client_ip: "unknown".to_string(),
            timestamp_str: current_timestamp_str(),
            verified_hmac: false,
        });
    }

    pub fn record_prompt_injection_blocked(&self, ip: &str, prompt_snippet: &str) {
        self.prompt_injections_blocked_count
            .fetch_add(1, Ordering::Relaxed);
        self.prompts_inspected_count.fetch_add(1, Ordering::Relaxed);
        self.push_live_event(LiveSecurityEvent {
            event_type: "AI_PROMPT_INJECTION_SHIELDED".to_string(),
            details: format!("Blocked malicious prompt snippet: {prompt_snippet}"),
            client_ip: normalize_ip(ip),
            timestamp_str: current_timestamp_str(),
            verified_hmac: false,
        });
    }

    pub fn record_prompt_inspected(&self) {
        self.prompts_inspected_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_pii_masked(&self, count: usize) {
        self.pii_masked_count
            .fetch_add(count as u64, Ordering::Relaxed);
    }

    pub fn record_rbac_denial(&self, actor: &str, resource: &str) {
        self.inc_rbac_denials();
        self.push_live_event(LiveSecurityEvent {
            event_type: "RBAC_ACCESS_DENIED".to_string(),
            details: format!("User {actor} denied access to {resource}"),
            client_ip: "unknown".to_string(),
            timestamp_str: current_timestamp_str(),
            verified_hmac: false,
        });
    }

    /// Removes expired display records and returns the current active-ban count.
    pub fn active_banned_count(&self) -> usize {
        self.prune_expired_bans();
        self.banned_ips.len()
    }

    /// Removes expired honeypot ban records from telemetry dashboards.
    pub fn prune_expired_bans(&self) {
        self.prune_expired_bans_at(unix_timestamp_secs());
    }

    fn prune_expired_bans_at(&self, now: u64) {
        self.banned_ips
            .retain(|_, record| record.expires_at_unix_secs > now);
    }

    /// Stores an unsigned local event. `verified_hmac` is forced to false because callers cannot
    /// truthfully promote an in-memory event to cryptographically verified telemetry.
    pub fn push_local_event(&self, mut event: LiveSecurityEvent) {
        event.verified_hmac = false;
        self.push_live_event(event);
    }

    fn push_live_event(&self, event: LiveSecurityEvent) {
        if let Ok(mut events) = self.live_events.lock() {
            events.insert(0, event);
            if events.len() > MAX_LIVE_EVENTS {
                events.truncate(MAX_LIVE_EVENTS);
            }
        }
    }

    pub fn inc_log_redactions(&self) {
        self.log_redactions_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_zero_trust_mismatches(&self) {
        self.zero_trust_mismatches_count
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_schema_violations(&self) {
        self.schema_violations_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_sri_signed_assets(&self) {
        self.sri_signed_assets_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_mfa_verifications(&self) {
        self.mfa_verifications_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_deception_hits(&self) {
        self.deception_hits_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_cswsh_blocks(&self) {
        self.cswsh_blocks_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_rate_limit_blocks(&self) {
        self.rate_limit_blocks_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_siem_dispatches(&self) {
        self.siem_dispatches_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_login_jail_bans(&self) {
        self.login_jail_bans_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_dlp_masked(&self) {
        self.dlp_secrets_masked_count
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_secure_headers(&self) {
        self.secure_headers_applied_count
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_idor_warnings(&self) {
        self.idor_warnings_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_timing_guard_protected(&self) {
        self.timing_guard_protected_count
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> TelemetrySnapshot {
        self.prune_expired_bans();
        TelemetrySnapshot {
            sanitizations: self.sanitizations_count.load(Ordering::Relaxed),
            honeypot_traps: self.honeypot_traps_count.load(Ordering::Relaxed),
            rbac_denials: self.rbac_denials_count.load(Ordering::Relaxed),
            log_redactions: self.log_redactions_count.load(Ordering::Relaxed),
            zero_trust_mismatches: self.zero_trust_mismatches_count.load(Ordering::Relaxed),
            schema_violations: self.schema_violations_count.load(Ordering::Relaxed),
            sri_signed_assets: self.sri_signed_assets_count.load(Ordering::Relaxed),
            mfa_verifications: self.mfa_verifications_count.load(Ordering::Relaxed),
            deception_hits: self.deception_hits_count.load(Ordering::Relaxed),
            cswsh_blocks: self.cswsh_blocks_count.load(Ordering::Relaxed),
            rate_limit_blocks: self.rate_limit_blocks_count.load(Ordering::Relaxed),
            siem_dispatches: self.siem_dispatches_count.load(Ordering::Relaxed),
            login_jail_bans: self.login_jail_bans_count.load(Ordering::Relaxed),
            dlp_secrets_masked: self.dlp_secrets_masked_count.load(Ordering::Relaxed),
            secure_headers_applied: self.secure_headers_applied_count.load(Ordering::Relaxed),
            idor_warnings: self.idor_warnings_count.load(Ordering::Relaxed),
            timing_guard_protected: self.timing_guard_protected_count.load(Ordering::Relaxed),
            prompt_injections_blocked: self.prompt_injections_blocked_count.load(Ordering::Relaxed),
        }
    }
}

pub fn current_timestamp_str() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn unix_timestamp_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Returns a canonical IP string only when the value is a syntactically valid address.
pub fn normalize_ip(value: &str) -> String {
    value
        .parse::<IpAddr>()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

pub fn get_real_rss_memory_mb() -> Option<f64> {
    rullst_core::radar::get_process_memory_mb()
}

pub type SecurityTelemetry = SecurityStore;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    pub sanitizations: u64,
    pub honeypot_traps: u64,
    pub rbac_denials: u64,
    pub log_redactions: u64,
    pub zero_trust_mismatches: u64,
    pub schema_violations: u64,
    pub sri_signed_assets: u64,
    pub mfa_verifications: u64,
    pub deception_hits: u64,
    pub cswsh_blocks: u64,
    pub rate_limit_blocks: u64,
    pub siem_dispatches: u64,
    pub login_jail_bans: u64,
    pub dlp_secrets_masked: u64,
    pub secure_headers_applied: u64,
    pub idor_warnings: u64,
    pub timing_guard_protected: u64,
    pub prompt_injections_blocked: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_are_absolute_rfc3339_values() {
        let timestamp = current_timestamp_str();
        assert!(chrono::DateTime::parse_from_rfc3339(&timestamp).is_ok());
        assert!(!timestamp.ends_with("s ago"));
        assert_ne!(timestamp, "Just now");
    }

    #[test]
    fn local_events_cannot_claim_hmac_verification_or_fake_ips() {
        let store = SecurityStore::new();
        store.push_local_event(LiveSecurityEvent {
            event_type: "TEST".to_string(),
            details: "local event".to_string(),
            client_ip: normalize_ip("attacker-controlled"),
            timestamp_str: current_timestamp_str(),
            verified_hmac: true,
        });

        let events = store.live_events.lock().expect("telemetry lock");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].client_ip, "unknown");
        assert!(!events[0].verified_hmac);
    }

    #[test]
    fn invalid_honeypot_identity_is_not_reported_as_an_active_ban() {
        let store = SecurityStore::new();
        store.record_honeypot_trap("not-a-peer-ip", "/.env");
        store.record_honeypot_observation("192.0.2.80", "/.git/config");

        assert_eq!(store.active_banned_count(), 0);
    }
}
