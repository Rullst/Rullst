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
pub use audit::{AuditChain, AuditLogger, AuditRecord, StdoutAuditLogger};
pub use cswsh::cswsh_guard_middleware;
pub use deception::{deception_trap_middleware, register_deception_trap};
pub use dlp::{DlpResponseLayer, DlpResponseService, mask_response_payload};
pub use headers::{SecureHeadersConfig, SecureHeadersLayer, SecureHeadersService};
pub use honey::{HoneypotLayer, HoneypotService, HoneypotState};
pub use log_redactor::redact_secrets;
pub use login_guard::LoginGuard;
pub use mfa::{
    build_otpauth_uri, decode_base32, generate_mfa_secret, generate_totp_code, verify_totp_code,
};
pub use rasp::{RaspInspector, RaspSecurityLayer, RaspSecurityService};
pub use rate_limit::{is_rate_limited, rate_limit_middleware};
pub use rbac::{RbacGuard, UserContext};
pub use sanitizer::{HtmlSanitizer, csp::CspSecurityLayer};
pub use schema_guard::{inspect_json_payload, schema_guard_middleware};
pub use siem::{SiemAlertPayload, dispatch_siem_alert, format_cef_event};
pub use sri::{compute_sri_hash, sri_link_tag, sri_script_tag};
pub use telemetry::{
    LiveSecurityEvent, SecurityStore, SecurityTelemetry, TelemetrySnapshot, current_timestamp_str,
    get_real_rss_memory_mb,
};
pub use timing_guard::{
    TimingGuardConfig, TimingScope, equalize_response_time, synthetic_argon2_cpu_work,
    timing_guard_middleware,
};
pub use vault::{FieldEncryptor, VaultSecret};
pub use zero_trust::{generate_fingerprint, verify_fingerprint};
