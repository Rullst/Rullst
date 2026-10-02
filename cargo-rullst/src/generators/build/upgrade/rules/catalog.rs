//! The v12 → v13 source rules. Each rule names the first-column title of its
//! row in `docs/src/migration-v13.md`; the classification of every row is
//! reviewed in the assisted-upgrade tutorial.

use super::{FindingKind, Rule};

use FindingKind::{MustChange, Review};

macro_rules! rules {
    ($($name:ident: $code:literal, $kind:ident, $row:literal, $message:literal, $guidance:literal;)+) => {
        $(pub(crate) const $name: Rule = Rule {
            code: $code,
            kind: $kind,
            row: $row,
            message: $message,
            guidance: $guidance,
        };)+
        /// Every rule, in catalog order.
        #[cfg(test)]
        pub(crate) const CATALOG: &[&Rule] = &[$(&$name),+];
    };
}

rules! {
    AUTH_RECOVERY_MIGRATE: "V13-AUTH-RECOVERY-MIGRATE", Review, "Account/session registry",
        "run `SqlRecoveryStore::migrate()` before serving requests; the upgrade does not apply this SQL migration",
        "Call `SqlRecoveryStore::migrate()` once at start-up before requests are served, configure expired-session maintenance and authorize the new session inventory/logout routes.";
    R2_PUBLIC_URL: "V13-R2-PUBLIC-URL", Review, "R2 public URLs",
        "`url(key)` on R2 storage now returns `StorageError::Unsupported` instead of an unsigned S3 API URL",
        "Build public object URLs from the application's r2.dev or custom domain, or use a signed download with the `storage-s3` feature; handle `StorageError::Unsupported`.";
    OTLP_ENVIRONMENT: "V13-OTLP-ENVIRONMENT", Review, "Distributed tracing",
        "the legacy OTLP initializer exports over HTTP to 127.0.0.1:4318; review collector endpoint variables",
        "`OTEL_EXPORTER_OTLP_ENDPOINT` is a base URL and `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT` is exact; check the configured collector, proxy variables and the 64-span batch limit.";
    QUEUE_SEMANTICS: "V13-QUEUE-SEMANTICS", Review, "Core queue",
        "queue claims, retries and timeouts changed (attempts kept on retry, zero timeouts and empty names rejected)",
        "`retry_failed_job` keeps the attempt counter, `job_timeout(Duration::ZERO)` and empty or 256+ byte job names return `QueueError::InvalidConfiguration`; adjust tests and configuration.";
    PRESENCE_COUNTING: "V13-PRESENCE-COUNTING", Review, "Core scheduler, cache and realtime",
        "`PresenceTracker` now counts connections: call `user_left` once per `user_joined`",
        "Make every `user_joined` pair with exactly one `user_left`; a user stays online until the last connection leaves.";
    QUEUE_DRIVER_PREVIEWS: "V13-QUEUE-DRIVER-PREVIEWS", Review, "Studio queue monitor",
        "Studio lists jobs with `QueueDriver::list_job_previews`; custom drivers can override it to bound payloads",
        "Optionally override `list_job_previews` in the custom driver to cut payloads and errors in the store; the default projection keeps working.";
    UTOIPA_6: "V13-STUDIO-UTOIPA-6", MustChange, "Studio API playground",
        "`Studio::with_openapi` now takes a utoipa 6 `OpenApi`; this workspace declares an older utoipa",
        "Upgrade `utoipa` to 6 (and `utoipa-axum` to 0.3, `utoipa-swagger-ui` to 10 when used) in Cargo.toml and adapt the OpenAPI derives.";
    STUDIO_TABLE_VALUES: "V13-STUDIO-TABLE-VALUES", Review, "Studio local access and table view",
        "`get_any_value_as_string` reports undecodable values as `unreadable` instead of `NULL`",
        "Update callers or tests that expected `NULL` for non-UTF-8 BLOBs or a text `NULL`.";
    HTML_EVENT_HANDLER: "V13-HTML-DYNAMIC-EVENT-HANDLER", MustChange, "`html!` event handlers",
        "a dynamic value in an `on*`/`hx-on*` attribute of `html!` no longer compiles",
        "Use a static handler string, or move the handler into a nonce'd script with `addEventListener` and pass the value through a `data-*` attribute.";
    VALIDATION_STATUS: "V13-VALIDATION-STATUS", Review, "Core request validation",
        "`ValidatedForm`/`ValidatedJson` now answer 413/415 for oversized or wrong-type bodies and key nested errors by path",
        "Update clients and tests that expected 400 for these cases and nested error keys such as `address.zip` or `items[0].name`.";
    SCHEDULER_WEEKDAYS: "V13-SCHEDULER-WEEKDAYS", Review, "Core feature flags and scheduler",
        "`Scheduler::task` uses POSIX weekday numbering (0 and 7 are Sunday) in UTC",
        "Review numeric weekdays in cron expressions; `Scheduler::start` rejects a zero task timeout.";
    MEMORY_FEATURE_SPLITS: "V13-MEMORY-FEATURE-SPLITS", Review, "Memory feature-flag splits",
        "A/B override flags no longer report `enabled`/`enabled_for` as true for everyone",
        "Gate A/B code on `variant`; `enabled_for` is true only for identifiers assigned a variant named `enabled`.";
    DB_FEATURE_SPLITS: "V13-DB-FEATURE-SPLITS", Review, "Database feature-flag splits",
        "a database flag holding an A/B split now makes `enabled` false",
        "Gate A/B code on `variant` instead of `enabled`; a negative `rollout_percentage` means 0%.";
    REFERRER_POLICY: "V13-REFERRER-NO-REFERRER", Review, "Security headers",
        "header layers now preserve an endpoint's exact `Referrer-Policy: no-referrer`",
        "Confirm the endpoint intends the more restrictive policy; other endpoint values still yield to the baseline.";
    GEMINI_STOP_REASONS: "V13-AI-GEMINI-STOP-REASONS", Review, "AI provider streaming and stop reasons",
        "Gemini now streams and fails with `AiError::ApiError` on `MAX_TOKENS` or safety stop reasons",
        "Handle `AiError::ApiError` for truncated or blocked answers and raise the output limit where needed.";
    RAG_TENANT_TAGS: "V13-AI-RAG-TENANT-TAGS", Review, "AI RAG tenant tags",
        "`RagPipeline::answer` fails with `RagError::InvalidDocument` when a retrieved document belongs to another tenant",
        "Make the retriever return only the request tenant's documents and handle `RagError::InvalidDocument`.";
    ANTHROPIC_OUTPUT: "V13-AI-ANTHROPIC-OUTPUT", Review, "Anthropic provider output",
        "Anthropic requests 16,000 output tokens and fails on `max_tokens`/`refusal` stop reasons",
        "Choose a limit with `with_max_tokens`, raise `with_request_timeout` for long replies and handle `AiError::ApiError`.";
    OPENAI_OUTPUT: "V13-AI-OPENAI-OUTPUT", Review, "OpenAI provider output",
        "OpenAI replies cut by `length` or `content_filter` now fail with `AiError::ApiError`",
        "Handle `AiError::ApiError` instead of using partial text; vision requests no longer send `max_tokens`.";
    CHAT_MEMORY_KEYS: "V13-AI-CHAT-MEMORY-KEYS", Review, "AI chat memory keys on MySQL/MariaDB",
        "`SqlChatMemory` IDs are case-sensitive; MySQL/MariaDB tables created by 12.x fail closed for IDs differing only by case",
        "On MySQL/MariaDB stop chat-memory writers, back up both tables and apply the `ascii_bin` migration from the AI README.";
    MACHINE_ENDPOINTS: "V13-HOT-RELOAD-MACHINE-ENDPOINTS", Review, "Hot-reload machine endpoints",
        "the hot-reload server now authenticates `with_machine_endpoints` routes",
        "Make development machine clients send the configured credentials.";
    PII_MASKING: "V13-PII-MASKING", Review, "Core PII masking of range responses",
        "PII masking now decodes JSON escapes and withholds masked 206/multipart range responses as 502",
        "Review clients that request byte ranges of masked responses and tests that compare masked JSON (see also the Core JSON PII masking row).";
    SECURITY_LAYERS: "V13-SECURITY-DLP-HONEYPOT", Review, "Security DLP, RASP and honeypot",
        "DLP masks XML/YAML/JS bodies and range responses; RASP, honeypot and `redact_secrets` telemetry changed",
        "Review clients comparing such bodies byte for byte, honeypot ban TTLs and dashboards built on `dlp_secrets_masked`.";
    AUDIT_LOG_LINES: "V13-SECURITY-AUDIT-LOG-LINES", Review, "Security audit log lines",
        "`StdoutAuditLogger` quotes and escapes `actor`, `action` and `resource`",
        "Update log parsers and alerts that match the unquoted `actor=<value>` form.";
    OIDC_OPTIONAL_NAME: "V13-OIDC-OPTIONAL-NAME", Review, "Connect generic OIDC",
        "`OidcProvider` accepts tokens without `name`; `ConnectUser::name` may now be empty",
        "Handle an empty `ConnectUser::name` (fall back to e-mail or another claim) wherever a display name is required.";
    ANDROID_RELEASE: "V13-ANDROID-RELEASE-SIGNING", Review, "Android release command",
        "`omni android --release` now requires the expected signing certificate and a trusted `apksigner.jar`",
        "Set `RULLST_ANDROID_SIGNING_CERTIFICATE` and `RULLST_ANDROID_APKSIGNER_JAR` (or the CLI options) in the release job.";
    TS_CLIENT_REGENERATE: "V13-TS-CLIENT-REGENERATE", Review, "CLI generator robustness",
        "regenerate `rullst-client.ts`: v13 clients send the CSRF header and URL-encode path parameters",
        "Run `cargo rullst generate:ts` and update callers of parameterized methods; older clients receive 403 from `csrf_middleware`.";
    GITIGNORE_DATABASES: "V13-GITIGNORE-DATABASES", Review, "CLI generator robustness",
        "`.gitignore` ignores SQLite databases but not the journal files that new projects ignore",
        "Add `*.sqlite-shm`, `*.sqlite-wal`, `*.duckdb` and `*.duckdb.wal` to `.gitignore`.";
    ORM_PARTIAL_UPDATE: "V13-ORM-PARTIAL-UPDATE", Review, "ORM partial updates",
        "`update_partial()` now merges into a freshly loaded row and runs full-save hooks, audit and SQL",
        "Review audit identity, triggers, unsaved local fields and outer rollback; use `save_with_tx` for explicit transactions.";
    ORM_BELONGS_TO_KEY: "V13-ORM-BELONGS-TO-KEY", MustChange, "ORM derive checks",
        "`belongs_to` without `foreign_key` now joins on `<related model>_id` instead of `<this model>_id`",
        "Add an explicit `foreign_key = \"...\"` naming the column this relation used before (or the intended one).";
    ORM_IGNORED_KEY: "V13-ORM-IGNORED-RELATION-KEY", MustChange, "ORM derive checks",
        "this relation ignores `local_key`/`related_key`; the v13 derive rejects the option",
        "Remove `local_key` from `belongs_to`/`morph_to` and `related_key` from `has_one`/`has_many`/`morph_one`/`morph_many`.";
    ORM_SEARCHABLE_TABLE: "V13-ORM-SEARCHABLE-TABLE", MustChange, "ORM derive checks",
        "`searchable` models need a lowercase Scout-valid table name",
        "Set `#[orm(table = \"...\")]` to a name that starts with a lowercase letter and uses only lowercase letters, digits and `_`.";
    ORM_JSON_SERIALIZE: "V13-ORM-SQLX-JSON-SERIALIZE", Review, "ORM queries and cache",
        "`#[sqlx(json)]` fields are now bound as SQLx `Json`, so the field type needs `Serialize`",
        "Derive or implement `serde::Serialize` for the field type; count/paginate totals and chunk ordering also changed.";
    ORM_CACHE_KEY: "V13-ORM-QUERY-CACHE-KEY", Review, "ORM queries and cache",
        "query-cache keys of models without `tenant_column` no longer include the tenant",
        "Rebuild manual `query_cache::query_key` calls the way generated keys are built for the model.";
    ORM_CACHE_PREFIX: "V13-ORM-CACHE-PREFIX", Review, "ORM query-cache index",
        "`.remember(...)` keys move to `rullst:orm:cache:v4:` with a sorted-set index; caches start cold",
        "Expect cold caches after deploying, avoid mixed-version writes during a rolling upgrade and update tools that read the old `:keys` set.";
    ORM_REDIS_HASHES: "V13-ORM-REDIS-HASHES", Review, "ORM Redis model hashes",
        "generated Redis model hashes moved from `orm:<table>:<id>` to namespaced keys; tenant models need a one-time migration",
        "Global models migrate on their next write; after deploying, migrate tenant model hashes with the Redis guide procedure and stop 12.x writers of these hashes before 13 instances write them.";
    OUTBOX_MYSQL_KEYS: "V13-OUTBOX-MYSQL-KEYS", Review, "ORM outbox keys on MySQL/MariaDB",
        "outbox keys are case-sensitive; a MySQL/MariaDB `rullst_outbox` table created by 12.x keeps a case-insensitive collation",
        "On MySQL/MariaDB run `Outbox::install()` once (for example from a new migration) to convert the key columns to `ascii_bin`.";
    AUDIT_PAYLOADS: "V13-ORM-AUDIT-PAYLOADS", Review, "ORM audit payloads on MySQL/MariaDB",
        "a MySQL/MariaDB `rullst_audits` table created by 12.x keeps 64 KiB `TEXT` payload columns",
        "On MySQL/MariaDB apply `ALTER TABLE rullst_audits MODIFY old_values LONGTEXT, MODIFY new_values LONGTEXT, MODIFY restore_patch LONGTEXT` in a maintenance window.";
    TURSO_ROLLBACK: "V13-TURSO-ROLLBACK-DRIFT", Review, "Turso migrations",
        "Turso `rollback_last` refuses a migration whose recorded digest differs",
        "Restore the applied migration definition before rolling it back.";
    SCHEMA_TABLE_CASE: "V13-SCHEMA-PG-TABLE-CASE", Review, "Schema table names on PostgreSQL",
        "`Schema::create`/`drop_if_exists` lower-case quoted table names on PostgreSQL",
        "On PostgreSQL, rename existing mixed-case tables (`ALTER TABLE \"Name\" RENAME TO name`) or use a lowercase name.";
    SECRET_STRING_INPUT: "V13-SECRET-STRING-CLIENT-INPUT", Review, "`SecretString` client input",
        "a `SecretString` deserialized from client input still decrypts any envelope under the configured key",
        "Add `#[serde(deserialize_with = \"rullst_orm::privacy::deserialize_plaintext_secret\")]` (or the optional variant) to fields filled from requests.";
    PROTECTED_VALUES: "V13-ORM-PROTECTED-VALUES", Review, "ORM protected values and `SecretString` serialization",
        "encrypted and masked values are `***` in audit rows and events, and `SecretString` serializes as an encrypted envelope",
        "Configure `RULLST_ENCRYPTION_KEY` where `SecretString` is serialized, purge or reindex 12.x audit rows and search documents that may hold plaintext, and upgrade readers of serialized secrets before writers.";
    REDIS_MOCK_ORDER: "V13-REDIS-MOCK-TIE-ORDER", Review, "Offline Redis mock",
        "the offline Redis mock returns equal `sorted_set_top` scores in descending member order",
        "Update tests that relied on ascending tie order.";
    AUTO_HEALING: "V13-AUTO-HEALING-DIAGNOSTICS", Review, "Auto-healing diagnostics",
        "`diagnose_sql_error` returns `None` for more messages and uses driver-specific DDL",
        "Handle `None` and review assertions on suggested DDL.";
    ORM_MISSING_ROW: "V13-ORM-MISSING-ROW", Review, "ORM instance mutations",
        "deleting, force-deleting or saving a missing row now fails with `RecordNotFound`",
        "Accept `RecordNotFound` where deletes were idempotent; `restore()` of a live row is a no-op.";
    ORM_SANDBOX_TEST: "V13-ORM-SANDBOX-TEST", Review, "ORM sandbox tests",
        "`#[rullst_orm::test]` now keeps the declared return type; an `Err` result fails the test",
        "Make the signature match the body (a `Termination` type such as `Result<(), E: Debug>`) and assert post-commit effects in a committing test.";
    ORM_SQLX_STRUCT_OPTION: "V13-ORM-SQLX-STRUCT-OPTION", MustChange, "ORM model SQLx options",
        "a struct-level `#[sqlx(...)]` option other than `default` on a `#[derive(Orm)]` model no longer compiles",
        "Remove the option (for example `rename_all`) and name the fields after their columns.";
    ORM_ONLY_TRASHED: "V13-ORM-ONLY-TRASHED", Review, "ORM trashed scopes",
        "`only_trashed()` on a model without soft deletes now fails with `Validation`",
        "Remove the call or give the model a `deleted_at` field or `#[orm(soft_delete)]`.";
    ORM_TRASHED_DELETE_ALL: "V13-ORM-TRASHED-DELETE-ALL", Review, "ORM bulk soft deletes",
        "`delete_all()` with `with_trashed()`/`only_trashed()` now fails with `Validation` on soft-delete models",
        "Bulk-delete live rows without these modifiers and purge trashed rows with `force_delete()`.";
    ORM_SQL_TEXT: "V13-ORM-SQL-TEXT", Review, "ORM joined scopes",
        "generated tenant, soft-delete and keyset predicates are now qualified with the table name",
        "Update tests that compare generated SQL text.";
    ORM_CHUNK_BOUNDS: "V13-ORM-CHUNK-BOUNDS", Review, "ORM chunk bounds",
        "chunked queries now honour an explicit `limit`/`offset` instead of ignoring them",
        "Remove `limit()`/`offset()` from chunked queries that relied on them being ignored.";
    PERSONAL_DATA_REPORT: "V13-PERSONAL-DATA-REPORT", Review, "PersonalData reports",
        "`compliance_schema()` reports the ORM table and lists only encrypted columns as encrypted",
        "Use `ComplianceModel::personal_fields()` for `#[privacy]` fields and review consumers of the report.";
    NEXUS_PRIMARY_KEY: "V13-NEXUS-PRIMARY-KEY", MustChange, "Nexus derive",
        "two `#[nexus(primary_key)]` declarations, or one contradicting the struct-level key, no longer compile",
        "Keep exactly one primary-key declaration.";
    NEXUS_FIELD_KIND_NUMBER: "V13-NEXUS-FIELD-KIND-NUMBER", Review, "Nexus derive",
        "derived integer fields are now `FieldKind::Integer { min, max }` instead of `FieldKind::Number`",
        "Expect `FieldKind::Integer` when comparing derived metadata; manual `FieldMeta` values keep their kind.";
    NEXUS_FIELD_KIND_MATCH: "V13-NEXUS-FIELD-KIND-MATCH", MustChange, "Nexus panel",
        "`FieldKind` is now `#[non_exhaustive]`; a `match` without a wildcard arm no longer compiles",
        "Add a wildcard arm (`_ => ...`) to the match.";
    NEXUS_DOTENV: "V13-NEXUS-DOTENV", Review, "Nexus credentials",
        "Nexus reads `NEXUS_ADMIN_*` from `.env` without loading it into the process environment",
        "Read other settings yourself (for example with `rullst::config::project_setting`) and handle `NexusBuildError::InvalidDotenv`.";
    DEV_TELEMETRY_ROUTE: "V13-DEV-TELEMETRY-ROUTE", Review, "Core development telemetry",
        "`/_rullst/*` paths are reserved for development routes such as `/_rullst/dev-telemetry`",
        "Move application routes away from `/_rullst/`.";
    FOUNDRY_SERVICE: "V13-FOUNDRY-SERVICE", Review, "CLI Foundry service environment",
        "`foundry:deploy` now sets `PORT`/`HOST`, uploads the reported executable and runs a dedicated service account",
        "Keep `[env] PORT` equal to `[app] port`, set `package.default-run` for several binaries and keep writable state under `/opt/rullst/<app>/data`.";
    MFA_CLIENT_SECRET: "V13-MFA-CLIENT-SECRET", Review, "CLI generators",
        "an MFA handler generated before v13 verified a client-supplied `secret`",
        "Replace it with the server-side factor store that `cargo rullst make:mfa` now generates.";
    WISE_WEBHOOK: "V13-CAPITAL-WEBHOOKS", Review, "Capital provider webhooks",
        "Wise `parse_webhook_payload` accepts only `mock_*` tokens; live deliveries need signature verification",
        "Configure `with_webhook_public_key_pem` and use `verify_transfer_state_change` for live Wise deliveries.";
    QUOTA_KEYS: "V13-CAPITAL-QUOTA-KEYS", Review, "Capital quota keys on MySQL/MariaDB",
        "`SqlQuotaStore` keys are case-sensitive; older MySQL/MariaDB tables need an `ALTER TABLE`",
        "On MySQL/MariaDB run the `ascii_bin` migration from the Capital README before serving quota calls.";
    ZERO_TIER: "V13-CAPITAL-ZERO-TIER", Review, "Capital zero tier limit",
        "a `tier_limit` of `Some(0)` now returns `QuotaError::LimitExceeded` instead of `InvalidRequest`",
        "Map it like any other quota denial (402/403 with an upgrade prompt).";
    OUTBOX_RELAY_KEY: "V13-OUTBOX-RELAY-KEY", Review, "Messaging outbox relay key",
        "`OrmOutboxRelay` publishes under a stream-scoped idempotency key",
        "Drain or acknowledge in-flight outbox claims before upgrading, or rely on consumer idempotency.";
    MAIL_FACADE: "V13-MAIL-FACADE-CONFIG", Review, "Mail sender",
        "the `Mail` facade needs `MAIL_FROM`, and `MAIL_DRIVER` in staging/production; invalid `MAIL_PORT` now fails",
        "Set `MAIL_FROM` and `MAIL_DRIVER` (see also the Mail driver default and Mail facade settings rows).";
    MAIL_QUEUED_ATTACHMENTS: "V13-MAIL-QUEUED-ATTACHMENTS", Review, "Mail queued attachments",
        "queued mail stores attachment bytes as base64, which 12.x mail workers cannot read",
        "In a rolling deployment upgrade every mail worker before any producer, retry jobs a 12.x worker failed once 13 workers run and drain the queue before rolling back.";
    MAIL_ATTACHMENTS: "V13-MAIL-ATTACHMENT-INSPECTION", Review, "Mail attachment inspection",
        "`LocalAttachmentInspector` classifies XML by root element and namespaces",
        "Review XML attachments that embed SVG/XHTML namespaces or entity declarations.";
    MAIL_RESEND_SCHEDULE: "V13-MAIL-RESEND-SCHEDULE", Review, "Mail Resend scheduling",
        "direct `ResendDriver` sends scheduled more than 30 days ahead now fail before the request",
        "Schedule such mail through a durable queue with `Mail::enqueue`.";
    MAIL_TRACKING: "V13-MAIL-TRACKING-RECIPIENT", Review, "Mail tracking recipient",
        "open/click tracking now signs the bare recipient address",
        "Compare `OpenEvent.email`/`ClickEvent.email` with the bare address instead of the display-name form.";
    MAIL_TEXT_FALLBACK: "V13-MAIL-TEXT-FALLBACK", Review, "Mail plain-text fallback",
        "the derived plain-text part now appends link targets as `label <URL>`",
        "Update tests comparing derived text, or set `text` explicitly.";
    LABS_RUNNER: "V13-LABS-RUNNER-REMOVED", MustChange, "Labs runner",
        "the `rullst-labs-runner` candidate was removed from v13 and is not published",
        "Remove the dependency and deploy an application-owned runner against the labs controller contract.";
    BILLING_SETTINGS: "V13-BILLING-PROJECT-SETTINGS", Review, "Generated billing settings",
        "generated billing code read `BILLING_*` from the process environment only",
        "Read `BILLING_*` with `rullst::config::project_setting` and check production with `RullstConfig::global().environment()`.";
    SQLITE_SEED_TIME: "V13-SQLITE-ONLY-SEED-TIME", Review, "Starter migrations",
        "this seed uses SQLite-only `datetime('now')`, which PostgreSQL, MySQL and MariaDB reject",
        "Bind a UTC timestamp from Rust (or use `CURRENT_TIMESTAMP`) and advance PostgreSQL sequences after explicit ids.";
    ERP_DASHBOARD_ACCESS: "V13-ERP-DASHBOARD-ACCESS", Review, "ERP starter access",
        "`GET /` is served outside the router that `protect_router` guards",
        "If `/` lists customers or revenue, move it into the routes passed to `admin_access.protect_router`.";
    LMS_PROGRESS_KEY: "V13-LMS-PROGRESS-KEY", Review, "LMS lesson progress",
        "a fixed LMS progress idempotency key makes later saves return 409",
        "Copy the new `new_progress_key`/`progress_event_key` helpers and scope keys per learner.";
    LMS_RECORD_PROGRESS: "V13-LMS-RECORD-PROGRESS", Review, "LMS concurrent progress saves",
        "`record_progress` should claim the idempotency key with its first write",
        "Copy the service's `replay` helper and `record_progress` from a new LMS project.";
    LMS_MEDIA_KINDS: "V13-LMS-MEDIA-KINDS", Review, "LMS lesson media in Nexus",
        "lesson `media_url`/`captions_url` are text fields in v13 so same-origin captions stay editable",
        "Make those Nexus fields `FieldKind::Text` and `media_kind` an `Enum` of `video`/`audio`.";
    BLOG_SITEMAP: "V13-BLOG-ROBOTS-SITEMAP", Review, "Blog robots and sitemap",
        "`robots.txt`/`sitemap.xml` advertise relative URLs that crawlers reject",
        "Copy `public_origin`, `path_segment`, `robots_txt` and `sitemap_xml` from a new Blog project and set `RULLST_PUBLIC_ORIGIN`.";
    CREDENTIAL_RATE_LIMIT: "V13-CREDENTIAL-RATE-LIMIT", Review, "Generated login and registration",
        "password hashing in this handler has no `credential_rate_limit` or bounded Argon2 permits",
        "Copy the bounded hashing and `credential_rate_limit` changes and apply them to `POST /login` and `/register`.";
    PASSWORD_HASH_HIDDEN: "V13-PASSWORD-HASH-HIDDEN", Review, "SaaS user JSON",
        "`password_hash` is not `#[orm(hidden)]`, so `to_json()`, audit rows and model events carry the hash",
        "Add `#[orm(hidden)]` to the field.";
    EMPTY_TIMESTAMPS: "V13-EMPTY-TIMESTAMPS", Review, "Registration timestamps",
        "an empty `created_at`/`updated_at` replaces the column default because the ORM inserts every field",
        "Store the current UTC time (`YYYY-MM-DD HH:MM:SS`) instead, as the new `utc_timestamp` helper does (see also Billing row timestamps).";
    UTF16_LENGTHS: "V13-UTF16-LENGTHS", Review, "Registration lengths",
        "registration limits are counted in UTF-8 bytes; v13 counts the forms' UTF-16 code units",
        "Copy `form_length` and `valid_password`: 120-character names and 12-character passwords in UTF-16 units, at most 72 password bytes.";
    VPS_PROXY: "V13-VPS-DEPLOY-PROXY", Review, "VPS deploy proxy",
        "`docker-compose.prod.yml` lacks the pinned `edge` network that lets the app trust Caddy",
        "Add the `edge` network (`172.31.250.0/24`, Caddy at `172.31.250.10`) and that address to `[security] trusted_proxies`.";
    MODEL_ALL: "V13-MODEL-ALL", Review, "Blog and ERP reads",
        "`Model::all()` stops at the ORM's 1,000-row cap without an `ORDER BY`",
        "Page with `paginate` (newest first) or read one row with `query().first()`; return 503 on database errors (see also Blank database status).";
    PROFILE_FIND: "V13-PORTFOLIO-PROFILE", Review, "Portfolio profile",
        "the Portfolio page hard-codes `Profile::find(1)`",
        "Show the first profile by id and answer 404 when none exists, as the new controller does.";
    PAGE_CSP_NONCE: "V13-PAGE-CSP-NONCE", Review, "Blog and Portfolio pages",
        "this page renders without the request's CSP nonce, so the production CSP blocks its inline styles",
        "Pass the request's `CspNonce` into the page, replace `style` attributes with classes and serve assets same-origin.";
    LINKER_CONFIG: "V13-LINKER-CONFIG", Review, "Generated linker configuration",
        "`.cargo/config.toml` selects a host linker but is not ignored by Git",
        "List `.cargo/config.toml` in `.gitignore` and `.dockerignore`, or delete its `-fuse-ld` flags before CI or Docker builds.";
    ERP_STORE_ORDER: "V13-ERP-STORE-ORDER", Review, "ERP orders and stock",
        "the ERP order handler reserves stock with a read-modify-write",
        "Copy the transactional `store_order`/`add_stock` controller with its 404/409/422/503 answers.";
    PAGE_BOUNDS: "V13-PAGE-BOUNDS", Review, "Blog and ERP page bounds",
        "`paginate` receives an unbounded `?page=` value",
        "Answer 404 for a page above a `MAX_PAGE` bound (for example 100,000) before calling `paginate`.";
    DOCKER_CONTEXT: "V13-DOCKER-CONTEXT", Review, "Docker build context",
        "`.dockerignore` does not exclude `.env.*`, `Foundry.toml` or local database files",
        "Add `.env.*` (keeping `!.env.example`), `Foundry.toml`, `*.sqlite`, `*.sqlite3`, `*.duckdb` and their journal files.";
    K8S_INGRESS_TLS: "V13-K8S-INGRESS-TLS", Review, "Kubernetes Ingress",
        "this Ingress uses the deprecated class annotation and may serve plain HTTP",
        "Use `ingressClassName: nginx` and add a `tls` block for the real host.";
    HEALTH_PROBES: "V13-HEALTH-PROBES", Review, "Starter health probes",
        "the server does not mount `rullst::health::health_router()`, so `/health` and `/ready` probes answer 404",
        "Merge `rullst::health::health_router()` (or `health_router_with_lifecycle`) outside any authentication layer.";
    K8S_NAMES: "V13-K8S-NAMES", Review, "Kubernetes and Buildah names",
        "the package name is not a lowercase DNS label; v13 derives Kubernetes and image names differently",
        "Regenerate the manifests or Buildah script, or rename the objects and image.";
    ORM_DEFAULT_FEATURES: "V13-ORM-DEFAULT-FEATURES", Review, "Generated `rullst-orm` dependency",
        "`rullst-orm` keeps its default `drivers-all`, adding every SQLx driver to a single-backend build",
        "Add `default-features = false` to the `rullst-orm` dependency and enable any other driver explicitly.";
    OMNI_RUNNER: "V13-OMNI-RUNNER", Review, "Omni desktop runner",
        "the Omni runtime does not print `Launching Omni interface...` before opening its window",
        "Add `println!(\"Launching Omni interface...\");` before `tauri::Builder::default()` in the desktop branch.";
    OMNI_BACKEND: "V13-OMNI-BACKEND", Review, "Omni managed backend",
        "the Omni runtime chooses `cargo run` from a relative `../Cargo.toml`",
        "Resolve the project from `CARGO_MANIFEST_DIR` in debug builds and start the `server` executable in release builds.";
    PAGE_LANGUAGE: "V13-RENDER-PAGE-LANGUAGE", Review, "Starter page language",
        "`render_page` declares `lang=\"pt-BR\"`; English pages should use `render_page_with_lang`",
        "Call `rullst::htmx::render_page_with_lang(&htmx, \"en\", title, content)` with the page's language.";
}
