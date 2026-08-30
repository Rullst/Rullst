pub mod error;
pub use error::*;

pub mod ai_firewall;
pub mod audit;
pub mod cswsh;
pub mod deception;
pub mod dlp;
pub mod headers;
pub mod honey;
pub mod log_redactor;
pub mod login_guard;
pub mod mfa;
pub mod rasp;
pub mod rate_limit;
pub mod rbac;
pub mod recovery_codes;
pub mod sanitizer;
pub mod schema_guard;
pub mod siem;
pub mod sri;
pub mod telemetry;
pub mod timing_guard;
pub mod vault;
pub mod zero_trust;

pub use ai_firewall::{
    LlmFirewall, PromptSafetyReport, PromptThreatCategory, ai_firewall_middleware,
};
pub use audit::{AuditChain, AuditLogger, AuditRecord, MIN_AUDIT_KEY_BYTES, StdoutAuditLogger};
pub use cswsh::{CswsPolicy, CswsPolicyError, cswsh_guard_middleware};
pub use deception::{
    MAX_DECEPTION_TRAPS, deception_trap_middleware, register_deception_trap,
    try_register_deception_trap,
};
pub use dlp::{DlpLayer, DlpResponseLayer, DlpResponseService, DlpService, mask_response_payload};
pub use headers::{CspNonce, SecureHeadersConfig, SecureHeadersLayer, SecureHeadersService};
pub use honey::{
    DEFAULT_HONEYPOT_BAN_TTL, DEFAULT_MAX_HONEYPOT_BANS, HoneypotLayer, HoneypotService,
    HoneypotState, MAX_HONEYPOT_TRAP_PATHS,
};
pub use log_redactor::redact_secrets;
pub use login_guard::LoginGuard;
pub use mfa::{
    MIN_TOTP_SECRET_BYTES, build_mfa_qr_svg, build_otpauth_uri, decode_base32, generate_mfa_secret,
    generate_totp_code, try_generate_mfa_secret, verify_totp_code,
};
pub use rasp::{RaspInspector, RaspSecurityLayer, RaspSecurityService};
pub use rate_limit::{
    RateLimitBackend, RateLimitError, RateLimiter, is_rate_limited, rate_limit_middleware,
};
#[cfg(feature = "redis-rate-limit")]
pub use rate_limit::{RateLimitDecision, RedisRateLimitMode, RedisRateLimiter};
pub use rbac::{RbacGuard, UserContext};
pub use recovery_codes::{
    GeneratedRecoveryCodes, RecoveryCodeError, RecoveryCodeVerifier, consume_recovery_code,
    generate_recovery_codes, verify_recovery_code,
};
pub use sanitizer::{HtmlSanitizer, csp::CspSecurityLayer};
pub use schema_guard::{
    JsonSchemaPolicy, SchemaPolicyError, inspect_json_payload, json_schema_guard_middleware,
    schema_guard_middleware,
};
pub use siem::{SiemAlertPayload, dispatch_siem_alert, format_cef_event};
pub use sri::{
    MAX_SRI_ASSET_BYTES, SriError, compute_sri_hash, sri_link_tag, sri_link_tag_from_file,
    sri_script_tag, sri_script_tag_from_file,
};
pub use telemetry::{
    LIVE_SECURITY_EVENT_V1_JSON_SCHEMA, LiveSecurityEvent, SECURITY_EVENT_SCHEMA_VERSION,
    SecurityStore, SecurityTelemetry, TelemetrySnapshot, current_timestamp_str,
    get_real_rss_memory_mb, normalize_ip,
};
pub use timing_guard::{
    TimingGuardConfig, TimingScope, equalize_response_time, synthetic_argon2_cpu_work,
    timing_guard_middleware,
};
pub use vault::{FieldEncryptor, VaultError, VaultSecret};
pub use zero_trust::{
    MIN_FINGERPRINT_KEY_BYTES, generate_fingerprint, try_generate_fingerprint, verify_fingerprint,
};
