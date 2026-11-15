//! Process-local security telemetry: the counters, honeypot bans and bounded
//! event buffer that Studio's `/studio/security` page and the Nexus Threat
//! Radar render.
//!
//! The store lives in Core so those dashboards read it without depending on
//! `rullst-security`, whose layers record into it. `rullst-security`
//! re-exports these items at their earlier paths (`rullst_security::SecurityStore`,
//! `rullst_security::telemetry::BannedIpRecord`, …): both paths name the same
//! types and the same [`SecurityStore::global`] instance.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

mod event;
pub use event::{LiveSecurityEvent, SECURITY_EVENT_SCHEMA_VERSION};

static GLOBAL_SECURITY_STORE: OnceLock<SecurityStore> = OnceLock::new();

const MAX_LIVE_EVENTS: usize = 50;
const MAX_TELEMETRY_BANS: usize = 10_000;
const MAX_HONEYPOT_ROUTES: usize = 1_024;
const DEFAULT_HONEYPOT_BAN_TTL: Duration = Duration::from_secs(15 * 60);

/// An active honeypot ban shown on the security dashboards.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BannedIpRecord {
    /// Canonical IP address of the banned peer.
    pub ip: String,
    /// Why the peer was banned, for example the trap route it requested.
    pub reason: String,
    /// When the ban was recorded, as an RFC 3339 timestamp.
    pub timestamp_str: String,
    /// When the ban expires, in seconds since the Unix epoch.
    pub expires_at_unix_secs: u64,
}

/// Bounded, process-local security telemetry.
///
/// The security layers of `rullst-security` record into
/// [`SecurityStore::global`]; Studio and Nexus read it. Counters count only
/// what a mounted layer reported, and nothing is persisted or shared between
/// processes.
pub struct SecurityStore {
    /// HTML payloads neutralized by the sanitizer.
    pub sanitizations_count: AtomicU64,
    /// Honeypot trap hits.
    pub honeypot_traps_count: AtomicU64,
    /// Requests denied by an RBAC or ownership check.
    pub rbac_denials_count: AtomicU64,
    /// Prompts the LLM firewall blocked as prompt injection.
    pub prompt_injections_blocked_count: AtomicU64,
    /// Prompts the LLM firewall inspected.
    pub prompts_inspected_count: AtomicU64,
    /// Personal data values masked.
    pub pii_masked_count: AtomicU64,
    /// Log lines whose secrets were redacted.
    pub log_redactions_count: AtomicU64,
    /// Zero-trust fingerprint mismatches.
    pub zero_trust_mismatches_count: AtomicU64,
    /// Payloads rejected by a JSON schema guard.
    pub schema_violations_count: AtomicU64,
    /// Subresource Integrity hashes computed.
    pub sri_signed_assets_count: AtomicU64,
    /// Successful TOTP verifications.
    pub mfa_verifications_count: AtomicU64,
    /// Deception trap hits.
    pub deception_hits_count: AtomicU64,
    /// WebSocket handshakes refused by the cross-site WebSocket hijacking guard.
    pub cswsh_blocks_count: AtomicU64,
    /// Requests refused by a rate limit or a lockout.
    pub rate_limit_blocks_count: AtomicU64,
    /// Local alerts recorded through the legacy SIEM dispatch facade.
    /// This is not a successful external-delivery counter.
    pub siem_dispatches_count: AtomicU64,
    /// Peers locked out by the Login Jail.
    pub login_jail_bans_count: AtomicU64,
    /// Payloads in which the DLP layer masked secrets.
    pub dlp_secrets_masked_count: AtomicU64,
    /// Responses given the secure headers.
    pub secure_headers_applied_count: AtomicU64,
    /// IDOR warnings.
    pub idor_warnings_count: AtomicU64,
    /// Responses whose timing was equalized by the timing guard.
    pub timing_guard_protected_count: AtomicU64,
    // Read through `active_bans`, `honeypot_route_hits` and `live_events`.
    banned_ips: DashMap<String, BannedIpRecord>,
    honeypot_route_hits: DashMap<String, AtomicU64>,
    live_events: Mutex<Vec<LiveSecurityEvent>>,
}

impl Default for SecurityStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityStore {
    /// An empty store; tests and tools use their own instead of [`global`](Self::global).
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

    /// The process-wide store that the security layers record into and the
    /// dashboards read.
    pub fn global() -> &'static SecurityStore {
        GLOBAL_SECURITY_STORE.get_or_init(SecurityStore::new)
    }

    /// Counts a sanitization without recording an event.
    pub fn inc_sanitizations(&self) {
        self.sanitizations_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Counts a honeypot trap hit without recording a ban or an event.
    pub fn inc_honeypot_traps(&self) {
        self.honeypot_traps_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Counts an RBAC denial without recording an event.
    pub fn inc_rbac_denials(&self) {
        self.rbac_denials_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a trap hit and bans `ip` for 15 minutes.
    pub fn record_honeypot_trap(&self, ip: &str, path: &str) {
        self.record_honeypot_trap_with_ttl(ip, path, DEFAULT_HONEYPOT_BAN_TTL);
    }

    /// Records a trap hit and bans `ip` for `ban_ttl`. An `ip` that is not an
    /// IP address is not banned.
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

        self.push_live_event(LiveSecurityEvent::local(
            "HONEYPOT_TRAP_TRIGGERED",
            format!("Peer {normalized_ip} accessed trap route {path}"),
            normalized_ip,
        ));
    }

    /// Counts a sanitization and records an `XSS_PAYLOAD_NEUTRALIZED` event.
    pub fn record_sanitization(&self, details: &str) {
        self.inc_sanitizations();
        self.push_live_event(LiveSecurityEvent::local(
            "XSS_PAYLOAD_NEUTRALIZED",
            details,
            "unknown",
        ));
    }

    /// Records a RASP rejection emitted by an actual inspector or middleware path.
    pub fn record_rasp_interception(&self, uri_bad: bool, headers_bad: bool, body_bad: bool) {
        self.push_live_event(LiveSecurityEvent::local(
            "RASP_PAYLOAD_INTERCEPTED",
            format!("Rejected request (uri={uri_bad}, headers={headers_bad}, body={body_bad})"),
            "unknown",
        ));
    }

    /// Counts a blocked prompt and records an `AI_PROMPT_INJECTION_SHIELDED` event.
    pub fn record_prompt_injection_blocked(&self, ip: &str, prompt_snippet: &str) {
        self.prompt_injections_blocked_count
            .fetch_add(1, Ordering::Relaxed);
        self.push_live_event(LiveSecurityEvent::local(
            "AI_PROMPT_INJECTION_SHIELDED",
            format!("Blocked malicious prompt snippet: {prompt_snippet}"),
            ip,
        ));
    }

    /// Counts an inspected prompt.
    pub fn record_prompt_inspected(&self) {
        self.prompts_inspected_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Adds `count` masked personal data values.
    pub fn record_pii_masked(&self, count: usize) {
        self.pii_masked_count
            .fetch_add(count as u64, Ordering::Relaxed);
    }

    /// Counts an RBAC denial and records an `RBAC_ACCESS_DENIED` event.
    pub fn record_rbac_denial(&self, actor: &str, resource: &str) {
        self.inc_rbac_denials();
        self.push_live_event(LiveSecurityEvent::local(
            "RBAC_ACCESS_DENIED",
            format!("User {actor} denied access to {resource}"),
            "unknown",
        ));
    }

    /// Removes expired display records and returns the current active-ban count.
    pub fn active_banned_count(&self) -> usize {
        self.prune_expired_bans();
        self.banned_ips.len()
    }

    /// The active honeypot bans, sorted by IP address. Expired bans are
    /// removed first, as [`active_banned_count`](Self::active_banned_count)
    /// does; the store keeps at most 10,000.
    pub fn active_bans(&self) -> Vec<BannedIpRecord> {
        self.prune_expired_bans();
        let mut bans: Vec<BannedIpRecord> = self
            .banned_ips
            .iter()
            .map(|entry| entry.value().clone())
            .collect();
        bans.sort_by(|left, right| left.ip.cmp(&right.ip));
        bans
    }

    /// Whether `ip` has an active honeypot ban in this store.
    pub fn is_banned(&self, ip: &str) -> bool {
        let now = unix_timestamp_secs();
        self.banned_ips
            .get(&normalize_ip(ip))
            .is_some_and(|record| record.expires_at_unix_secs > now)
    }

    /// Hits per honeypot route, sorted by route. The store counts at most
    /// 1,024 routes.
    pub fn honeypot_route_hits(&self) -> Vec<(String, u64)> {
        let mut hits: Vec<(String, u64)> = self
            .honeypot_route_hits
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().load(Ordering::Relaxed)))
            .collect();
        hits.sort_by(|left, right| left.0.cmp(&right.0));
        hits
    }

    /// The most recent live security events, newest first (at most 50).
    pub fn live_events(&self) -> Vec<LiveSecurityEvent> {
        match self.live_events.lock() {
            Ok(events) => events.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
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
            events.insert(0, event.normalized());
            if events.len() > MAX_LIVE_EVENTS {
                events.truncate(MAX_LIVE_EVENTS);
            }
        }
    }

    /// Counts a redacted log line.
    pub fn inc_log_redactions(&self) {
        self.log_redactions_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Counts a zero-trust fingerprint mismatch.
    pub fn inc_zero_trust_mismatches(&self) {
        self.zero_trust_mismatches_count
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Counts a payload rejected by a JSON schema guard.
    pub fn inc_schema_violations(&self) {
        self.schema_violations_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Counts a computed Subresource Integrity hash.
    pub fn inc_sri_signed_assets(&self) {
        self.sri_signed_assets_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Counts a successful TOTP verification.
    pub fn inc_mfa_verifications(&self) {
        self.mfa_verifications_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Counts a deception trap hit.
    pub fn inc_deception_hits(&self) {
        self.deception_hits_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Counts a refused cross-site WebSocket handshake.
    pub fn inc_cswsh_blocks(&self) {
        self.cswsh_blocks_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Counts a request refused by a rate limit or a lockout.
    pub fn inc_rate_limit_blocks(&self) {
        self.rate_limit_blocks_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Counts a local alert recorded through the SIEM dispatch facade.
    pub fn inc_siem_dispatches(&self) {
        self.siem_dispatches_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Counts a Login Jail lockout.
    pub fn inc_login_jail_bans(&self) {
        self.login_jail_bans_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Counts a payload in which the DLP layer masked secrets.
    pub fn inc_dlp_masked(&self) {
        self.dlp_secrets_masked_count
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Counts a response given the secure headers.
    pub fn inc_secure_headers(&self) {
        self.secure_headers_applied_count
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Counts an IDOR warning.
    pub fn inc_idor_warnings(&self) {
        self.idor_warnings_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Counts a response whose timing was equalized.
    pub fn inc_timing_guard_protected(&self) {
        self.timing_guard_protected_count
            .fetch_add(1, Ordering::Relaxed);
    }

    /// The current counter values, after removing expired bans.
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

// Timestamp format of the telemetry store's events.
fn current_timestamp_str() -> String {
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

/// The counter values of a [`SecurityStore`] at one moment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    /// [`SecurityStore::sanitizations_count`].
    pub sanitizations: u64,
    /// [`SecurityStore::honeypot_traps_count`].
    pub honeypot_traps: u64,
    /// [`SecurityStore::rbac_denials_count`].
    pub rbac_denials: u64,
    /// [`SecurityStore::log_redactions_count`].
    pub log_redactions: u64,
    /// [`SecurityStore::zero_trust_mismatches_count`].
    pub zero_trust_mismatches: u64,
    /// [`SecurityStore::schema_violations_count`].
    pub schema_violations: u64,
    /// [`SecurityStore::sri_signed_assets_count`].
    pub sri_signed_assets: u64,
    /// [`SecurityStore::mfa_verifications_count`].
    pub mfa_verifications: u64,
    /// [`SecurityStore::deception_hits_count`].
    pub deception_hits: u64,
    /// [`SecurityStore::cswsh_blocks_count`].
    pub cswsh_blocks: u64,
    /// [`SecurityStore::rate_limit_blocks_count`].
    pub rate_limit_blocks: u64,
    /// [`SecurityStore::siem_dispatches_count`].
    pub siem_dispatches: u64,
    /// [`SecurityStore::login_jail_bans_count`].
    pub login_jail_bans: u64,
    /// [`SecurityStore::dlp_secrets_masked_count`].
    pub dlp_secrets_masked: u64,
    /// [`SecurityStore::secure_headers_applied_count`].
    pub secure_headers_applied: u64,
    /// [`SecurityStore::idor_warnings_count`].
    pub idor_warnings: u64,
    /// [`SecurityStore::timing_guard_protected_count`].
    pub timing_guard_protected: u64,
    /// [`SecurityStore::prompt_injections_blocked_count`].
    pub prompt_injections_blocked: u64,
}

#[cfg(test)]
mod store_tests;

#[cfg(test)]
mod tests;
