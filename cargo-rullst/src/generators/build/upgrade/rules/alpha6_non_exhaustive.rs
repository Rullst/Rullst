//! 13.0.0-alpha.6: the public enums and structs the 13.0 API freeze made
//! `#[non_exhaustive]`. [`super::rust`] calls [`match_rule`] with each `match`
//! that has no catch-all arm, [`literal_rule`] with each struct literal (also
//! inside `vec![...]`) and [`call_rule`] with each call of a tuple struct.
//!
//! A type counts only in a file that names its crate, and only with names of
//! its own: a `match` must name one of the enum's variants and a literal only
//! fields of the struct, so an application's own type of the same name is
//! left alone.

use super::Rule;
use super::catalog::{NON_EXHAUSTIVE_LITERAL, NON_EXHAUSTIVE_LITERAL_GLOB, NON_EXHAUSTIVE_MATCH};

/// One enum per line: its name, its crate and its variants.
const ENUMS: &str = "
DurableAuditError ai InvalidCapacity InvalidPath UnsafeFileType CapacityExceeded RecordCapacityExceeded RecordTooLarge InvalidEvent CorruptRecord ExternalModification UnhealthyWriter DurabilityUncertain RecoveryFailed LockUnavailable Io Encoding
JsonCapability ai Unsupported PromptOnly NativeMode
SqlChatBackend ai Postgres Mysql Sqlite
JwtPolicyMode auth Development Production
JwtRevocationMode auth ProcessLocal Shared
CeremonyDurability auth ProcessLocal SharedDurable
SubscriptionStatus capital Active Canceled PastDue Unpaid Trialing Paused
DiError core NotRegistered DowncastFailed InjectionFailed
RealtimeError core BroadcastError InvalidChannel InvalidEvent PayloadTooLarge InvalidPresenceIdentity
StorageError core Io NotFound PathTraversal Driver Unsupported Cloud
ValidationError core ExtractionError ValidationError
WsError core SendError RecvError
ParentPolicy core StartNew TrustedPeer
TelemetryError core InvalidConfig InvalidContext Unavailable Export
AnomalyState iot Normal Warning CriticalAnomaly
CoapMessageType iot Confirmable NonConfirmable
CoapMethod iot Get Post Put Delete
HarvesterState iot Charging Full Low Critical
ModbusFunction iot ReadHoldingRegisters WriteSingleRegister
NodeStatus iot Online Degraded Offline
PinMode iot Input Output Analog
PowerMode iot FullActive LowPower DeepSleep
MissedRunPolicy messaging CatchUp Coalesce
OccurrenceState messaging Pending Leased Published DeadLetter Cancelled
ModelOperation orm Created Updated Deleted
ColumnDefault orm CurrentTimestamp Null Integer Float Text
AgeError privacy InvalidConfiguration MethodNotAllowed InvalidChallenge InvalidChallengeToken Expired BindingMismatch InvalidAttestation InvalidSignature Replay DurableReplayRequired StoreUnavailable StoreCapacity StoreConfiguration ClockRollback MockInProduction EntropyUnavailable
ReplayDurability privacy ProcessLocal SharedDurable
ConsentDurability privacy ProcessLocal SharedDurable
ConsentError privacy InvalidConfiguration BindingMismatch RevisionConflict RevisionExhausted DurableStateRequired StoreUnavailable StoreCapacity StoreConfiguration ClockRollback
AuthenticatedSiemSpoolError security InvalidCapacity InvalidPath UnsafeFileType InvalidKeyId InvalidKeyMaterial DuplicateKeyId TooManyKeys CapacityExceeded RecordCapacityExceeded RecordTooLarge CorruptRecord UnknownKey AuthenticationFailed ExternalModification UnhealthyWriter DurabilityUncertain RecoveryFailed LockUnavailable Io Encoding
SiemSpoolError security InvalidCapacity InvalidPath UnsafeFileType CapacityExceeded RecordCapacityExceeded RecordTooLarge CorruptRecord ExternalModification UnhealthyWriter DurabilityUncertain RecoveryFailed LockUnavailable Io Encoding
SriError security Inspect TooLarge Read
SecurityError security Unauthorized Forbidden VaultError AuditChainError SanitizationError SchemaGuardError WafBlocked General
PromptThreatCategory security DirectJailbreak SystemPromptLeakage DelimiterHijacking DataExfiltration InvisibleUnicode
RateLimitBackend security Memory Distributed
RedisRateLimitMode security Distributed OfflineMock
DistributedTraceKind studio Http Sql Ai Job Cache Application
QueryFindingKind studio RepeatedOperation SlowOperation
PlaybackKind media Embed Hls Mp4_720p Original
ProviderMode media Offline ProtocolFixture RemoteUnvalidated
Lifecycle media Creating Active Deleting Deleted
";

/// One struct with named fields per line: its name, its crate and its fields.
const STRUCTS: &str = "
FetchedResource ai final_url status content_type body
Message ai role content
ProviderCapabilities ai text chat embeddings vision json json_schema streaming tools request_timeout retries explicit_cancellation
RecordedToolAuditEvent ai sequence event
ToolAuditEvent ai principal tool risk approved_by approval_reason outcome
ToolParam ai name param_type description required
VectorDocument ai id vector payload
RagAuditEvent ai tenant_id query_sha256 retrieved_documents included_documents context_chars outcome
RecordedRagAuditEvent ai sequence event
ApplicationJwtClaims auth sub iss aud iat nbf exp jti session_version scopes token_use schema_version
AllowCredential auth type id
AuthenticatorAssertionResponse auth authenticator_data signature client_data_json
AuthenticatorAttestationResponse auth attestation_object client_data_json
AuthenticatorSelection auth user_verification resident_key
CreationChallengeResponse auth public_key
Passkey auth credential_id public_key sign_count
PubKeyCredParam auth type alg
PublicKeyCredential auth id raw_id type response
PublicKeyCredentialCreationOptions auth challenge rp user pub_key_cred_params timeout authenticator_selection attestation
PublicKeyCredentialRequestOptions auth challenge timeout rp_id allow_credentials user_verification
RegisterPublicKeyCredential auth id raw_id type response
RelyingPartyInfo auth name id
RequestChallengeResponse auth public_key
UserInfo auth id name display_name
RevenueMetrics capital mrr_cents arr_cents net_revenue_cents active_subscriptions churn_rate_percent
WebhookEvent capital subscription_id customer_id customer_email plan_id status ends_at
WebhookEventRecord capital id provider event_type status timestamp payload_snippet
ConnectUser connect id name email email_verified avatar_url raw_data access_token refresh_token expires_in
UniversalProfile connect id name email email_verified avatar_url
HttpRequest connect method url headers form json basic_auth bearer_auth
HttpResponse connect status body
AuthCallback connect code state error error_description
AuthQuery connect client_id redirect_uri response_type scope state
TokenForm connect client_id client_secret code grant_type redirect_uri
ExchangeParams connect auth_code code_verifier expected_nonce
Oauth2TokenResponse connect access_token refresh_token expires_in
TokenExchangeForm connect client_id client_secret code grant_type redirect_uri code_verifier
DeviceAuthorizationResponse connect device_code user_code verification_uri verification_uri_complete expires_in interval
ObjectMetadata core size_bytes etag
QueuedJobDetail core id name payload status error attempts created_at updated_at
RealtimeMessage core channel event payload
InfrastructureRecommendation core metric_name current_value target_value recommended_action urgency
StackFrame core file line function
QueuedJob core id name payload attempts
RadarSnapshot core uptime_seconds memory_rss_mb cpu_usage_percent active_tokio_tasks tokio_latency_micros timestamp
BannedIpRecord core ip reason timestamp_str expires_at_unix_secs
LiveSecurityEvent core schema_version event_type details client_ip timestamp_str verified_hmac
TelemetrySnapshot core sanitizations honeypot_traps rbac_denials log_redactions zero_trust_mismatches schema_violations sri_signed_assets mfa_verifications deception_hits cswsh_blocks rate_limit_blocks siem_dispatches login_jail_bans dlp_secrets_masked secure_headers_applied idor_warnings timing_guard_protected prompt_injections_blocked
MultipartPart core number size_bytes matches_checkpoint
TelemetryStatus core forwarded_spans rejected_spans offline_spans accepted_batches failed_batches
TraceSpan core name kind duration_us timestamp
DigitalTwin iot device_id readings is_online
GattCharacteristic iot uuid value notify
GattService iot service_uuid characteristics
MeshNode iot node_id status rssi_dbm
MeshTopology iot nodes
PowerGovernor iot mode battery_mv solar_mv
SensorTelemetry iot device_id metric value timestamp
Attachment mail filename content mime_type cid
ClickEvent mail email target_url timestamp
MailAssertion mail message
Message mail to subject body_html body_text from reply_to unsubscribe_url unsubscribe_email send_at attachments
OpenEvent mail email campaign_id timestamp
ResendDriver mail api_key
SmtpDriver mail host port username password
BatchActionForm nexus action selected_ids
ChatRequest nexus message
FieldMeta nexus name label kind hidden readonly
NexusState nexus registry brand audit_policy
PaginationParams nexus page q sort_by order
RegistryEntry nexus table label icon pk pk_columns tenant_column fields
ClaimedOutboxEvent orm id stream event_key event_kind payload_json attempts claim_key claim_expires_at_epoch
EnqueuedOutboxEvent orm id inserted
JoinClause orm table conditions bindings errors
JsonResource orm data
ModelCommittedEvent orm table id operation payload
PaginationResult orm data total per_page current_page last_page
PrivacyReport orm table_name has_encrypted_data encrypted_fields
ResourceCollection orm data
AuditLog orm id model_type model_id event old_values new_values actor_kind actor_id tenant_key correlation_id reverted_audit_id reason format_version restore_patch created_at
SuggestedMigration orm name sql_statement target_table reason
RawQueryBuilder orm sql bindings
Column orm name col_type is_nullable is_primary_key is_auto_increment default_value is_unique
AuditRecord security sequence_id timestamp actor action resource payload previous_hash hash
PromptSafetyReport security is_safe threat_category matched_pattern sanitized_prompt
RateLimitDecision security allowed remaining retry_after
SiemAlertPayload security version event_type severity details client_ip timestamp_str
TimingGuardConfig security min_duration max_jitter enable_synthetic_cpu_cycles
TableQuery studio page search
IngestionSummary studio accepted duplicates
QueryFinding studio kind source trace_id operation occurrences maximum_duration_us
StoredDistributedTraceSpan studio source received_at_unix_s span
LoggerState studio tx
RequestLog studio method uri status latency_ms timestamp
ApiResponse studio success message
MigrationStatusItem studio name status
AlertRules studio unreachable error_rate_percent p95_ms p99_ms queue_pending queue_failures security_events cooldown_minutes
FetchLimits studio max_bytes timeout connect_timeout
Retention studio max_age_seconds max_samples_per_project max_alerts
StudioPaths studio projects key history
ProviderBinding media library mode environment
RemoteVideo media id library title description processing length_seconds mp4_720p
WebhookHeaders media version algorithm signature
Asset media id scope owner metadata video lifecycle processing published pending revision updated_at length_seconds mp4_720p failure
ThreatRadarStats studio honeypot_traps_blocked active_ip_bans xss_sanitizations rbac_violations_prevented log_redactions zero_trust_mismatches schema_violations sri_signed_assets mfa_verifications deception_hits cswsh_blocks rate_limit_blocks siem_dispatches login_jail_bans dlp_secrets_masked secure_headers_applied idor_warnings timing_guard_protected prompt_injections_blocked audit_chain_integrity threat_level live_events
";

/// The tuple structs, built by calling them like functions.
const TUPLE_STRUCTS: &str = "
RagAuditError ai
RagRetrievalError ai
";

/// The entries of a table: name, crate and the remaining words.
fn entries(
    table: &'static str,
) -> impl Iterator<Item = (&'static str, &'static str, Vec<&'static str>)> {
    table.lines().filter_map(|line| {
        let mut words = line.split_whitespace();
        Some((words.next()?, words.next()?, words.collect()))
    })
}

/// Whether `text` names the crate: `rullst_<name>`, or `<name>::` after a
/// character that cannot end an identifier, as in `rullst::<name>::...` (so
/// `openai::` does not name `ai`), but not after `crate::`, `self::` or
/// `super::`, which name a module of the application. Core's types are at
/// the root of the facade, so any `rullst` path counts for them.
fn names_crate(text: &str, name: &str) -> bool {
    if name == "core" {
        return text.contains("rullst");
    }
    let path = format!("{name}::");
    text.contains(&format!("rullst_{name}"))
        || text.match_indices(&path).any(|(at, _)| {
            let before = &text[..at];
            let own = ["crate::", "self::", "super::"]
                .iter()
                .any(|module| before.ends_with(module));
            !own && !before
                .chars()
                .next_back()
                .is_some_and(|before| before.is_alphanumeric() || before == '_')
        })
}

/// The rule for a `match` without a catch-all arm, given whether its arm
/// patterns name an identifier and the file's text: arms that name one of
/// the enums above and one of its variants, in a file that names its crate.
pub(super) fn match_rule(names: impl Fn(&str) -> bool, text: &str) -> Option<&'static Rule> {
    entries(ENUMS)
        .any(|(name, krate, variants)| {
            names(name) && variants.into_iter().any(&names) && names_crate(text, krate)
        })
        .then_some(&NON_EXHAUSTIVE_MATCH)
}

/// Whether `text` imports names through a glob that can bring in a Rullst
/// type without naming its crate: `use super::*;`, `use self::…::*;`,
/// `use crate::…::*;`, or a glob of a Rullst path.
fn glob_import(text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim();
        let path = line
            .strip_prefix("pub use ")
            .or_else(|| line.strip_prefix("use "))
            .and_then(|rest| rest.strip_suffix("::*;"));
        path.is_some_and(|path| {
            ["super", "self", "crate"]
                .iter()
                .any(|module| path == *module || path.starts_with(&format!("{module}::")))
                || path.contains("rullst")
        })
    })
}

/// The rule for a construction of a struct of one of `crates`: the
/// must-change rule when the file names one of them, or the review rule when
/// it names none but imports a glob, which can bring the type in unnamed
/// ([`super::non_exhaustive_globs`] then looks for the name's origin).
fn construction(crates: &[&'static str], text: &str) -> Option<&'static Rule> {
    if crates.is_empty() {
        return None;
    }
    if crates.iter().any(|krate| names_crate(text, krate)) {
        return Some(&NON_EXHAUSTIVE_LITERAL);
    }
    glob_import(text).then_some(&NON_EXHAUSTIVE_LITERAL_GLOB)
}

/// The crates of the structs above named `name` whose fields include every
/// one of `fields` (none without fields).
pub(super) fn literal_crates(name: &str, fields: &[String]) -> Vec<&'static str> {
    if fields.is_empty() {
        return Vec::new();
    }
    entries(STRUCTS)
        .filter(|(struct_name, _, own)| {
            *struct_name == name && fields.iter().all(|field| own.contains(&field.as_str()))
        })
        .map(|(_, krate, _)| krate)
        .collect()
}

/// The crates of the tuple structs above named `name`.
pub(super) fn call_crates(name: &str) -> Vec<&'static str> {
    entries(TUPLE_STRUCTS)
        .filter(|(struct_name, _, _)| *struct_name == name)
        .map(|(_, krate, _)| krate)
        .collect()
}

/// The rule for a struct literal whose path ends in `name` and whose named
/// fields are `fields`: a struct above whose fields include every one of
/// them, in a file that names its crate (or, for review, imports a glob).
pub(super) fn literal_rule(name: &str, fields: &[String], text: &str) -> Option<&'static Rule> {
    construction(&literal_crates(name, fields), text)
}

/// The rule for a call whose path ends in `name`: one of the tuple structs
/// above, in a file that names its crate (or, for review, imports a glob).
pub(super) fn call_rule(name: &str, text: &str) -> Option<&'static Rule> {
    construction(&call_crates(name), text)
}
