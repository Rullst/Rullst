//! 13.0.0-alpha.6 removed every API still deprecated: Mail's infallible
//! constructors and tracking helpers, the legacy OTA methods, Nexus's
//! infallible builders, AI's `structured_prompt` and `is_safe`, Security's
//! `with_distributed` and `AuditChain::new`, `Queue::purge_completed_jobs`,
//! the Connect providers' `new`, the `#[route]` marker and
//! `rullst_orm::schema_diff`.

use super::*;

const MAIL: &str = "V13-MAIL-DEPRECATED-REMOVED";
const IOT: &str = "V13-IOT-OTA-LEGACY-REMOVED";
const NEXUS: &str = "V13-NEXUS-INFALLIBLE-BUILD-REMOVED";
const AI: &str = "V13-AI-DEPRECATED-REMOVED";
const SECURITY: &str = "V13-SECURITY-DEPRECATED-REMOVED";
const QUEUE: &str = "V13-QUEUE-PURGE-COMPLETED-REMOVED";
const CONNECT: &str = "V13-CONNECT-INFALLIBLE-NEW-REMOVED";
const ROUTE: &str = "V13-ROUTE-MARKER-REMOVED";
const SCHEMA_DIFF: &str = "V13-ORM-SCHEMA-DIFF-REMOVED";

/// The lines at which `code` is found in `source`.
fn found(source: &str, code: &str) -> Vec<usize> {
    scan_with(source, "lib.rs", &WorkspaceFacts::default())
        .into_iter()
        .filter(|(found, _)| *found == code)
        .map(|(_, line)| line)
        .collect()
}

fn rule(code: &str) -> &'static Rule {
    CATALOG
        .iter()
        .copied()
        .find(|rule| rule.code == code)
        .unwrap()
}

#[test]
fn the_rules_date_from_alpha6_and_name_their_replacements() {
    for (code, kind, replacement) in [
        (MAIL, FindingKind::MustChange, "ResendDriver::try_new"),
        (IOT, FindingKind::Review, "OtaManager::new_with_trusted_key"),
        (NEXUS, FindingKind::Review, ".try_build()"),
        (AI, FindingKind::Review, "passed_heuristics"),
        (SECURITY, FindingKind::MustChange, "AuditChain::try_new"),
        (QUEUE, FindingKind::Review, "purge_failed_jobs"),
        (CONNECT, FindingKind::MustChange, "GithubProvider::try_new"),
        (ROUTE, FindingKind::MustChange, "rullst::routes!"),
        (SCHEMA_DIFF, FindingKind::MustChange, "make:migration:auto"),
    ] {
        let rule = rule(code);
        assert_eq!(rule.introduced, "13.0.0-alpha.6", "{code}");
        assert_eq!(rule.kind, kind, "{code}");
        assert!(rule.guidance.contains(replacement), "{code}");
        assert!(rule.message.contains("13.0.0-alpha.6"), "{code}");
        // A review rule for a removed API repeats its guidance on a failed gate.
        assert_eq!(
            REMOVED_API_RULES.iter().any(|removed| removed.code == code),
            kind == FindingKind::Review,
            "{code}"
        );
    }
}

#[test]
fn removed_mail_constructors_and_tracking_helpers_must_change() {
    let source = r#"
use rullst::mail::{AwsSesDriver, Message, ResendDriver, TrackingEngine};
fn drivers() -> (ResendDriver, AwsSesDriver) {
    (ResendDriver::new("re_key"), AwsSesDriver::new("us-east-1", "token"))
}
fn tracked(message: Message, secret: &[u8]) -> Message {
    message.with_open_tracking("https://t.example", secret, "c1")
        .with_click_tracking("https://t.example", secret)
}
fn tokens(secret: &[u8], html: &str) -> (String, String, String, String) {
    (
        TrackingEngine::generate_open_token(secret, "a@example.com", "c1", 0),
        TrackingEngine::generate_click_token(secret, "a@example.com", "https://x.example", 0),
        TrackingEngine::inject_open_pixel(html, "https://t.example/open"),
        TrackingEngine::rewrite_links(html, "https://t.example", secret, "a@example.com", 0),
    )
}
"#;
    assert_eq!(found(source, MAIL), [4, 7, 8, 12, 13, 14, 15]);
}

#[test]
fn the_fallible_mail_forms_and_other_types_are_not_reported() {
    let source = r#"
use rullst::mail::{Message, ResendDriver, TrackingEngine};
fn driver() -> Result<ResendDriver, rullst::mail::MailError> { ResendDriver::try_new("re_key") }
fn tracked(message: Message, secret: &[u8]) -> Result<Message, rullst::mail::TrackingError> {
    message.try_with_open_tracking("https://t.example", secret, "c1")?
        .try_with_click_tracking("https://t.example", secret)
}
fn token(secret: &[u8]) -> Result<String, rullst::mail::TrackingError> {
    TrackingEngine::try_generate_open_token(secret, "a@example.com", "c1", 0)
}
// ResendDriver::new and with_open_tracking were removed.
const NOTE: &str = "TrackingEngine::rewrite_links";
"#;
    assert_eq!(found(source, MAIL), Vec::<usize>::new());
    // A builder method of the same name in a file that does not name Mail.
    let own = "fn f(b: Banner) -> Banner { b.with_open_tracking(\"x\", &[], \"c\") }\n";
    assert_eq!(found(own, MAIL), Vec::<usize>::new());
}

#[test]
fn legacy_ota_calls_are_reviewed_in_files_that_name_the_manager() {
    let source = r#"
use rullst_iot::{OtaError, OtaManager};
fn legacy(payload: &[u8], signature: &[u8]) -> Result<(), OtaError> {
    let mut ota = OtaManager::new("1.0.0")?;
    ota.verify_signature(payload, signature)?;
    ota.commit_update("2.0.0").map(drop)
}
fn unsupported(error: &OtaError) -> bool {
    matches!(error, OtaError::LegacyApiUnsupported { .. })
}
"#;
    assert_eq!(found(source, IOT), [4, 5, 6, 9]);
    let kept = r#"
use rullst_iot::OtaManager;
fn verified(key: [u8; 32]) -> Result<OtaManager, rullst_iot::OtaError> {
    OtaManager::new_with_trusted_key("board", "1.0.0", 1, key)
}
"#;
    assert_eq!(found(kept, IOT), Vec::<usize>::new());
    // Capital's `verify_signature` in a file without the OTA manager.
    let capital = "fn f(p: &StripeProvider) { let _ = p.verify_signature(b\"{}\", \"sig\"); }\n";
    assert_eq!(found(capital, IOT), Vec::<usize>::new());
}

#[test]
fn nexus_infallible_builders_are_reviewed_in_files_that_name_nexus() {
    let source = r#"
use rullst::nexus::{Nexus, NexusAuthPolicy};
fn admin(policy: NexusAuthPolicy) -> rullst::web::axum::Router {
    Nexus::new().with_auth_policy(policy).register::<Post>().build()
}
fn legacy() -> rullst::web::axum::Router {
    Nexus::new().into_router()
}
fn studio(access: LocalStudioAccess) -> Result<Router, StudioBuildError> {
    Studio::new().into_router(access)
}
fn checked() -> Result<rullst::web::axum::Router, rullst::nexus::NexusBuildError> {
    Nexus::new().try_build()
}
"#;
    // Studio's `into_router(access)` takes an argument and is not Nexus's.
    assert_eq!(found(source, NEXUS), [4, 7]);
    let other = "fn client() -> Client { Client::builder().build() }\n";
    assert_eq!(found(other, NEXUS), Vec::<usize>::new());
}

#[test]
fn removed_ai_helpers_are_reviewed() {
    let source = r#"
use rullst::ai::{AiClient, AiGuardrails};
async fn answer(client: &AiClient) -> Result<Summary, rullst::ai::AiError> {
    client.structured_prompt::<Summary>("summarize").await
}
fn allowed(text: &str) -> bool {
    AiGuardrails::inspect(text).is_safe()
}
"#;
    assert_eq!(found(source, AI), [4, 7]);
    let kept = r#"
use rullst::ai::{AiClient, AiGuardrails};
async fn answer(client: &AiClient) -> Result<Summary, rullst::ai::AiError> {
    client.json_prompt::<Summary>("summarize").await
}
fn allowed(text: &str) -> bool {
    AiGuardrails::inspect(text).passed_heuristics()
}
"#;
    assert_eq!(found(kept, AI), Vec::<usize>::new());
    // `is_safe` of another type, and Security's firewall report field.
    let other = "fn f(path: &Path, report: Report) -> bool { path.is_safe() && report.is_safe }\n";
    assert_eq!(found(other, AI), Vec::<usize>::new());
}

#[test]
fn removed_security_builders_must_change() {
    let source = r#"
use rullst::security::{AuditChain, RateLimiter, StdoutAuditLogger};
fn limiter() -> RateLimiter {
    RateLimiter::new(10, std::time::Duration::from_secs(1)).with_distributed()
}
fn chain(key: &[u8]) -> AuditChain {
    AuditChain::new(key, std::sync::Arc::new(StdoutAuditLogger))
}
"#;
    assert_eq!(found(source, SECURITY), [4, 7]);
    let kept = r#"
use rullst::security::{AuditChain, RateLimiter, StdoutAuditLogger};
fn limiter() -> Result<RateLimiter, rullst::security::RateLimitError> {
    RateLimiter::new(10, std::time::Duration::from_secs(1)).try_with_distributed()
}
fn chain(key: &[u8]) -> Result<AuditChain, rullst::security::SecurityError> {
    AuditChain::try_new(key, std::sync::Arc::new(StdoutAuditLogger))
}
fn studio(store: DistributedTraceStore) -> Studio { Studio::new().with_distributed_traces(store) }
"#;
    assert_eq!(found(kept, SECURITY), Vec::<usize>::new());
}

#[test]
fn the_queue_alias_is_reviewed_and_the_driver_hook_is_not_a_call() {
    let source = r#"
use rullst::queue::{Queue, QueueDriver, QueueError};
async fn clean(queue: &Queue) -> Result<(), QueueError> {
    queue.purge_completed_jobs().await?;
    Queue::purge_completed_jobs(queue).await
}
struct Driver;
#[async_trait::async_trait]
impl QueueDriver for Driver {
    async fn purge_completed_jobs(&self) -> Result<(), QueueError> { Ok(()) }
}
"#;
    // The `impl` itself is reported by its own rule, not this one.
    assert_eq!(found(source, QUEUE), [4, 5]);
    let kept = "async fn clean(queue: &rullst::queue::Queue) { let _ = queue.purge_failed_jobs().await; }\n";
    assert_eq!(found(kept, QUEUE), Vec::<usize>::new());
}

#[test]
fn connect_infallible_constructors_must_change_in_files_that_name_connect() {
    let source = r#"
use rullst::connect::providers::{AppleProvider, Auth0Provider, CognitoProvider, GithubProvider, GoogleProvider};
use rullst::connect::configuration::CredentialMode;
fn providers(secret: SecretString) {
    let github = GithubProvider::new("id", secret.clone(), "https://app.example/callback");
    let google = rullst::connect::providers::GoogleProvider::new("id", secret.clone(), "https://app.example/cb");
    let auth0 = Auth0Provider::new("id", secret.clone(), "https://app.example/cb", "tenant.auth0.com");
    let cognito = CognitoProvider::new("id", secret, "https://app.example/cb", "https://x.example");
    let apple = AppleProvider::new("id", "team", "key", "pem", "https://app.example/cb");
    for provider in [DiscordProvider::new, FacebookProvider::new, LinkedinProvider::new, MicrosoftProvider::new, XProvider::new] {}
    assert!(github.credential_mode() != CredentialMode::Invalid);
}
"#;
    assert_eq!(found(source, CONNECT), [5, 6, 7, 8, 9, 10, 11]);
    let kept = r#"
use rullst::connect::providers::GithubProvider;
fn github(secret: SecretString) -> Result<GithubProvider, rullst::connect::ConnectError> {
    GithubProvider::try_new("id", secret, "https://app.example/callback")
}
"#;
    assert_eq!(found(kept, CONNECT), Vec::<usize>::new());
    // An application's own `GithubProvider` in a file that does not name Connect.
    let own = "struct GithubProvider;\nimpl GithubProvider { fn new() -> Self { Self } }\nfn f() -> GithubProvider { GithubProvider::new() }\n";
    assert_eq!(found(own, CONNECT), Vec::<usize>::new());
}

#[test]
fn the_route_marker_and_its_imports_must_change() {
    let source = r#"
use rullst::{html, route};
use rullst_macros::route as marker;

#[route]
async fn home() -> &'static str { "home" }

#[rullst::route]
async fn about() -> &'static str { "about" }

struct Pages;
impl Pages {
    #[rullst_core::route]
    async fn contact() -> &'static str { "contact" }
}
"#;
    assert_eq!(found(source, ROUTE), [2, 3, 5, 8, 13]);
}

#[test]
fn other_route_attributes_and_routes_are_not_the_marker() {
    let source = r#"
use actix_web::route;
use rullst::routes;

#[route("/", method = "GET")]
async fn index() -> &'static str { "actix" }

#[routes]
fn legacy() {}

fn router() -> rullst::Router {
    routes! { get("/" => index) }
}
"#;
    assert_eq!(found(source, ROUTE), Vec::<usize>::new());
    // A bare `#[route]` in a file that never names Rullst.
    let other = "use other::route;\n#[route]\nfn handler() {}\n";
    assert_eq!(found(other, ROUTE), Vec::<usize>::new());
}

#[test]
fn schema_diff_paths_and_names_must_change() {
    let source = r#"
use rullst_orm::schema_diff::{ParsedField, ParsedTable, extract_tables_from_ast};
fn tables() -> Vec<ParsedTable> {
    rullst::orm::schema_diff::extract_tables_from_ast()
}
fn first(table: &ParsedTable) -> Option<&ParsedField> { table.fields.first() }
fn count() -> usize { extract_tables_from_ast().len() }
"#;
    assert_eq!(found(source, SCHEMA_DIFF), [2, 3, 4, 6, 7]);
    // An application's own parser types without the module.
    let own =
        "struct ParsedTable;\nfn extract_tables_from_ast() -> Vec<ParsedTable> { Vec::new() }\n";
    assert_eq!(found(own, SCHEMA_DIFF), Vec::<usize>::new());
}
