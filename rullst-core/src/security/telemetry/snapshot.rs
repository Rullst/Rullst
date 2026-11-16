//! The counter values of a security store at one moment.

use serde::{Deserialize, Serialize};

/// The counter values of a [`SecurityStore`](super::SecurityStore) at one moment.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TelemetrySnapshot {
    /// [`SecurityStore::sanitizations_count`](super::SecurityStore::sanitizations_count).
    pub sanitizations: u64,
    /// [`SecurityStore::honeypot_traps_count`](super::SecurityStore::honeypot_traps_count).
    pub honeypot_traps: u64,
    /// [`SecurityStore::rbac_denials_count`](super::SecurityStore::rbac_denials_count).
    pub rbac_denials: u64,
    /// [`SecurityStore::log_redactions_count`](super::SecurityStore::log_redactions_count).
    pub log_redactions: u64,
    /// [`SecurityStore::zero_trust_mismatches_count`](super::SecurityStore::zero_trust_mismatches_count).
    pub zero_trust_mismatches: u64,
    /// [`SecurityStore::schema_violations_count`](super::SecurityStore::schema_violations_count).
    pub schema_violations: u64,
    /// [`SecurityStore::sri_signed_assets_count`](super::SecurityStore::sri_signed_assets_count).
    pub sri_signed_assets: u64,
    /// [`SecurityStore::mfa_verifications_count`](super::SecurityStore::mfa_verifications_count).
    pub mfa_verifications: u64,
    /// [`SecurityStore::deception_hits_count`](super::SecurityStore::deception_hits_count).
    pub deception_hits: u64,
    /// [`SecurityStore::cswsh_blocks_count`](super::SecurityStore::cswsh_blocks_count).
    pub cswsh_blocks: u64,
    /// [`SecurityStore::rate_limit_blocks_count`](super::SecurityStore::rate_limit_blocks_count).
    pub rate_limit_blocks: u64,
    /// [`SecurityStore::siem_dispatches_count`](super::SecurityStore::siem_dispatches_count).
    pub siem_dispatches: u64,
    /// [`SecurityStore::login_jail_bans_count`](super::SecurityStore::login_jail_bans_count).
    pub login_jail_bans: u64,
    /// [`SecurityStore::dlp_secrets_masked_count`](super::SecurityStore::dlp_secrets_masked_count).
    pub dlp_secrets_masked: u64,
    /// [`SecurityStore::secure_headers_applied_count`](super::SecurityStore::secure_headers_applied_count).
    pub secure_headers_applied: u64,
    /// [`SecurityStore::idor_warnings_count`](super::SecurityStore::idor_warnings_count).
    pub idor_warnings: u64,
    /// [`SecurityStore::timing_guard_protected_count`](super::SecurityStore::timing_guard_protected_count).
    pub timing_guard_protected: u64,
    /// [`SecurityStore::prompt_injections_blocked_count`](super::SecurityStore::prompt_injections_blocked_count).
    pub prompt_injections_blocked: u64,
}
