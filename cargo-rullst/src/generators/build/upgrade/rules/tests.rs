#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

mod data;
mod files;

/// Codes and lines found in one Rust source.
fn scan_with(source: &str, file_name: &str, facts: &WorkspaceFacts) -> Vec<(&'static str, usize)> {
    let context = rust::FileContext {
        text: source,
        file_name,
        facts,
    };
    let mut collector = Collector::default();
    rust::scan_source(&context, &mut collector).expect("fixture parses");
    let mut findings = Vec::new();
    collector.finish(Path::new(file_name), &mut findings);
    findings
        .into_iter()
        .map(|finding| (finding.rule.code, finding.line))
        .collect()
}

fn codes(source: &str) -> Vec<&'static str> {
    scan_with(source, "lib.rs", &WorkspaceFacts::default())
        .into_iter()
        .map(|(code, _)| code)
        .collect()
}

fn assert_only(source: &str, expected: &[&str]) {
    let mut found = codes(source);
    found.sort_unstable();
    found.dedup();
    let mut expected = expected.to_vec();
    expected.sort_unstable();
    assert_eq!(found, expected, "{source}");
}

#[test]
fn dynamic_html_event_handlers_must_change_and_static_ones_do_not() {
    let source = r#"
fn view(handler: &str, value: &str) -> String {
    html! {
        <div>
            <button onclick={handler}>"Go"</button>
            <form hx-on-submit={handler} data-hx-on-click={handler} hx-on={handler}></form>
            <a onclick="go()" data-on={value} on={value} style={value} href={value}>"x"</a>
            {items.iter().map(|item| html! { <li ONLOAD={item}>"i"</li> }).collect::<String>()}
        </div>
    }
}
"#;
    let found = scan_with(source, "view.rs", &WorkspaceFacts::default());
    let lines: Vec<usize> = found
        .iter()
        .filter(|(code, _)| *code == "V13-HTML-DYNAMIC-EVENT-HANDLER")
        .map(|(_, line)| *line)
        .collect();
    assert_eq!(lines, vec![5, 6, 8], "{found:?}");
}

#[test]
fn comments_strings_and_other_macros_never_match_the_html_rule() {
    assert_only(
        r#"
// html! { <button onclick={handler}>"Go"</button> }
/// html! { <button onclick={handler}>"Go"</button> }
const TEXT: &str = "html! { <button onclick={handler}></button> }";
fn view() -> String { other! { <button onclick={handler}>"Go"</button> } }
"#,
        &[],
    );
}

#[test]
fn identifiers_match_in_code_and_macros_but_not_in_comments_or_strings() {
    assert_only(
        r#"
// SqlRecoveryStore::migrate is documented here.
/// Uses ValidatedJson, PresenceTracker and OidcProvider.
#[doc = "RagPipeline"]
const NOTE: &str = "SqlRecoveryStore GeminiProvider";
"#,
        &[],
    );
    assert_only(
        r#"
use rullst::auth::SqlRecoveryStore;
async fn submit(rullst::ValidatedJson(form): rullst::ValidatedJson<Form>) {
    tracing::info!("{:?}", rullst::realtime::PresenceTracker::new());
}
"#,
        &[
            "V13-AUTH-RECOVERY-MIGRATE",
            "V13-VALIDATION-STATUS",
            "V13-PRESENCE-COUNTING",
        ],
    );
}

#[test]
fn review_rules_report_the_first_location_and_must_change_every_one() {
    let source = "fn a() { User::all().await; }\nfn b() { User::all().await; }\nfn c() -> String { html! { <a onclick={x}></a> } }\nfn d() -> String { html! { <a onclick={y}></a> } }\n";
    let found = scan_with(source, "lib.rs", &WorkspaceFacts::default());
    assert_eq!(
        found,
        vec![
            ("V13-MODEL-ALL", 1),
            ("V13-HTML-DYNAMIC-EVENT-HANDLER", 3),
            ("V13-HTML-DYNAMIC-EVENT-HANDLER", 4),
        ]
    );
}

#[test]
fn calls_match_their_paths_but_not_definitions_or_similar_names() {
    assert_only(
        r#"
fn render_page(title: &str) -> String { title.to_string() }
async fn home(htmx: HtmxRequest) -> Html<String> {
    let flags = Permission::all();
    let any = items.iter().all(|item| item.ok);
    render_page_with_lang(&htmx, "en", "Home", String::new())
}
"#,
        &[],
    );
    assert_only(
        r#"
async fn home(htmx: HtmxRequest) -> Html<String> {
    let storage = Storage::r2("account", "bucket");
    let users = User::all().await?;
    let profile = Profile::find(1).await?;
    Mail::to("a@example.com").send(message).await?;
    rullst::htmx::render_page(&htmx, "Home", body)
}
"#,
        &[
            "V13-R2-PUBLIC-URL",
            "V13-MODEL-ALL",
            "V13-PORTFOLIO-PROFILE",
            "V13-MAIL-FACADE-CONFIG",
            "V13-RENDER-PAGE-LANGUAGE",
        ],
    );
}

#[test]
fn call_arguments_decide_literal_rules() {
    assert_only(
        r#"
async fn up() {
    Schema::create("UserProfiles", |table| table.id()).await;
    let plan = std::env::var("BILLING_PLAN_IDS");
}
"#,
        &["V13-SCHEMA-PG-TABLE-CASE", "V13-BILLING-PROJECT-SETTINGS"],
    );
    assert_only(
        r#"
async fn up() {
    Schema::create("user_profiles", |table| table.id()).await;
    let url = std::env::var("DATABASE_URL");
    let plan = rullst::config::project_setting("BILLING_PLAN_IDS");
    let profile = Profile::find(id).await;
}
"#,
        &[],
    );
}

#[test]
fn method_chains_decide_orm_rules() {
    assert_only(
        r#"
async fn jobs() {
    Post::query().where_eq("a", 1).limit(50).chunk(10, |rows| async { Ok(()) }).await;
    Post::query().where_eq("a", 1).remember(60).get().await;
    Post::query().only_trashed().delete_all().await;
    post.restore().await;
}
"#,
        &[
            "V13-ORM-CHUNK-BOUNDS",
            "V13-ORM-CACHE-PREFIX",
            "V13-ORM-ONLY-TRASHED",
            "V13-ORM-TRASHED-DELETE-ALL",
            "V13-ORM-MISSING-ROW",
        ],
    );
    assert_only(
        r#"
async fn jobs() {
    Post::query().chunk(10, |rows| async { Ok(()) }).await;
    cache.remember("key", ttl, || async { 1 }).await;
    Post::query().delete_all().await;
    backup.restore(path).await;
}
"#,
        &[],
    );
}

#[test]
fn string_rules_match_literals_only() {
    assert_only(
        r#"
const POLICY: &str = "no-referrer";
const SEED: &str = "INSERT INTO posts VALUES (1, datetime('now'))";
fn route() -> &'static str { "/_rullst/status" }
fn sitemap() -> &'static str { "<urlset><url><loc>/</loc></url></urlset>" }
"#,
        &[
            "V13-REFERRER-NO-REFERRER",
            "V13-SQLITE-ONLY-SEED-TIME",
            "V13-DEV-TELEMETRY-ROUTE",
            "V13-BLOG-ROBOTS-SITEMAP",
        ],
    );
    assert_only(
        r#"
// INSERT ... datetime('now') and "no-referrer"
fn routes() -> Router { routes![get("/sitemap.xml" => sitemap_xml)] }
fn robots() -> &'static str { "Sitemap: https://example.com/sitemap.xml" }
fn sitemap() -> &'static str { "<urlset><url><loc>https://example.com/</loc></url></urlset>" }
"#,
        &[],
    );
}

#[test]
fn orm_derives_report_relation_and_sqlx_changes() {
    assert_only(
        r#"
#[derive(Orm, Deserialize)]
#[sqlx(rename_all = "camelCase")]
#[orm(table = "BlogPosts", searchable)]
pub struct Post {
    pub id: i32,
    #[orm(belongs_to = "User")]
    pub author: Option<User>,
    #[orm(belongs_to = "Team", foreign_key = "team_id", local_key = "uuid")]
    pub team: Option<Team>,
    #[orm(has_many = "Comment", related_key = "post_uuid")]
    pub comments: Vec<Comment>,
    #[sqlx(json)]
    pub meta: Meta,
    pub password_hash: String,
    pub token: SecretString,
}
"#,
        &[
            "V13-ORM-SQLX-STRUCT-OPTION",
            "V13-ORM-SEARCHABLE-TABLE",
            "V13-ORM-BELONGS-TO-KEY",
            "V13-ORM-IGNORED-RELATION-KEY",
            "V13-ORM-SQLX-JSON-SERIALIZE",
            "V13-PASSWORD-HASH-HIDDEN",
            "V13-SECRET-STRING-CLIENT-INPUT",
            "V13-ORM-PROTECTED-VALUES",
        ],
    );
    assert_only(
        r#"
#[derive(Orm)]
#[sqlx(default)]
#[orm(table = "posts", searchable)]
pub struct Post {
    pub id: i32,
    #[orm(belongs_to = "User", foreign_key = "post_id")]
    pub author: Option<User>,
    #[orm(hidden)]
    pub password_hash: String,
}
#[derive(Debug)]
#[sqlx(rename_all = "camelCase")]
pub struct NotAModel {
    #[orm(belongs_to = "User")]
    pub author: Option<User>,
    pub password_hash: String,
}
#[derive(Deserialize)]
pub struct Login {
    #[serde(deserialize_with = "rullst_orm::privacy::deserialize_plaintext_secret")]
    pub token: SecretString,
}
"#,
        &[],
    );
}

#[test]
fn nexus_keys_and_field_kinds() {
    assert_only(
        r#"
#[derive(Nexus)]
#[nexus(primary_key = "code")]
pub struct Product {
    #[nexus(primary_key)]
    pub id: i64,
    pub code: String,
}
fn widget(kind: &FieldKind) -> &'static str {
    assert_eq!(kind, &FieldKind::Number);
    if *kind == FieldKind::Number { return "n"; }
    match kind {
        FieldKind::Text => "text",
        FieldKind::Number => "number",
    }
}
"#,
        &[
            "V13-NEXUS-PRIMARY-KEY",
            "V13-NEXUS-FIELD-KIND-NUMBER",
            "V13-NEXUS-FIELD-KIND-MATCH",
        ],
    );
    assert_only(
        r#"
#[derive(Nexus)]
pub struct Product {
    #[nexus(primary_key)]
    pub code: String,
}
fn fields() -> Vec<FieldMeta> {
    vec![FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true }]
}
fn widget(kind: &FieldKind) -> &'static str {
    match kind {
        FieldKind::Text => "text",
        _ => "other",
    }
}
"#,
        &[],
    );
}

#[test]
fn structural_rules_for_functions_impls_literals_and_lengths() {
    assert_only(
        r#"
struct Store;
impl rullst::queue::QueueDriver for Store {}
#[rullst_orm::test]
async fn creates_posts() {}
pub fn index_page(posts: Vec<Post>) -> String { String::new() }
fn register(name: &str, password: &str) -> User {
    if password.len() < 12 || name.len() > 120 { return User::default(); }
    User { created_at: String::new(), updated_at: "".to_string(), ..User::default() }
}
"#,
        &[
            "V13-QUEUE-DRIVER-PREVIEWS",
            "V13-ORM-SANDBOX-TEST",
            "V13-PAGE-CSP-NONCE",
            "V13-UTF16-LENGTHS",
            "V13-EMPTY-TIMESTAMPS",
        ],
    );
    assert_only(
        r#"
#[tokio::test]
async fn creates_posts() {}
pub fn index_page(posts: Vec<Post>, csp_nonce: &str) -> String { String::new() }
fn register(items: &[u8], password: &str) -> User {
    if items.len() < 12 || password.len() > 72 { return User::default(); }
    User { created_at: utc_timestamp(), ..User::default() }
}
"#,
        &[],
    );
}

#[test]
fn utoipa_and_mfa_rules_depend_on_their_context() {
    let source = "fn studio(doc: OpenApi) { Studio::new().with_openapi(doc); }";
    let old = WorkspaceFacts {
        utoipa_below_6: true,
    };
    let found = scan_with(source, "lib.rs", &old);
    assert_eq!(found, vec![("V13-STUDIO-UTOIPA-6", 1)]);
    assert!(scan_with(source, "lib.rs", &WorkspaceFacts::default()).is_empty());

    let mfa = "#[derive(Deserialize)]\npub struct Verify {\n    pub secret: String,\n}\n";
    assert_eq!(
        scan_with(mfa, "mfa.rs", &WorkspaceFacts::default()),
        vec![("V13-MFA-CLIENT-SECRET", 3)]
    );
    assert!(scan_with(mfa, "settings.rs", &WorkspaceFacts::default()).is_empty());
}

#[test]
fn erp_root_route_outside_the_protected_router_is_reviewed() {
    let protected = r#"
fn router(admin_access: Policy) {
    let admin = admin_access.protect_router(admin_routes.into_axum());
    let public = routes![get("/" => controllers::erp_controller::index)];
}
"#;
    assert_only(protected, &["V13-ERP-DASHBOARD-ACCESS"]);
    assert_only(
        r#"fn router() { let public = routes![get("/" => index), get("/about" => about)]; }"#,
        &[],
    );
}

/// The shapes the v12.1 Blank and Blog starters generate.
#[test]
fn v12_starter_shapes_produce_their_review_findings() {
    assert_only(
        r#"
use rullst::htmx::{HtmxRequest, render_page};
async fn home(htmx: HtmxRequest) -> impl IntoResponse {
    let db_status = match User::all().await { Ok(_) => "ok", Err(_) => "down" };
    render_page(&htmx, "Welcome to Rullst", content)
}
"#,
        &["V13-MODEL-ALL", "V13-RENDER-PAGE-LANGUAGE"],
    );
    assert_only(
        r#"
pub async fn index() -> impl IntoResponse {
    let posts = Post::all().await.unwrap_or_default();
    Html(blog::index_page(posts))
}
pub async fn robots_txt() -> impl IntoResponse {
    (StatusCode::OK, "User-agent: *\nSitemap: /sitemap.xml\n")
}
"#,
        &["V13-MODEL-ALL", "V13-BLOG-ROBOTS-SITEMAP"],
    );
}

#[test]
fn the_catalog_has_unique_codes_and_one_line_texts() {
    let mut seen = BTreeSet::new();
    for rule in CATALOG {
        assert!(seen.insert(rule.code), "duplicate {}", rule.code);
        assert!(rule.code.starts_with("V13-"), "{}", rule.code);
        for text in [rule.row, rule.message, rule.guidance] {
            assert!(!text.is_empty() && !text.contains('\n'), "{}", rule.code);
        }
    }
}

/// Every rule points at a row that exists exactly once in the guide.
#[test]
fn every_rule_names_a_unique_migration_row() {
    let guide = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/src/migration-v13.md");
    let Ok(text) = std::fs::read_to_string(guide) else {
        return; // Packaged crates do not ship the repository documentation.
    };
    for rule in CATALOG {
        let cell = format!("| {} |", rule.row);
        let rows = text.lines().filter(|line| line.starts_with(&cell)).count();
        assert_eq!(rows, 1, "{}: row `{}`", rule.code, rule.row);
    }
}

#[test]
fn unparsable_sources_are_listed_instead_of_scanned() {
    let root =
        std::env::temp_dir().join(format!("rullst-rules-unparsed-{}", rand::random::<u64>()));
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("src/main.rs"),
        "fn main() { Server::new(router).run(3000) }\n",
    )
    .unwrap();
    std::fs::write(root.join("src/broken.rs"), "fn broken( {\n").unwrap();
    let mut scan = SourceScan::default();
    scan_package(&root, &WorkspaceFacts::default(), &mut scan).unwrap();
    assert_eq!(scan.unscanned, vec![root.join("src/broken.rs")]);
    let codes: Vec<_> = scan
        .findings
        .iter()
        .map(|finding| finding.rule.code)
        .collect();
    assert_eq!(codes, vec!["V13-HEALTH-PROBES"]);

    std::fs::write(
        root.join("src/health.rs"),
        "pub fn probes() -> Router { rullst::health::health_router() }\n",
    )
    .unwrap();
    let mut scan = SourceScan::default();
    scan_package(&root, &WorkspaceFacts::default(), &mut scan).unwrap();
    assert!(scan.findings.is_empty(), "{:?}", scan.findings);
    std::fs::remove_dir_all(root).unwrap();
}
