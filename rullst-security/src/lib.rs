pub mod audit;
pub mod cswsh;
pub mod deception;
pub mod honey;
pub mod log_redactor;
pub mod mfa;
pub mod rasp;
pub mod rate_limit;
pub mod rbac;
pub mod sanitizer;
pub mod schema_guard;
pub mod siem;
pub mod sri;
pub mod telemetry;
pub mod vault;
pub mod zero_trust;

pub use audit::{AuditChain, AuditLogger, AuditRecord, StdoutAuditLogger};
pub use cswsh::cswsh_guard_middleware;
pub use deception::{deception_trap_middleware, register_deception_trap};
pub use honey::{HoneypotLayer, HoneypotService, HoneypotState};
pub use log_redactor::redact_secrets;
pub use mfa::{
    build_otpauth_uri, decode_base32, generate_mfa_secret, generate_totp_code, verify_totp_code,
};
pub use rasp::{RaspInspector, RaspSecurityLayer, RaspSecurityService};
pub use rate_limit::{is_rate_limited, rate_limit_middleware};
pub use rbac::{RbacGuard, UserContext};
pub use sanitizer::{csp::CspSecurityLayer, HtmlSanitizer};
pub use schema_guard::{inspect_json_payload, schema_guard_middleware};
pub use siem::{dispatch_siem_alert, format_cef_event, SiemAlertPayload};
pub use sri::{compute_sri_hash, sri_link_tag, sri_script_tag};
pub use telemetry::{
    current_timestamp_str, get_real_rss_memory_mb, LiveSecurityEvent, SecurityStore,
    SecurityTelemetry, TelemetrySnapshot,
};
pub use vault::{FieldEncryptor, VaultSecret};
pub use zero_trust::{generate_fingerprint, verify_fingerprint};


