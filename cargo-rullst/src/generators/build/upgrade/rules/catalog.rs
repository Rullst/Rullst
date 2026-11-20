//! The v12 → v13 source rules. Each rule names the first-column title of its
//! row in `docs/src/migration-v13.md`; the classification of every row is
//! reviewed in the assisted-upgrade tutorial.
//!
//! Each rule records the release that introduced the change it reports: the
//! v12 → v13 rules date from 13.0.0-alpha.1, and a rule added or reworded for a
//! later change (`V13-ORM-DEFAULT-FEATURES` in alpha.3) carries that release.
//! To date one, look for its code in this file at each release tag
//! (`git show v13.0.0-alpha.2:<this file>` or `git log -S'"V13-…"'`) and read
//! the CHANGELOG entry that names it; a new rule takes the next release.

use super::{FindingKind, Rule};

#[path = "catalog/removed.rs"]
mod removed;
pub(crate) use removed::*;

use FindingKind::{MustChange, Review};

macro_rules! rules {
    ($($name:ident: $code:literal, $kind:ident, $introduced:literal, $row:literal, $message:literal, $guidance:literal;)+) => {
        $(pub(crate) const $name: Rule = Rule {
            code: $code,
            kind: $kind,
            introduced: $introduced,
            row: $row,
            message: $message,
            guidance: $guidance,
        };)+
        /// Every rule, in catalog order.
        pub(crate) const CATALOG: &[&Rule] = &[$(&$name),+];
    };
}

rules! {
    AUTH_RECOVERY_MIGRATE: "V13-AUTH-RECOVERY-MIGRATE", Review, "13.0.0-alpha.1", "Account/session registry",
        "run `SqlRecoveryStore::migrate()` before serving requests; the upgrade does not apply this SQL migration",
        "Call `SqlRecoveryStore::migrate()` once at start-up before requests are served, configure expired-session maintenance and authorize the new session inventory/logout routes.";
    R2_PUBLIC_URL: "V13-R2-PUBLIC-URL", Review, "13.0.0-alpha.1", "R2 public URLs",
        "`url(key)` on R2 storage now returns `StorageError::Unsupported` instead of an unsigned S3 API URL",
        "Build public object URLs from the application's r2.dev or custom domain, or use a signed download with the `storage-s3` feature; handle `StorageError::Unsupported`.";
    OTLP_ENVIRONMENT: "V13-OTLP-ENVIRONMENT", Review, "13.0.0-alpha.1", "Distributed tracing",
        "the legacy OTLP initializer exports over HTTP to 127.0.0.1:4318; review collector endpoint variables",
        "`OTEL_EXPORTER_OTLP_ENDPOINT` is a base URL and `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT` is exact; check the configured collector, proxy variables and the 64-span batch limit.";
    QUEUE_SEMANTICS: "V13-QUEUE-SEMANTICS", Review, "13.0.0-alpha.1", "Core queue",
        "queue claims, retries and timeouts changed (attempts kept on retry, zero timeouts and empty names rejected)",
        "`retry_failed_job` keeps the attempt counter, `job_timeout(Duration::ZERO)` and empty or 256+ byte job names return `QueueError::InvalidConfiguration`; adjust tests and configuration.";
    PRESENCE_COUNTING: "V13-PRESENCE-COUNTING", Review, "13.0.0-alpha.1", "Core scheduler, cache and realtime",
        "`PresenceTracker` now counts connections: call `user_left` once per `user_joined`",
        "Make every `user_joined` pair with exactly one `user_left`; a user stays online until the last connection leaves.";
    QUEUE_DRIVER_PREVIEWS: "V13-QUEUE-DRIVER-PREVIEWS", Review, "13.0.0-alpha.1", "Studio queue monitor",
        "Studio lists jobs with `QueueDriver::list_job_previews`; custom drivers can override it to bound payloads",
        "Optionally override `list_job_previews` in the custom driver to cut payloads and errors in the store; the default projection keeps working.";
    STUDIO_TABLE_VALUES: "V13-STUDIO-TABLE-VALUES", Review, "13.0.0-alpha.1", "Studio local access and table view",
        "`get_any_value_as_string` reports undecodable values as `unreadable` instead of `NULL`",
        "Update callers or tests that expected `NULL` for non-UTF-8 BLOBs or a text `NULL`.";
    HTML_EVENT_HANDLER: "V13-HTML-DYNAMIC-EVENT-HANDLER", MustChange, "13.0.0-alpha.1", "`html!` event handlers",
        "a dynamic value in an `on*`/`hx-on*` attribute of `html!` no longer compiles",
        "Use a static handler string, or move the handler into a nonce'd script with `addEventListener` and pass the value through a `data-*` attribute.";
    HTML_MARKUP_RETURN: "V13-HTML-MARKUP-RETURN", MustChange, "13.0.0-alpha.2", "`html!` returns `Markup`",
        "`html!` now returns `rullst::html::Markup`; a function declared to return a `String` built by `html!` no longer compiles",
        "Declare the function `-> Markup` (`use rullst::html::Markup;`) so other `html!` calls insert it without `RawHtml`; call `.into_string()` only where a `String` is really needed.";
    HTML_RAW_MARKUP: "V13-HTML-RAW-MARKUP", MustChange, "13.0.0-alpha.2", "`html!` returns `Markup`",
        "`RawHtml` wraps `html!` output, which is now `Markup` and is inserted into other `html!` calls without it",
        "Interpolate the `Markup` directly (`{badge}`), collect fragments with `.collect::<Markup>()` and keep `RawHtml` only for trusted HTML produced outside `html!`.";
    VALIDATION_STATUS: "V13-VALIDATION-STATUS", Review, "13.0.0-alpha.1", "Core request validation",
        "`ValidatedForm`/`ValidatedJson` now answer 413/415 for oversized or wrong-type bodies and key nested errors by path",
        "Update clients and tests that expected 400 for these cases and nested error keys such as `address.zip` or `items[0].name`.";
    SCHEDULER_WEEKDAYS: "V13-SCHEDULER-WEEKDAYS", Review, "13.0.0-alpha.1", "Core feature flags and scheduler",
        "`Scheduler::task` uses POSIX weekday numbering (0 and 7 are Sunday) in UTC",
        "Review numeric weekdays in cron expressions; `Scheduler::start` rejects a zero task timeout.";
    MEMORY_FEATURE_SPLITS: "V13-MEMORY-FEATURE-SPLITS", Review, "13.0.0-alpha.1", "Memory feature-flag splits",
        "A/B override flags no longer report `enabled`/`enabled_for` as true for everyone",
        "Gate A/B code on `variant`; `enabled_for` is true only for identifiers assigned a variant named `enabled`.";
    DB_FEATURE_SPLITS: "V13-DB-FEATURE-SPLITS", Review, "13.0.0-alpha.1", "Database feature-flag splits",
        "a database flag holding an A/B split now makes `enabled` false",
        "Gate A/B code on `variant` instead of `enabled`; a negative `rollout_percentage` means 0%.";
    REFERRER_POLICY: "V13-REFERRER-NO-REFERRER", Review, "13.0.0-alpha.1", "Security headers",
        "header layers now preserve an endpoint's exact `Referrer-Policy: no-referrer`",
        "Confirm the endpoint intends the more restrictive policy; other endpoint values still yield to `SecureHeadersLayer`, while Core now keeps them.";
    GEMINI_STOP_REASONS: "V13-AI-GEMINI-STOP-REASONS", Review, "13.0.0-alpha.1", "AI provider streaming and stop reasons",
        "Gemini now streams and fails with `AiError::ApiError` on `MAX_TOKENS` or safety stop reasons",
        "Handle `AiError::ApiError` for truncated or blocked answers and raise the output limit where needed.";
    RAG_TENANT_TAGS: "V13-AI-RAG-TENANT-TAGS", Review, "13.0.0-alpha.1", "AI RAG tenant tags",
        "`RagPipeline::answer` fails with `RagError::InvalidDocument` when a retrieved document belongs to another tenant",
        "Make the retriever return only the request tenant's documents and handle `RagError::InvalidDocument`.";
    ANTHROPIC_OUTPUT: "V13-AI-ANTHROPIC-OUTPUT", Review, "13.0.0-alpha.1", "Anthropic provider output",
        "Anthropic requests 16,000 output tokens and fails on `max_tokens`/`refusal` stop reasons",
        "Choose a limit with `with_max_tokens`, raise `with_request_timeout` for long replies and handle `AiError::ApiError`.";
    OPENAI_OUTPUT: "V13-AI-OPENAI-OUTPUT", Review, "13.0.0-alpha.1", "OpenAI provider output",
        "OpenAI replies cut by `length` or `content_filter` now fail with `AiError::ApiError`",
        "Handle `AiError::ApiError` instead of using partial text; vision requests no longer send `max_tokens`.";
    CHAT_MEMORY_KEYS: "V13-AI-CHAT-MEMORY-KEYS", Review, "13.0.0-alpha.1", "AI chat memory keys on MySQL/MariaDB",
        "`SqlChatMemory` IDs are case-sensitive; MySQL/MariaDB tables created by 12.x fail closed for IDs differing only by case",
        "On MySQL/MariaDB stop chat-memory writers, back up both tables and apply the `ascii_bin` migration from the AI README.";
    MACHINE_ENDPOINTS: "V13-HOT-RELOAD-MACHINE-ENDPOINTS", Review, "13.0.0-alpha.1", "Hot-reload machine endpoints",
        "the hot-reload server now authenticates `with_machine_endpoints` routes",
        "Make development machine clients send the configured credentials.";
    PII_MASKING: "V13-PII-MASKING", Review, "13.0.0-alpha.1", "Core PII masking of range responses",
        "PII masking now decodes JSON escapes and withholds masked 206/multipart range responses as 502",
        "Review clients that request byte ranges of masked responses and tests that compare masked JSON (see also the Core JSON PII masking row).";
    SECURITY_LAYERS: "V13-SECURITY-DLP-HONEYPOT", Review, "13.0.0-alpha.1", "Security DLP, RASP and honeypot",
        "DLP masks XML/YAML/JS bodies and range responses; RASP, honeypot and `redact_secrets` telemetry changed",
        "Review clients comparing such bodies byte for byte, honeypot ban TTLs and dashboards built on `dlp_secrets_masked`.";
    AUDIT_LOG_LINES: "V13-SECURITY-AUDIT-LOG-LINES", Review, "13.0.0-alpha.1", "Security audit log lines",
        "`StdoutAuditLogger` quotes and escapes `actor`, `action` and `resource`",
        "Update log parsers and alerts that match the unquoted `actor=<value>` form.";
    OIDC_OPTIONAL_NAME: "V13-OIDC-OPTIONAL-NAME", Review, "13.0.0-alpha.1", "Connect generic OIDC",
        "`OidcProvider` accepts tokens without `name`; `ConnectUser::name` may now be empty",
        "Handle an empty `ConnectUser::name` (fall back to e-mail or another claim) wherever a display name is required.";
    CONNECT_GITHUB_EMAIL: "V13-CONNECT-GITHUB-EMAIL", Review, "13.0.0-alpha.3", "Connect GitHub email",
        "the GitHub provider now returns the primary account email and GitHub's verified flag instead of the public profile email",
        "Link or create accounts on a GitHub email only when `email_verified == Some(true)`, and request the `user:email` scope so the primary address can be read.";
    TS_CLIENT_REGENERATE: "V13-TS-CLIENT-REGENERATE", Review, "13.0.0-alpha.1", "CLI generator robustness",
        "regenerate `rullst-client.ts`: v13 clients send the CSRF header and URL-encode path parameters",
        "Run `cargo rullst generate:ts` and update callers of parameterized methods; older clients receive 403 from `csrf_middleware`.";
    GITIGNORE_DATABASES: "V13-GITIGNORE-DATABASES", Review, "13.0.0-alpha.1", "CLI generator robustness",
        "`.gitignore` ignores SQLite databases but not the journal files that new projects ignore",
        "Add `*.sqlite-shm`, `*.sqlite-wal`, `*.duckdb` and `*.duckdb.wal` to `.gitignore`.";
    ORM_PARTIAL_UPDATE: "V13-ORM-PARTIAL-UPDATE", Review, "13.0.0-alpha.1", "ORM partial updates",
        "`update_partial()` now merges into a freshly loaded row and runs full-save hooks, audit and SQL",
        "Review audit identity, triggers, unsaved local fields and outer rollback; use `save_with_tx` for explicit transactions.";
    ORM_BELONGS_TO_KEY: "V13-ORM-BELONGS-TO-KEY", MustChange, "13.0.0-alpha.1", "ORM derive checks",
        "`belongs_to` without `foreign_key` now joins on `<related model>_id` instead of `<this model>_id`",
        "Add an explicit `foreign_key = \"...\"` naming the column this relation used before (or the intended one).";
    ORM_IGNORED_KEY: "V13-ORM-IGNORED-RELATION-KEY", MustChange, "13.0.0-alpha.1", "ORM derive checks",
        "this relation ignores `local_key`/`related_key`; the v13 derive rejects the option",
        "Remove `local_key` from `belongs_to`/`morph_to` and `related_key` from `has_one`/`has_many`/`morph_one`/`morph_many`.";
    ORM_SEARCHABLE_TABLE: "V13-ORM-SEARCHABLE-TABLE", MustChange, "13.0.0-alpha.1", "ORM derive checks",
        "`searchable` models need a lowercase Scout-valid table name",
        "Set `#[orm(table = \"...\")]` to a name that starts with a lowercase letter and uses only lowercase letters, digits and `_`.";
    ORM_JSON_SERIALIZE: "V13-ORM-SQLX-JSON-SERIALIZE", Review, "13.0.0-alpha.1", "ORM queries and cache",
        "`#[sqlx(json)]` fields are now bound as SQLx `Json`, so the field type needs `Serialize`",
        "Derive or implement `serde::Serialize` for the field type; count/paginate totals and chunk ordering also changed.";
    ORM_CACHE_KEY: "V13-ORM-QUERY-CACHE-KEY", Review, "13.0.0-alpha.1", "ORM queries and cache",
        "query-cache keys of models without `tenant_column` no longer include the tenant",
        "Rebuild manual `query_cache::query_key` calls the way generated keys are built for the model.";
    ORM_CACHE_PREFIX: "V13-ORM-CACHE-PREFIX", Review, "13.0.0-alpha.1", "ORM query-cache index",
        "`.remember(...)` keys move to `rullst:orm:cache:v4:` with a sorted-set index; caches start cold",
        "Expect cold caches after deploying, avoid mixed-version writes during a rolling upgrade and update tools that read the old `:keys` set.";
    ORM_REDIS_HASHES: "V13-ORM-REDIS-HASHES", Review, "13.0.0-alpha.1", "ORM Redis model hashes",
        "generated Redis model hashes moved from `orm:<table>:<id>` to namespaced keys; tenant models need a one-time migration",
        "Global models migrate on their next write; after deploying, migrate tenant model hashes with the Redis guide procedure and stop 12.x writers of these hashes before 13 instances write them.";
    OUTBOX_MYSQL_KEYS: "V13-OUTBOX-MYSQL-KEYS", Review, "13.0.0-alpha.1", "ORM outbox keys on MySQL/MariaDB",
        "outbox keys are case-sensitive; a MySQL/MariaDB `rullst_outbox` table created by 12.x keeps a case-insensitive collation",
        "On MySQL/MariaDB run `Outbox::install()` once (for example from a new migration) to convert the key columns to `ascii_bin`.";
    AUDIT_PAYLOADS: "V13-ORM-AUDIT-PAYLOADS", Review, "13.0.0-alpha.1", "ORM audit payloads on MySQL/MariaDB",
        "a MySQL/MariaDB `rullst_audits` table created by 12.x keeps 64 KiB `TEXT` payload columns",
        "On MySQL/MariaDB apply `ALTER TABLE rullst_audits MODIFY old_values LONGTEXT, MODIFY new_values LONGTEXT, MODIFY restore_patch LONGTEXT` in a maintenance window.";
    TURSO_ROLLBACK: "V13-TURSO-ROLLBACK-DRIFT", Review, "13.0.0-alpha.1", "Turso migrations",
        "Turso `rollback_last` refuses a migration whose recorded digest differs",
        "Restore the applied migration definition before rolling it back.";
    SCHEMA_TABLE_CASE: "V13-SCHEMA-PG-TABLE-CASE", Review, "13.0.0-alpha.1", "Schema table names on PostgreSQL",
        "`Schema::create`/`drop_if_exists` lower-case quoted table names on PostgreSQL",
        "On PostgreSQL, rename existing mixed-case tables (`ALTER TABLE \"Name\" RENAME TO name`) or use a lowercase name.";
    SECRET_STRING_INPUT: "V13-SECRET-STRING-CLIENT-INPUT", Review, "13.0.0-alpha.1", "`SecretString` client input",
        "a `SecretString` deserialized from client input still decrypts any envelope under the configured key",
        "Add `#[serde(deserialize_with = \"rullst_orm::privacy::deserialize_plaintext_secret\")]` (or the optional variant) to fields filled from requests.";
    PROTECTED_VALUES: "V13-ORM-PROTECTED-VALUES", Review, "13.0.0-alpha.1", "ORM protected values and `SecretString` serialization",
        "encrypted and masked values are `***` in audit rows and events, and `SecretString` serializes as an encrypted envelope",
        "Configure `RULLST_ENCRYPTION_KEY` where `SecretString` is serialized, purge or reindex 12.x audit rows and search documents that may hold plaintext, and upgrade readers of serialized secrets before writers.";
    REDIS_MOCK_ORDER: "V13-REDIS-MOCK-TIE-ORDER", Review, "13.0.0-alpha.1", "Offline Redis mock",
        "the offline Redis mock returns equal `sorted_set_top` scores in descending member order",
        "Update tests that relied on ascending tie order.";
    AUTO_HEALING: "V13-AUTO-HEALING-DIAGNOSTICS", Review, "13.0.0-alpha.1", "Auto-healing diagnostics",
        "`diagnose_sql_error` returns `None` for more messages and uses driver-specific DDL",
        "Handle `None` and review assertions on suggested DDL.";
    ORM_MISSING_ROW: "V13-ORM-MISSING-ROW", Review, "13.0.0-alpha.1", "ORM instance mutations",
        "deleting, force-deleting or saving a missing row now fails with `RecordNotFound`",
        "Accept `RecordNotFound` where deletes were idempotent; `restore()` of a live row is a no-op.";
    ORM_CONSTRAINT_ERRORS: "V13-ORM-CONSTRAINT-ERRORS", Review, "13.0.0-alpha.2", "ORM constraint errors",
        "unique and foreign-key violations now return `UniqueViolation`/`ForeignKeyViolation` instead of `DatabaseError`",
        "Match `Error::UniqueViolation { .. }` and `Error::ForeignKeyViolation { .. }` instead of `DatabaseError` messages such as `UNIQUE constraint failed`, `duplicate key` or `Duplicate entry`.";
    ORM_TRANSACTION_ERRORS: "V13-ORM-TRANSACTION-ERRORS", Review, "13.0.0-alpha.3", "ORM transaction errors",
        "`Orm::transaction` returns its closure's own error after the rollback instead of `DatabaseError(\"Transaction failed: ...\")`",
        "Match the closure's error (`UniqueViolation`, `ForeignKeyViolation` or the application's own type) instead of `DatabaseError` on the result; the closure's error type must implement `From<rullst_orm::Error>`.";
    ORM_SANDBOX_TEST: "V13-ORM-SANDBOX-TEST", Review, "13.0.0-alpha.1", "ORM sandbox tests",
        "`#[rullst_orm::test]` now keeps the declared return type; an `Err` result fails the test",
        "Make the signature match the body (a `Termination` type such as `Result<(), E: Debug>`) and assert post-commit effects in a committing test.";
    ORM_SQLX_STRUCT_OPTION: "V13-ORM-SQLX-STRUCT-OPTION", MustChange, "13.0.0-alpha.1", "ORM model SQLx options",
        "a struct-level `#[sqlx(...)]` option other than `default` on a `#[derive(Orm)]` model no longer compiles",
        "Remove the option (for example `rename_all`) and name the fields after their columns.";
    ORM_ONLY_TRASHED: "V13-ORM-ONLY-TRASHED", Review, "13.0.0-alpha.1", "ORM trashed scopes",
        "`only_trashed()` on a model without soft deletes now fails with `Validation`",
        "Remove the call or give the model a `deleted_at` field or `#[orm(soft_delete)]`.";
    ORM_TRASHED_DELETE_ALL: "V13-ORM-TRASHED-DELETE-ALL", Review, "13.0.0-alpha.1", "ORM bulk soft deletes",
        "`delete_all()` with `with_trashed()`/`only_trashed()` now fails with `Validation` on soft-delete models",
        "Bulk-delete live rows without these modifiers and purge trashed rows with `force_delete()`.";
    ORM_SQL_TEXT: "V13-ORM-SQL-TEXT", Review, "13.0.0-alpha.1", "ORM joined scopes",
        "generated tenant, soft-delete and keyset predicates are now qualified with the table name",
        "Update tests that compare generated SQL text.";
    ORM_CHUNK_BOUNDS: "V13-ORM-CHUNK-BOUNDS", Review, "13.0.0-alpha.1", "ORM chunk bounds",
        "chunked queries now honour an explicit `limit`/`offset` instead of ignoring them",
        "Remove `limit()`/`offset()` from chunked queries that relied on them being ignored.";
    PERSONAL_DATA_REPORT: "V13-PERSONAL-DATA-REPORT", Review, "13.0.0-alpha.1", "PersonalData reports",
        "`compliance_schema()` reports the ORM table and lists only encrypted columns as encrypted",
        "Use `ComplianceModel::personal_fields()` for `#[privacy]` fields and review consumers of the report.";
    NEXUS_PRIMARY_KEY: "V13-NEXUS-PRIMARY-KEY", MustChange, "13.0.0-alpha.1", "Nexus derive",
        "two `#[nexus(primary_key)]` declarations, or one contradicting the struct-level key, no longer compile",
        "Keep exactly one primary-key declaration.";
    NEXUS_FIELD_KIND_NUMBER: "V13-NEXUS-FIELD-KIND-NUMBER", Review, "13.0.0-alpha.1", "Nexus derive",
        "derived integer fields are now `FieldKind::Integer { min, max }` instead of `FieldKind::Number`",
        "Expect `FieldKind::Integer` when comparing derived metadata; manual `FieldMeta` values keep their kind.";
    NEXUS_FIELD_KIND_MATCH: "V13-NEXUS-FIELD-KIND-MATCH", MustChange, "13.0.0-alpha.1", "Nexus panel",
        "`FieldKind` is now `#[non_exhaustive]`; a `match` without a wildcard arm no longer compiles",
        "Add a wildcard arm (`_ => ...`) to the match.";
    NEXUS_DOTENV: "V13-NEXUS-DOTENV", Review, "13.0.0-alpha.1", "Nexus credentials",
        "Nexus reads `NEXUS_ADMIN_*` from `.env` without loading it into the process environment",
        "Read other settings yourself (for example with `rullst::config::project_setting`) and handle `NexusBuildError::InvalidDotenv`.";
    DEV_TELEMETRY_ROUTE: "V13-DEV-TELEMETRY-ROUTE", Review, "13.0.0-alpha.1", "Core development telemetry",
        "`/_rullst/*` paths are reserved for development routes such as `/_rullst/dev-telemetry`",
        "Move application routes away from `/_rullst/`.";
    FOUNDRY_SERVICE: "V13-FOUNDRY-SERVICE", Review, "13.0.0-alpha.1", "CLI Foundry service environment",
        "`foundry:deploy` now sets `PORT`/`HOST`, uploads the reported executable and runs a dedicated service account",
        "Keep `[env] PORT` equal to `[app] port`, set `package.default-run` for several binaries and keep writable state under `/opt/rullst/<app>/data`.";
    MFA_CLIENT_SECRET: "V13-MFA-CLIENT-SECRET", Review, "13.0.0-alpha.1", "CLI generators",
        "an MFA handler generated before v13 verified a client-supplied `secret`",
        "Replace it with the server-side factor store that `cargo rullst make:mfa` now generates.";
    CAPITAL_REMOVED: "V13-CAPITAL-REMOVED", Review, "13.0.0-alpha.1", "Capital providers and NFS-e removed",
        "this names a payment provider, billing setting, payout or NFS-e API that v13 removed from `rullst-capital`",
        "Move to `StripeProvider`, or implement `BillingProvider` for the gateway in application code (see \"Writing your own payment provider\"); `InfinitePayProvider` covers one-off payment links only, with no subscriptions. In billing code generated by 12.x, delete the `lemonsqueezy` branches, `lemon_provider` and `BILLING_STORE_ID`, and keep `BILLING_PROVIDER=stripe`. Remove NFS-e and payout code, which are no longer part of Rullst.";
    QUOTA_KEYS: "V13-CAPITAL-QUOTA-KEYS", Review, "13.0.0-alpha.1", "Capital quota keys on MySQL/MariaDB",
        "`SqlQuotaStore` keys are case-sensitive; older MySQL/MariaDB tables need an `ALTER TABLE`",
        "On MySQL/MariaDB run the `ascii_bin` migration from the Capital README before serving quota calls.";
    ZERO_TIER: "V13-CAPITAL-ZERO-TIER", Review, "13.0.0-alpha.1", "Capital zero tier limit",
        "a `tier_limit` of `Some(0)` now returns `QuotaError::LimitExceeded` instead of `InvalidRequest`",
        "Map it like any other quota denial (402/403 with an upgrade prompt).";
    OUTBOX_RELAY_KEY: "V13-OUTBOX-RELAY-KEY", Review, "13.0.0-alpha.1", "Messaging outbox relay key",
        "`OrmOutboxRelay` publishes under a stream-scoped idempotency key",
        "Drain or acknowledge in-flight outbox claims before upgrading, or rely on consumer idempotency.";
    MAIL_FACADE: "V13-MAIL-FACADE-CONFIG", Review, "13.0.0-alpha.1", "Mail sender",
        "the `Mail` facade needs `MAIL_FROM`, and `MAIL_DRIVER` in staging/production; invalid `MAIL_PORT` now fails",
        "Set `MAIL_FROM` and `MAIL_DRIVER` (see also the Mail driver default and Mail facade settings rows).";
    MAIL_QUEUED_ATTACHMENTS: "V13-MAIL-QUEUED-ATTACHMENTS", Review, "13.0.0-alpha.1", "Mail queued attachments",
        "queued mail stores attachment bytes as base64, which 12.x mail workers cannot read",
        "In a rolling deployment upgrade every mail worker before any producer, retry jobs a 12.x worker failed once 13 workers run and drain the queue before rolling back.";
    MAIL_ATTACHMENTS: "V13-MAIL-ATTACHMENT-INSPECTION", Review, "13.0.0-alpha.1", "Mail attachment inspection",
        "`LocalAttachmentInspector` classifies XML by root element and namespaces",
        "Review XML attachments that embed SVG/XHTML namespaces or entity declarations.";
    MAIL_RESEND_SCHEDULE: "V13-MAIL-RESEND-SCHEDULE", Review, "13.0.0-alpha.1", "Mail Resend scheduling",
        "direct `ResendDriver` sends scheduled more than 30 days ahead now fail before the request",
        "Schedule such mail through a durable queue with `Mail::enqueue`.";
    MAIL_TRACKING: "V13-MAIL-TRACKING-RECIPIENT", Review, "13.0.0-alpha.1", "Mail tracking recipient",
        "open/click tracking now signs the bare recipient address",
        "Compare `OpenEvent.email`/`ClickEvent.email` with the bare address instead of the display-name form.";
    MAIL_TEXT_FALLBACK: "V13-MAIL-TEXT-FALLBACK", Review, "13.0.0-alpha.1", "Mail plain-text fallback",
        "the derived plain-text part now appends link targets as `label <URL>`",
        "Update tests comparing derived text, or set `text` explicitly.";
    MAIL_REMOVED: "V13-MAIL-REMOVED", Review, "13.0.0-alpha.1", "Mail providers removed",
        "this names a mail transport, `MAIL_DRIVER` value or setting that v13 removed from `rullst-mail`",
        "Use Resend, AWS SES, SendPulse or SMTP, or implement `MailDriver` for the provider in application code (see the custom transport guide); a removed `MAIL_DRIVER` fails every send.";
    LABS_RUNNER: "V13-LABS-RUNNER-REMOVED", MustChange, "13.0.0-alpha.1", "Labs runner",
        "the `rullst-labs-runner` candidate was removed from v13 and is not published",
        "Remove the dependency and deploy an application-owned runner against the labs controller contract.";
    BILLING_SETTINGS: "V13-BILLING-PROJECT-SETTINGS", Review, "13.0.0-alpha.1", "Generated billing settings",
        "generated billing code read `BILLING_*` from the process environment only",
        "Read `BILLING_*` with `rullst::config::project_setting` and check production with `RullstConfig::global().environment()`.";
    SQLITE_SEED_TIME: "V13-SQLITE-ONLY-SEED-TIME", Review, "13.0.0-alpha.1", "Starter migrations",
        "this seed uses SQLite-only `datetime('now')`, which PostgreSQL, MySQL and MariaDB reject",
        "Bind a UTC timestamp from Rust (or use `CURRENT_TIMESTAMP`) and advance PostgreSQL sequences after explicit ids.";
    ERP_DASHBOARD_ACCESS: "V13-ERP-DASHBOARD-ACCESS", Review, "13.0.0-alpha.1", "ERP starter access",
        "`GET /` is served outside the router that `protect_router` guards",
        "If `/` lists customers or revenue, move it into the routes passed to `admin_access.protect_router`.";
    LMS_PROGRESS_KEY: "V13-LMS-PROGRESS-KEY", Review, "13.0.0-alpha.1", "LMS lesson progress",
        "a fixed LMS progress idempotency key makes later saves return 409",
        "Copy the new `new_progress_key`/`progress_event_key` helpers and scope keys per learner.";
    LMS_RECORD_PROGRESS: "V13-LMS-RECORD-PROGRESS", Review, "13.0.0-alpha.1", "LMS concurrent progress saves",
        "`record_progress` writes before its `lesson_progress_events` insert, so its first write does not claim the idempotency key",
        "Copy the service's `replay` helper and `record_progress` from the repository's `examples/lms` (`src/services/learning_service.rs`).";
    LMS_MEDIA_KINDS: "V13-LMS-MEDIA-KINDS", Review, "13.0.0-alpha.1", "LMS lesson media in Nexus",
        "lesson `media_url`/`captions_url` are text fields in v13 so same-origin captions stay editable",
        "Make those Nexus fields `FieldKind::Text` and `media_kind` an `Enum` of `video`/`audio`.";
    BLOG_SITEMAP: "V13-BLOG-ROBOTS-SITEMAP", Review, "13.0.0-alpha.1", "Blog robots and sitemap",
        "`robots.txt`/`sitemap.xml` advertise relative URLs that crawlers reject",
        "Copy `public_origin`, `path_segment`, `robots_txt` and `sitemap_xml` from a new Blog project and set `RULLST_PUBLIC_ORIGIN`.";
    CREDENTIAL_RATE_LIMIT: "V13-CREDENTIAL-RATE-LIMIT", Review, "13.0.0-alpha.1", "Generated login and registration",
        "password hashing in this request code runs outside `run_password_work` or another bounded Argon2 permit, and this file applies no `credential_rate_limit`; when the limit lives in another module that this rule does not follow, put `// rullst-upgrade: reviewed V13-CREDENTIAL-RATE-LIMIT: <reason>` on the line above the call",
        "Copy the bounded hashing (`run_password_work`) and `credential_rate_limit` changes and apply them to `POST /login` and `/register`. A call of the application's own `hash_password` already counts as bounded when the package defines it with its hashing under `run_password_work` or a semaphore permit.";
    PASSWORD_HASH_HIDDEN: "V13-PASSWORD-HASH-HIDDEN", Review, "13.0.0-alpha.1", "SaaS user JSON",
        "`password_hash` is not `#[orm(hidden)]`, so `to_json()`, audit rows and model events carry the hash",
        "Add `#[orm(hidden)]` to the field.";
    ORM_SKIP_MANAGED_COLUMNS: "V13-ORM-SKIP-MANAGED-COLUMNS", Review, "13.0.0-alpha.3", "ORM database-managed columns",
        "this field is `#[orm(skip)]` but `FromRow` reads it, so it names a column; the query builder treats every `skip` field as not a database column, so a filter, ordering or selection on it fails at run time",
        "If the table has this column (as `is_admin` and `session_epoch` in the alpha.2 starters), declare it `#[orm(db_default)]` instead: ORM writes still never touch it, and queries can filter, order and select it. A field without a column also needs `#[sqlx(skip)]` (or `#[sqlx(default)]`), or the model's `FromRow` fails to read it.";
    EMPTY_TIMESTAMPS: "V13-EMPTY-TIMESTAMPS", Review, "13.0.0-alpha.1", "Registration timestamps",
        "an empty `created_at`/`updated_at` replaces the column default because the ORM inserts every field",
        "Store the current UTC time instead, as the new `utc_timestamp` helper does (`rullst::orm::Timestamp::now()`; see also Billing row timestamps and ORM date/time columns).";
    TEXT_TIMESTAMPS: "V13-TEXT-TIMESTAMPS", Review, "13.0.0-alpha.2", "ORM date/time columns",
        "a `String` `created_at`/`updated_at` cannot decode the TIMESTAMPTZ/DATETIME(6) columns `timestamps()` creates with a strict driver feature",
        "If this table's migration uses `timestamps()`, make the field `rullst::orm::Timestamp` (set with `Timestamp::now()`) or `Option<Timestamp>` with `#[orm(db_default)]`; keep `String` only on the SQLx Any pool or for a `string()` column.";
    UTF16_LENGTHS: "V13-UTF16-LENGTHS", Review, "13.0.0-alpha.1", "Registration lengths",
        "registration limits are counted in UTF-8 bytes; v13 counts the forms' UTF-16 code units",
        "Copy `form_length` and `valid_password`: 120-character names and 12-character passwords in UTF-16 units, at most 72 password bytes.";
    VPS_PROXY: "V13-VPS-DEPLOY-PROXY", Review, "13.0.0-alpha.1", "VPS deploy proxy",
        "`docker-compose.prod.yml` lacks the pinned `edge` network that lets the app trust Caddy",
        "Add the `edge` network (`172.31.250.0/24`, Caddy at `172.31.250.10`) and that address to `[security] trusted_proxies`.";
    MODEL_ALL: "V13-MODEL-ALL", Review, "13.0.0-alpha.1", "Blog and ERP reads",
        "`Model::all()` stops at the ORM's 1,000-row cap without an `ORDER BY`",
        "Page with `paginate` (newest first) or read one row with `query().first()`; return 503 on database errors (see also Blank database status).";
    PROFILE_FIND: "V13-PORTFOLIO-PROFILE", Review, "13.0.0-alpha.1", "Portfolio profile",
        "the Portfolio page hard-codes `Profile::find(1)`",
        "Show the first profile by id and answer 404 when none exists.";
    PAGE_CSP_NONCE: "V13-PAGE-CSP-NONCE", Review, "13.0.0-alpha.1", "Blog and Portfolio pages",
        "this page renders without the request's CSP nonce, so the production CSP blocks its inline styles",
        "Pass the request's `CspNonce` into the page, replace `style` attributes with classes and serve assets same-origin.";
    LINKER_CONFIG: "V13-LINKER-CONFIG", Review, "13.0.0-alpha.1", "Generated linker configuration",
        "`.cargo/config.toml` selects a host linker but is not ignored by Git",
        "List `.cargo/config.toml` in `.gitignore` and `.dockerignore`, or delete its `-fuse-ld` flags before CI or Docker builds.";
    MSVC_FASTLINK: "V13-MSVC-FASTLINK", Review, "13.0.0-alpha.1", "Build files from the 12.0 CLI",
        "`.cargo/config.toml` passes the unsupported MSVC linker flag `/DEBUG:FASTLINK`",
        "Delete the `[target.x86_64-pc-windows-msvc]` block that sets `/DEBUG:FASTLINK`.";
    CARGO_LOCK_IGNORED: "V13-CARGO-LOCK-IGNORED", Review, "13.0.0-alpha.1", "Build files from the 12.0 CLI",
        "`.gitignore` ignores `Cargo.lock`, so builds and deployments may resolve untested versions",
        "Remove the `Cargo.lock` line from `.gitignore` and commit the lockfile.";
    DOCKER_UNLOCKED_BUILD: "V13-DOCKER-UNLOCKED-BUILD", Review, "13.0.0-alpha.1", "Build files from the 12.0 CLI",
        "the Dockerfile runs `cargo build` without `--locked`",
        "Commit `Cargo.lock` and add `--locked` to the Dockerfile's `cargo build`.";
    ERP_STORE_ORDER: "V13-ERP-STORE-ORDER", Review, "13.0.0-alpha.1", "ERP orders and stock",
        "the ERP order handler reserves stock with a read-modify-write",
        "Copy the transactional `store_order`/`add_stock` controller with its 404/409/422/503 answers.";
    PAGE_BOUNDS: "V13-PAGE-BOUNDS", Review, "13.0.0-alpha.1", "Blog and ERP page bounds",
        "`paginate` receives an unbounded `?page=` value",
        "Answer 404 for a page above a `MAX_PAGE` bound (for example 100,000) before calling `paginate`.";
    DOCKER_CONTEXT: "V13-DOCKER-CONTEXT", Review, "13.0.0-alpha.1", "Docker build context",
        "`.dockerignore` does not exclude `.env.*`, `Foundry.toml` or local database files",
        "Add `.env.*` (keeping `!.env.example`), `Foundry.toml`, `*.sqlite`, `*.sqlite3`, `*.duckdb` and their journal files.";
    K8S_INGRESS_TLS: "V13-K8S-INGRESS-TLS", Review, "13.0.0-alpha.1", "Kubernetes Ingress",
        "this Ingress uses the deprecated class annotation and may serve plain HTTP",
        "Use `ingressClassName: nginx` and add a `tls` block for the real host.";
    HEALTH_PROBES: "V13-HEALTH-PROBES", Review, "13.0.0-alpha.1", "Starter health probes",
        "the server does not mount `rullst::health::health_router()`, so `/health` and `/ready` probes answer 404",
        "Merge `rullst::health::health_router()` (or `health_router_with_lifecycle`) outside any authentication layer.";
    K8S_NAMES: "V13-K8S-NAMES", Review, "13.0.0-alpha.1", "Kubernetes and Buildah names",
        "the package name is not a lowercase DNS label; v13 derives Kubernetes and image names differently",
        "Regenerate the manifests or Buildah script, or rename the objects and image.";
    ORM_DEFAULT_FEATURES: "V13-ORM-DEFAULT-FEATURES", Review, "13.0.0-alpha.3", "Minimal default features",
        "this `rullst-orm` dependency relies on the default features, which no longer include `drivers-all`, so it has no SQLx database driver",
        "Enable the backend the application connects to, for example `default-features = false, features = [\"strict-sqlite\"]` (or `strict-postgres`, `strict-mysql`, or `drivers-all` for all three).";
    RULLST_DEFAULT_FEATURES: "V13-RULLST-DEFAULT-FEATURES", Review, "13.0.0-alpha.3", "Minimal default features",
        "this `rullst` dependency relies on the default features, which no longer include `orm`, `drivers-all` and `queue-sqlite`",
        "If the application uses the ORM or the SQLite queue, list them, for example `default-features = false, features = [\"strict-sqlite\", \"queue-sqlite\"]` (`drivers-all` for all three drivers); a database-free application needs no change.";
    POLYGLOT_FEATURE: "V13-POLYGLOT-FEATURE", MustChange, "13.0.0-alpha.2", "ORM adapter aggregate features",
        "the `orm-polyglot` (`rullst`) and `polyglot` (`rullst-orm`) features were removed; Cargo refuses a manifest that enables them",
        "Enable only the adapters the application uses: `orm-turso` or `orm-surrealdb` on `rullst`, or `turso` or `surrealdb` on `rullst-orm` (13.0.0-alpha.5 removed MongoDB, DuckDB and Qdrant: see `V13-ORM-ADAPTERS-REMOVED` and `V13-ORM-QDRANT-REMOVED`).";
    OMNI_REMOVED: "V13-OMNI-REMOVED", Review, "13.0.0-alpha.3", "Omni and offline sync removed",
        "13.0 removed `make:omni` and `cargo rullst omni`; this project has an Omni shell or runs the removed commands",
        "Run `cargo rullst make:pwa` (with `--android` for Google Play), ship the installable PWA, then retire `omni-app/` and the `cargo rullst omni` steps.";
    OFFLINE_SYNC_REMOVED: "V13-OFFLINE-SYNC-REMOVED", MustChange, "13.0.0-alpha.3", "Omni and offline sync removed",
        "the `offline-sync` feature and `rullst::offline_sync` were removed; Cargo refuses a manifest that enables the feature",
        "Remove the feature; keep the server authoritative and queue browser writes in IndexedDB with idempotency keys as the PWA guide shows.";
    PAGE_LANGUAGE: "V13-RENDER-PAGE-LANGUAGE", Review, "13.0.0-alpha.1", "Starter page language",
        "`render_page` declares `lang=\"pt-BR\"`; English pages should use `render_page_with_lang`",
        "Call `rullst::htmx::render_page_with_lang(&htmx, \"en\", title, content)` with the page's language.";
    DEPLOY_TARGETS_REMOVED: "V13-DEPLOY-TARGETS-REMOVED", Review, "13.0.0-alpha.4", "CLI deploy targets removed",
        "`cargo rullst deploy` no longer writes or runs Fly.io, Railway or Render files; this project has one or runs `deploy --platform` with them",
        "The file stays yours and keeps working with the provider's own CLI. `cargo rullst deploy` now writes a Dockerfile, compose.yaml and Caddyfile for any Linux VM; other targets: https://rullst.github.io/Rullst/book/advanced-deployment.html";
    DEPLOY_GENERATORS_REMOVED: "V13-DEPLOY-GENERATORS-REMOVED", Review, "13.0.0-alpha.4", "CLI Foundry, Nix, Buildah and gRPC generators removed",
        "13.0 removed `foundry:init|deploy`, `nixify`/`new --nix`, `generate:buildah`/`new --buildah` and `make:grpc`; this file came from one of them or runs them",
        "Generated files stay yours; nothing regenerates or updates them. Keep or retire them (gRPC code needs your own tonic and tonic-build setup) and see https://rullst.github.io/Rullst/book/advanced-deployment.html for Nix, Buildah, Podman Quadlet and Kubernetes.";
    CLI_SELF_UPDATE: "V13-CLI-SELF-UPDATE-REMOVED", Review, "13.0.0-alpha.4", "CLI self-update removed",
        "the CLI no longer installs itself: `update check|verify|stage|install` and `update guided --scope|--root` were removed; these scripts run them",
        "Install the CLI with `cargo install cargo-rullst --version =<version> --locked` (verify downloaded release binaries by hand as SECURITY.md shows), and keep only `update project …` or `update guided --to <version> --project <path>`.";
    CLI_AI_REMOVED: "V13-CLI-AI-REMOVED", Review, "13.0.0-alpha.4", "CLI `ai` removed, `footprint` renamed `green`",
        "`cargo rullst ai` was removed, and `cargo rullst footprint` changed its flags and report (it was `green` in 13.0.0-alpha.4 and is `footprint` again since alpha.5); these scripts run them",
        "Check the `cargo rullst footprint` steps: it reports only measured figures (`--bench` for CPU time and bytes per request, `--page-weight` for a page's bytes; no requests per second, latency, peak memory or RAPL energy) and its JSON is `rullst.footprint.v1` instead of `rullst.cli-footprint.v1`. Remove the `cargo rullst ai` steps: use your editor's or terminal's AI assistant with the project's AGENTS.md (`cargo rullst generate:ai-context`) and https://rullst.github.io/Rullst/llms.txt.";
    ORM_FROM_ROW: "V13-ORM-FROM-ROW", MustChange, "13.0.0-alpha.4", "ORM `FromRow` and SQLx macros",
        "`#[derive(Orm)]` now implements `FromRow`, so deriving `FromRow` too no longer compiles (E0119, or \"expected derive macro, found trait\")",
        "Remove `FromRow` from this derive list and from the `use` line that imports it. A model that keeps its own `FromRow` (hand-written, or SQLx's derive with SQLx's `derive` feature in Cargo.toml) declares `#[orm(custom_from_row)]`.";
    SQLX_MACROS: "V13-SQLX-MACROS", MustChange, "13.0.0-alpha.4", "ORM `FromRow` and SQLx macros",
        "this code uses SQLx's derive or query macros, which Rullst crates no longer enable, and no manifest here enables them",
        "Enable them on the application's own SQLx dependency, `sqlx = { version = \"0.9\", default-features = false, features = [\"derive\"] }` (`\"macros\"` for `query!`), which also builds `sqlx-macros`; or decode into a tuple or a hand-written `FromRow` impl.";
    NEXUS_AI_FEATURE: "V13-NEXUS-AI-FEATURE", Review, "13.0.0-alpha.4", "Nexus AI assistant feature",
        "this Nexus dependency no longer includes the AI assistant: `/nexus/chat` answers 404 and the panel shows no assistant link",
        "If the application uses the assistant, enable `nexus-ai` on `rullst` (or `ai` on a `rullst-nexus` dependency without default features); otherwise nothing is needed, and `rullst-ai` leaves the build.";
    SQLX_TLS_RING: "V13-SQLX-TLS-RING", Review, "13.0.0-alpha.4", "TLS crypto provider",
        "this `sqlx` dependency enables `tls-rustls` or `tls-rustls-ring`, so SQLx keeps ring for database TLS while Rullst's other TLS connections use aws-lc-rs",
        "Change the feature to `tls-rustls-aws-lc-rs`; SQLx prefers ring whenever any crate enables it, so one ring feature keeps every SQLx connection on ring. See https://rullst.github.io/Rullst/book/security-architecture.html#cryptography-providers";
    SQLX_TLS_NATIVE_ROOTS: "V13-SQLX-TLS-NATIVE-ROOTS", Review, "13.0.0-alpha.5", "TLS crypto provider",
        "this `sqlx` dependency enables `tls-rustls-ring-native-roots`, so SQLx keeps ring, with the operating system's roots, for database TLS while Rullst's other TLS connections use aws-lc-rs",
        "SQLx 0.9 has no aws-lc-rs feature that trusts the operating system's roots. Either change the feature to `tls-rustls-aws-lc-rs`, which trusts the bundled WebPKI (Mozilla) roots (for a database certificate from a private CA, pass that CA with the connection's `sslrootcert` (PostgreSQL) or `ssl-ca` (MySQL) option), or keep native roots and record why: SQLx then keeps ring for every database connection. See https://rullst.github.io/Rullst/book/security-architecture.html#cryptography-providers";
    DEPLOY_SQLITE_URL: "V13-DEPLOY-SQLITE-URL", Review, "13.0.0-alpha.5", "SQLite database in the deployed image",
        "this SQLite project's Dockerfile sets no `DATABASE_URL`, so in the deployed container the database path points into the read-only `/app` and the app stops at startup",
        "Add `ENV DATABASE_URL=sqlite:///app/data/rullst.db?mode=rwc` to the runtime stage (the last `FROM`), which keeps the database in the `/app/data` volume, or set `DATABASE_URL` in `.env.production`; `cargo rullst deploy` and `dockerize` write that line since 13.0.0-alpha.4.";
    DEPLOY_CADDY_HEALTH: "V13-DEPLOY-CADDY-HEALTH", Review, "13.0.0-alpha.5", "CLI deploy targets removed",
        "this `Caddyfile` proxies to `app:3000` without the health checks `cargo rullst deploy` now writes, so Caddy answers with an error while the app starts or restarts instead of holding requests",
        "Move the file aside and run `cargo rullst deploy` again, or add `health_uri /health`, `health_interval 5s` and `lb_try_duration 15s` inside a `reverse_proxy app:3000 { ... }` block: Caddy then stops routing to the app while `/health` fails and holds each request for up to 15 seconds.";
    MIGRATION_AUTO_STATUS: "V13-CLI-MIGRATION-AUTO-STATUS", Review, "13.0.0-alpha.5", "CLI `make:migration:auto` without a database to compare",
        "this script runs `make:migration:auto`, which exits with status 1 (\"No migration generated\") since 13.0.0-alpha.4 when there is no SQLite database to compare",
        "Run it only where `.env` has a SQLite `DATABASE_URL` (no `.env`, no `DATABASE_URL`, a non-SQLite URL or a Turso primary has nothing to compare), or stop expecting status 0 there; a database already in sync with the models still exits with status 0.";
    BLUEPRINTS_REMOVED: "V13-CLI-BLUEPRINTS-REMOVED", Review, "13.0.0-alpha.5", "CLI starters moved to the examples",
        "this script passes `--blueprint lms|blog|portfolio|erp`, which `cargo rullst new` refuses with status 1 since 13.0.0-alpha.4, before writing anything",
        "Copy the application from the repository's `examples/lms` or `examples/blog` (https://github.com/Rullst/Rullst/tree/main/examples; the blog example also replaces the Portfolio and ERP examples that 13.0.0-alpha.5 removed) instead, or start from `--blueprint api` or `--blueprint saas`.";
    ALIAS_PATHS: "V13-API-ALIAS-PATHS", MustChange, "13.0.0-alpha.5", "Duplicate API paths removed",
        "13.0.0-alpha.5 removed `rullst::rullst_orm`, `rullst::security_runtime`, `rullst_mail::mail` and `rullst_mail::drivers::mock`; this path no longer compiles",
        "Write `rullst::orm` (with `use rullst::orm as rullst_orm;` where code names `rullst_orm`), `rullst::security::runtime`, the `rullst_mail` crate root and `rullst_mail::mock`.";
    HIDDEN_INTERNALS: "V13-HIDDEN-INTERNALS-PRIVATE", Review, "13.0.0-alpha.5", "Hidden internals made private",
        "this names an internal that 13.0.0-alpha.4 hid from the documentation and 13.0.0-alpha.5 made crate-private or removed",
        "Use the documented replacement: `Orm::transaction`, `with_tenant`/`get_tenant_id`, the `schema` query-log, timeout and limit setters and getters, `artisan::check_and_run_artisan`, `rullst_core::radar::get_process_memory_mb`, `SecurityStore::live_events`; the Connect provider macros, `DEFAULT_HTTP_CLIENT`, `passkey::cbor` and the Nexus helpers have no public replacement. `Outbox::claim_next_at` and `query_cache::model_hash_key` need rullst-orm's `test-util` feature in `[dev-dependencies]`, outside the 13.x stability promise.";
    STORE_COLLECTIONS: "V13-SECURITY-STORE-COLLECTIONS", Review, "13.0.0-alpha.5", "Security store collections",
        "the `banned_ips`, `honeypot_route_hits` and `live_events` fields of `SecurityStore` are private since 13.0.0-alpha.5",
        "Read them with `store.active_bans()`, `store.is_banned(ip)`, `store.honeypot_route_hits()` and `store.live_events()`, which return sorted copies (events newest first).";
    ERROR_PAYLOADS: "V13-ERROR-PAYLOADS", Review, "13.0.0-alpha.5", "Rullst-owned error payloads",
        "this error variant no longer carries a `reqwest` or `serde_json` value: `AiError::RequestError` holds `AiRequestError`, `AiError::SerializationError` `AiJsonError` and `TwinSyncError::Serialization` `TwinJsonError`",
        "Use the payload through `Display`, `source()` or its methods (`is_timeout`, `is_connect`); match `TwinSyncError` with a `_` arm. `?` no longer converts `reqwest`, `serde_json`, `base64` or `jsonwebtoken` errors into `ConnectError`, or `serde_json`/`redis` errors into `rullst_orm::Error`: map them, for example `.map_err(|error| Error::SerializationError(error.to_string()))`.";
    SECURITY_GLOBALS: "V13-SECURITY-GLOBAL-COLLECTIONS", Review, "13.0.0-alpha.5", "Security store collections",
        "`global_deception_routes()` returns `DeceptionRoutes` and `default_deception_traps()` a sorted `Vec<String>` instead of `dashmap` sets, and `global_rate_limit_store()` is private since 13.0.0-alpha.5",
        "Call `contains`, `len`, `is_empty` or `routes()` on `DeceptionRoutes` and register paths with `try_register_deception_trap`; use `is_rate_limited` or a `RateLimiter` instead of reading the rate-limit store.";
    CONNECT_JWKS: "V13-CONNECT-JWKS-KEY-SET", Review, "13.0.0-alpha.5", "Connect JWKS key sets",
        "`JwksCache` and `fetch_and_cache_jwks` return `JsonWebKeySet` instead of `jsonwebtoken`'s `JwkSet` since 13.0.0-alpha.5, and the deprecated `JWKS_CACHE` was removed",
        "Read the set with `key_ids()`, `contains_key_id(kid)`, `len()` and `is_empty()`; the providers verify tokens with it. Replace `JWKS_CACHE` with a `JwksCache` the application owns.";
    AI_HTTP_STATUS: "V13-AI-HTTP-STATUS", Review, "13.0.0-alpha.5", "AI provider HTTP errors",
        "an unsuccessful provider HTTP status is `AiError::HttpStatus` since 13.0.0-alpha.5, no longer `AiError::ApiError`",
        "Match `AiError::HttpStatus(error)` and read `error.status()`, `error.retry_after()`, `error.code()` and `error.message()` instead of searching the text for `429` or `404`; the text is unchanged, and `ApiError` still reports malformed or incomplete replies.";
    ISLANDS_REMOVED: "V13-ISLANDS-REMOVED", MustChange, "13.0.0-alpha.5", "Wasm islands removed",
        "`#[island]` (`rullst::island`) was removed in 13.0.0-alpha.5; this code no longer compiles",
        "Render the component on the server with `html!` and drop the attribute; send interaction to the server with htmx (`hx-get`, `hx-post`) and use native HTML (`<dialog>`, `popover`, `<details>`) or a small same-origin script with the request's CSP nonce for client-only behaviour. Remove `src/islands/`, the `wasm32` dependencies `make:island` added and `static/rullst-islands.js`.";
    ISLAND_COMMANDS_REMOVED: "V13-ISLAND-COMMANDS-REMOVED", Review, "13.0.0-alpha.5", "Wasm islands removed",
        "this script runs `make:island` or `build:client`, which 13.0.0-alpha.5 removed; both now exit with status 1",
        "Remove the step: there is no island client to build. Serve the page with `html!` and htmx, and delete the generated `static/rullst-islands.js` and wasm-bindgen output.";
    LIVE_REMOVED: "V13-LIVE-REMOVED", MustChange, "13.0.0-alpha.5", "Live components removed",
        "`rullst::live` (`LiveComponent`, `Live::mount`, `live_ws_handler`, `live::recovery`) and `#[live_component]`/`#[live_event]` were removed in 13.0.0-alpha.5; this code no longer compiles",
        "Handle events as ordinary routes that return HTML fragments for htmx (`hx-post`, `hx-target`, `hx-swap`), and push server updates with Server-Sent Events: publish to a `BroadcastManager` channel and return `RealtimeSse::channel` from a `GET` route that the page opens with the htmx SSE extension (`hx-ext=\"sse\"`, `sse-connect`, `sse-swap`). Remove the WebSocket route and the `htmx-ext-ws` script.";
    LIVE_COMMAND_REMOVED: "V13-LIVE-COMMAND-REMOVED", Review, "13.0.0-alpha.5", "Live components removed",
        "this script runs `make:live`, which 13.0.0-alpha.5 removed; it now exits with status 1",
        "Remove the step; write the route and its `html!` fragment by hand, with `cargo rullst make:controller` for the module.";
    SERVER_FUNCTIONS_REMOVED: "V13-SERVER-FUNCTIONS-REMOVED", MustChange, "13.0.0-alpha.5", "Typed server functions removed",
        "`#[server_function]`, `rullst::rpc` (`RpcResult`, `RpcFailure`), its `wasm32` caller `rullst::client` and the `rullst::client_contract` envelope were removed in 13.0.0-alpha.5; this code no longer compiles",
        "Make the function an ordinary handler registered with `routes!` (`post(\"/counter\" => increment)`) inside the CSRF layer: return an `html!` fragment for htmx (`hx-post`, `hx-target`) or `rullst::web::axum::Json` that Alpine.js reads with `fetch`, sending the CSRF token in the `X-CSRF-Token` header. Report failures with HTTP status codes, and remove the `<name>_rpc_router()` calls.";
    FRONTEND_ADAPTERS_REMOVED: "V13-FRONTEND-ADAPTERS-REMOVED", MustChange, "13.0.0-alpha.5", "Frontend adapters removed",
        "`rullst::frontend` (`RenderAdapter`, `HtmlAdapter`, `PicoAdapter`, `TopcoatAdapter`, `TemplateAdapter`, `LiveViewAdapter`, `WasmIslandAdapter`, `LeptosAdapter`, `DioxusAdapter`) was removed in 13.0.0-alpha.5; this code no longer compiles",
        "Return the `Markup` of `html!` from the handler, or wrap HTML rendered elsewhere in `rullst::response::Html(...)`: each adapter only answered its string as `text/html`.";
    HTML_ALPINE_DIRECTIVE: "V13-HTML-DYNAMIC-ALPINE-DIRECTIVE", MustChange, "13.0.0-alpha.5", "`html!` event handlers",
        "a dynamic value in an Alpine.js `x-*` attribute of `html!` no longer compiles: Alpine runs it as JavaScript",
        "Write the directive as a static string and pass the server value through a `data-*` attribute (`data-count={count}` read as `$el.dataset.count`), or render the JSON into a `<script type=\"application/json\">` data block that the component reads.";
    IOT_OTA_STATE: "V13-IOT-OTA-STATE", Review, "13.0.0-alpha.5", "IoT OTA manager state",
        "`OtaManager`'s `current_partition`, `status` and `firmware_version` are private fields since 13.0.0-alpha.5; this file reads or assigns one",
        "Call the methods of the same names (`ota.current_partition()`, `ota.status()`, `ota.firmware_version()`). Replace an assignment `ota.current_partition = bank` with `ota = ota.with_running_partition(bank)` before verifying an update, or build the manager with `new_with_running_partition`.";
    IOT_CODEC_ERROR_MATCH: "V13-IOT-CODEC-ERROR-MATCH", MustChange, "13.0.0-alpha.5", "IoT encoder errors",
        "`MqttCodecError` and `CoapCodecError` are `#[non_exhaustive]` since 13.0.0-alpha.5 and gained `AllocationFailed`; a `match` on them without a wildcard arm no longer compiles",
        "Add a wildcard arm (`_ => ...`). Treat `AllocationFailed` as running out of memory, not as invalid input: the same packet encodes once memory is free.";
    IOT_SENSOR_CARD_HTML: "V13-IOT-SENSOR-CARD-HTML", Review, "13.0.0-alpha.5", "IoT sensor card HTML",
        "`IotDashboard::render_sensor_card` returns `IotHtml` instead of `String` since 13.0.0-alpha.5 and shows the value with at most two decimals",
        "Call `.into_string()` where a `String` is needed, or insert it into a view with `Markup::from_trusted(card.into_string())`; `IotHtml` reads like a `&str`. Update tests or parsers that expected the raw `f64` text.";
    CAPITAL_INVOICE_MINOR_UNITS: "V13-CAPITAL-INVOICE-MINOR-UNITS", MustChange, "13.0.0-alpha.5", "Capital invoice minor units",
        "Capital's `Invoice` and `InvoiceItem` hold integer minor units since 13.0.0-alpha.5: the `f64` fields `amount` and `total` are gone and both structs are built with constructors",
        "Build `Invoice::new(id, email, date, currency).with_item(description, amount_minor)` with each amount in the currency's smallest unit (49.90 BRL is `4_990`); the total is the sum of the items (`total_minor()`). Read `item.amount_minor` instead of `item.amount`, and use `with_locale(InvoiceLocale::PtBr)` or `InvoiceLocale::Es` for receipt labels.";
    ORM_ADAPTERS_REMOVED: "V13-ORM-ADAPTERS-REMOVED", MustChange, "13.0.0-alpha.5", "MongoDB and DuckDB adapters removed",
        "13.0.0-alpha.5 removed the MongoDB and DuckDB adapters: Cargo refuses the `orm-mongodb` and `orm-duckdb` (`rullst`) or `mongodb` and `duckdb` (`rullst-orm`) features, and `MongoDbStore`, `DuckDbStore` and the DuckDB analytics types no longer compile",
        "Remove the feature and use the official `mongodb` or `duckdb` crate in the application: one client opened at start-up, shared through application state, behind a small trait with an offline fake for tests. https://rullst.github.io/Rullst/book/recipes/mongodb.html and https://rullst.github.io/Rullst/book/recipes/duckdb.html show both; `CollectionName`, `DocumentId`, the document traits and encrypted document recovery stay in `rullst::orm::polyglot`.";
    MAIL_MESSAGE_REPLY_TO: "V13-MAIL-MESSAGE-REPLY-TO", MustChange, "13.0.0-alpha.5", "Mail Reply-To",
        "Mail's `Message` has a `reply_to` field since 13.0.0-alpha.5, so a struct literal that lists every field without `..` no longer compiles",
        "Build the message with `Message::new()` and its builders (`.to(..)`, `.subject(..)`, `.reply_to(..)`); since 13.0.0-alpha.6 `Message` is `#[non_exhaustive]`, so no literal compiles outside `rullst-mail`.";
    PRIVACY_CONSENT_CHOICE_MATCH: "V13-PRIVACY-CONSENT-CHOICE-MATCH", MustChange, "13.0.0-alpha.5", "Privacy consent choices",
        "`rullst-privacy`'s `ConsentChoice` is `#[non_exhaustive]` since 13.0.0-alpha.5; a `match` on it without a wildcard arm no longer compiles",
        "Add a wildcard arm (`_ => ...`) that treats an unknown choice like `Unset`: no consent, so the optional processing stays off.";
    LABS_NON_EXHAUSTIVE_MATCH: "V13-LABS-NON-EXHAUSTIVE-MATCH", MustChange, "13.0.0-alpha.5", "Labs outcomes and states",
        "`rullst-labs`'s `JobState`, `JobResult`, `CaseGrade`, `CaseOutput`, `TrapKind`, `WorkerOutcome`, `ExecutionFailure`, `ResultEvidence` and `ExecutionProfile` are `#[non_exhaustive]` since 13.0.0-alpha.5, and `ExecutionFailure` gained `TimeLimit`; a `match` on them without a wildcard arm no longer compiles",
        "Add a wildcard arm (`_ => ...`) that shows an unknown state or result generically. Show `ExecutionFailure::TimeLimit` as the exercise's time limit, not as a cancellation; a runner controller that stops an attempt at the wall limit reports it through `complete` (Labs runner contract, step 4).";
    CLI_GREEN_RENAMED: "V13-CLI-GREEN-RENAMED", Review, "13.0.0-alpha.5", "CLI `green` renamed `footprint`",
        "`cargo rullst green` is `cargo rullst footprint` since 13.0.0-alpha.5, without an alias, and its JSON report is `rullst.footprint.v1` instead of `rullst.green.v1`; this file runs or reads the old one",
        "Replace `cargo rullst green` with `cargo rullst footprint` (the same flags, plus `--page-weight` and `--network-kwh-per-gb`) and `rullst.green.v1` with `rullst.footprint.v1`; the report adds `page_weight` and the benchmark's bytes. Where the file installs a pinned CLI (`cargo install cargo-rullst --version =…`), pin 13.0.0-alpha.5 or later: the 13.0.0-alpha.4 CLI has no `footprint`. A line kept on purpose, such as a changelog entry, can carry `rullst-upgrade: reviewed V13-CLI-GREEN-RENAMED`.";
    ACADEMY_REMOVED: "V13-ACADEMY-REMOVED", MustChange, "13.0.0-alpha.5", "Academy production preset removed",
        "`ProductionPreset::academy()`, the `Academy*` production types and `cargo rullst academy:doctor` were removed in 13.0.0-alpha.5; this code no longer compiles, and the command exits with status 1",
        "Keep the application's own production checklist in its tests or CI instead of `ProductionPreset::academy()` and the `AcademyProductionRequirement`, `AcademyCheckStatus`, `AcademyBoundaryAssessment`, `AcademyPresetError` and `AcademyProductionPreset` types, and remove `academy:doctor` steps; `ProductionPreset::middleware_order()` and `ProductionMiddlewareStage` remain.";
    ORM_QDRANT_REMOVED: "V13-ORM-QDRANT-REMOVED", MustChange, "13.0.0-alpha.5", "Qdrant adapter removed",
        "13.0.0-alpha.5 removed the Qdrant adapter: Cargo refuses the `orm-qdrant` (`rullst`) and `qdrant` (`rullst-orm`) features, and `QdrantStore`, `QdrantConfig`, `VectorRepository` and the `Vector*` types no longer compile",
        "Remove the feature and use the official `qdrant-client` crate in the application: one client created at start-up, shared through application state, behind a small trait with an in-memory fake for tests, and every search filtered by tenant and bounded. https://rullst.github.io/Rullst/book/recipes/qdrant.html shows it; for vectors next to the rows they describe, `pgvector` on PostgreSQL stays.";
    AI_PROVIDER_PRESETS: "V13-AI-PROVIDER-PRESETS", MustChange, "13.0.0-alpha.5", "DeepSeek and Ollama as OpenAI-compatible presets",
        "`DeepSeekProvider` and `OllamaProvider` (`rullst_ai::ai::providers::deepseek` and `::ollama`) were removed in 13.0.0-alpha.5; this code no longer compiles",
        "Use the presets of the OpenAI-compatible adapter: `OpenAiCompatibleProvider::deepseek(api_key)?` (`.try_with_model(..)?` instead of `.with_model(..)`) and `OpenAiCompatibleProvider::ollama(host, model)?` (`.try_with_embedding_model(..)?` instead of `.with_embedding_model(..)`), from `rullst::ai::providers::openai_compatible`. Both return a `Result`; Ollama is now reached through its `/v1` API.";
    HEALTH_FRAMEWORK_VERSION: "V13-HEALTH-FRAMEWORK-VERSION", Review, "13.0.0-alpha.6", "Health probes without the framework version",
        "`/health` and `/ready` no longer report the framework version as `version` since 13.0.0-alpha.6; this file reads `version` next to a probe path",
        "Read `app_version` instead, after mounting `health_router_with_version(env!(\"CARGO_PKG_VERSION\"))` (or `health_router_with_lifecycle_and_version`), or read `framework.version` from the authenticated production monitor (`Server::with_monitor`, `GET /_rullst/monitor/v1`). A probe the application writes itself is unaffected.";
    HEALTH_PROBE_VERSION_FIELD: "V13-HEALTH-PROBE-VERSION-FIELD", MustChange, "13.0.0-alpha.6", "Health probes without the framework version",
        "`HealthProbeResponse` and `ReadinessProbeResponse` have no `version` field since 13.0.0-alpha.6, and `HealthProbeResponse` is `#[non_exhaustive]`; this code no longer compiles",
        "Remove the `version` reads and build a probe payload of your own type: the probes no longer carry the framework version. `health_router_with_version` adds the application's version as `app_version`, and the authenticated production monitor reports the framework version as `framework.version`.";
    CSRF_STATIC_PREFIXES: "V13-CSRF-STATIC-PREFIXES", Review, "13.0.0-alpha.6", "CSRF static files by path",
        "since 13.0.0-alpha.6 the CSRF middleware treats only `[security] csrf_static_prefixes` (default `/static/`, `/.well-known/`) and `csrf_static_paths` as static files, not file extensions; this router serves files at another path, whose first visit now sets the `rullst_csrf` cookie",
        "List the prefix with the defaults you keep, `csrf_static_prefixes = [\"/static/\", \"/.well-known/\", \"/assets/\"]` in `Rullst.toml` (or one file in `csrf_static_paths`); in code, `SecurityConfig::with_csrf_static_prefix(\"/assets/\")` adds one. Nothing breaks if you leave it: those files then set the cookie for a visitor who has none, unless their response is publicly cacheable.";
    CSRF_COOKIE_IN_PAGE: "V13-CSRF-COOKIE-IN-PAGE", Review, "13.0.0-alpha.6", "CSRF token masked per response",
        "since 13.0.0-alpha.6 `CsrfToken::as_str` is masked with a fresh random mask in every response; this code reads the raw `rullst_csrf` cookie value, which still verifies, but a page that embeds it repeats the same secret in every response, which a compressed page next to reflected input exposes to BREACH",
        "Render `CsrfToken::as_str()` from `Extension<CsrfToken>`, or mask the cookie value before a page embeds it: `.strip_prefix(\"rullst_csrf=\").and_then(rullst::security::mask_csrf_token)`. A value the code only sends in a request header or compares needs no change.";
    ROUTER_BASELINE_LAYERS: "V13-ROUTER-BASELINE-LAYERS", Review, "13.0.0-alpha.6", "Security baseline in development",
        "since 13.0.0-alpha.6 `Server` applies the CSRF check and the secure headers in every environment, development and the hot-reload server of `cargo rullst dev` included; this router adds `csrf_middleware` or `headers_middleware` again, which a request served by `Server` pays for twice",
        "Remove the layer when `Server` serves this router, as new applications do; a test that calls the router directly wraps it in `rullst::security::apply_security_baseline(router.into_axum(), security, Environment::Test)`, as the generated `src/security_tests.rs` does. Keep it for a router served without `Server` (`axum::serve`). Nothing breaks if you leave it: the second layer only repeats the first.";
    DEPLOY_CADDY_ENCODE: "V13-DEPLOY-CADDY-ENCODE", Review, "13.0.0-alpha.6", "Compressed pages in the generated Caddyfile",
        "since 13.0.0-alpha.6 `cargo rullst deploy` writes `encode zstd gzip`, which compresses pages now that the CSRF token in them is masked per response; this `Caddyfile` proxies to `app:3000` without `encode`, so its pages stay uncompressed",
        "Nothing breaks if you leave it. To compress pages, first make every page embed `CsrfToken::as_str()` or a `mask_csrf_token` value (fix the `V13-CSRF-COOKIE-IN-PAGE` findings) and no other secret next to text from the request, then add `encode zstd gzip` as the first line of the site block. Static files keep their pre-compressed `.br`/`.zst` siblings.";
    HTMX_2_CLIENT: "V13-HTMX-2", Review, "13.0.0-alpha.5", "htmx 2 client",
        "new applications load htmx 2.0.11 since 13.0.0-alpha.5; this one vendors htmx 1.9.12, which `render_page*` keep loading while it is the only htmx file in `static/`",
        "Nothing breaks if you wait. To move to htmx 2 (the SSE extension `hx-ext=\"sse\"` needs it), copy `htmx-2.0.11.min.js`, `htmx-ext-sse-2.2.4.js`, `HTMX-LICENSE` and `HTMX-EXT-SSE-LICENSE` from a new project's `static/`, point your own layouts at them and delete `static/htmx-1.9.12.min.js`; write `hx-on=\"click: …\"` as `hx-on:click`, and send the CSRF token of `hx-delete` in the `X-CSRF-Token` header (`hx-headers`), because htmx 2 puts DELETE parameters in the query string.";
    MAIL_DEPRECATED_REMOVED: "V13-MAIL-DEPRECATED-REMOVED", MustChange, "13.0.0-alpha.6", "Mail deprecated constructors and tracking helpers removed",
        "`ResendDriver::new`, `AwsSesDriver::new`, `Message::with_open_tracking`/`with_click_tracking` and the infallible `TrackingEngine` helpers, deprecated in 12.0.0, were removed in 13.0.0-alpha.6; this code no longer compiles",
        "Use the fallible forms, which report invalid configuration instead of failing later or returning an empty token or the unchanged HTML: `ResendDriver::try_new(api_key)?`, `AwsSesDriver::try_new(region, token)?`, `message.try_with_open_tracking(url, secret, campaign)?`, `.try_with_click_tracking(url, secret)?`, and `TrackingEngine::try_generate_open_token`, `try_generate_click_token`, `try_inject_open_pixel` and `try_rewrite_links`.";
    IOT_OTA_LEGACY_REMOVED: "V13-IOT-OTA-LEGACY-REMOVED", Review, "13.0.0-alpha.6", "IoT legacy OTA methods removed",
        "`OtaManager::new`, `verify_signature` and `commit_update`, which always failed, and `OtaError::LegacyApiUnsupported` were removed in 13.0.0-alpha.6",
        "Build the manager with `OtaManager::new_with_trusted_key(target, version, rollback_counter, public_key)?` (or `new_with_counter_store`), verify an image with `begin_update` or `verify_update(&manifest, firmware, signature)`, commit it with `commit_verified_update()`, and remove arms that match `LegacyApiUnsupported`. A `verify_signature` or `commit_update` of another type is unaffected.";
    NEXUS_INFALLIBLE_BUILD_REMOVED: "V13-NEXUS-INFALLIBLE-BUILD-REMOVED", Review, "13.0.0-alpha.6", "Nexus `build` and `into_router` removed",
        "`Nexus::build` and `Nexus::into_router`, deprecated in 12.0.0, were removed in 13.0.0-alpha.6; on a Nexus builder this call no longer compiles",
        "End the Nexus builder with `.try_build()?` (or match its `NexusBuildError`), so a missing or invalid access policy stops start-up instead of mounting a router that answers 503. A `.build()` of another builder in the file is unaffected.";
    AI_DEPRECATED_REMOVED: "V13-AI-DEPRECATED-REMOVED", Review, "13.0.0-alpha.6", "AI `structured_prompt` and `is_safe` removed",
        "`AiClient::structured_prompt` and `GuardrailReport::is_safe`, deprecated in 12.0.0, were removed in 13.0.0-alpha.6",
        "Replace `structured_prompt::<T>(text)`, which always returned `AiError::UnsupportedCapability`, with `json_prompt::<T>(text)` for parseable JSON or `structured_prompt_with_schema::<T>(text, &schema)` for output the provider checks against a JSON Schema, and `report.is_safe()` with `report.passed_heuristics()`.";
    SECURITY_DEPRECATED_REMOVED: "V13-SECURITY-DEPRECATED-REMOVED", MustChange, "13.0.0-alpha.6", "Security `with_distributed` and `AuditChain::new` removed",
        "`RateLimiter::with_distributed` and `AuditChain::new`, deprecated in 12.0.0, were removed in 13.0.0-alpha.6; this code no longer compiles",
        "Replace `AuditChain::new(key, logger)` with `AuditChain::try_new(key, logger)?`, which rejects a weak key at start-up. Remove `.with_distributed()`: it selected a backend that does not exist, so every request was refused; keep the process-local `RateLimiter`, or share a budget between processes with `RedisRateLimiter` (feature `redis-rate-limit`).";
    QUEUE_PURGE_COMPLETED_REMOVED: "V13-QUEUE-PURGE-COMPLETED-REMOVED", Review, "13.0.0-alpha.6", "Queue `purge_completed_jobs` removed",
        "`Queue::purge_completed_jobs`, deprecated in 12.0.0, was removed in 13.0.0-alpha.6; on a `Queue` this call no longer compiles",
        "Call `queue.purge_failed_jobs()`, which does what the alias did (it removed failed jobs), or `purge_completed_history()` to remove completed jobs where the driver keeps them. `QueueDriver::purge_completed_jobs`, the hook a custom driver implements, is unchanged.";
    CONNECT_INFALLIBLE_NEW_REMOVED: "V13-CONNECT-INFALLIBLE-NEW-REMOVED", MustChange, "13.0.0-alpha.6", "Connect infallible `new` constructors removed",
        "the infallible `new` constructors of the Connect providers, built in or defined with `define_provider!`, and `CredentialMode::Invalid`, deprecated in 12.0.0, were removed in 13.0.0-alpha.6; this code no longer compiles",
        "Call `try_new` with the same arguments (`GithubProvider::try_new(client_id, secret, redirect_url)?`) and handle its `ConnectError::InvalidConfiguration`, which names the invalid field, instead of mounting a disabled provider whose redirect is `about:blank`. Remove checks of `CredentialMode::Invalid` and `is_invalid()`: a provider that builds is live or mock.";
    ROUTE_MARKER_REMOVED: "V13-ROUTE-MARKER-REMOVED", MustChange, "13.0.0-alpha.6", "`#[route]` marker removed",
        "the `#[route]` marker attribute (`rullst::route`), deprecated in 12.0.0, was removed in 13.0.0-alpha.6; this code no longer compiles",
        "Delete the attribute and its import: it never registered anything. Register the handler with `rullst::routes!` (`get(\"/path\" => handler)`).";
    ORM_SCHEMA_DIFF_REMOVED: "V13-ORM-SCHEMA-DIFF-REMOVED", MustChange, "13.0.0-alpha.6", "ORM `schema_diff` removed",
        "`rullst_orm::schema_diff` (`extract_tables_from_ast`, `ParsedTable`, `ParsedField`), deprecated in 13.0.0-alpha.5, was removed in 13.0.0-alpha.6; this code no longer compiles",
        "It has no library replacement: write a migration from the models with `cargo rullst make:migration:auto`, or read their tables and columns with `cargo rullst inspect schema --json`.";
    ORM_SAVE_MODE: "V13-ORM-SAVE-MODE", MustChange, "13.0.0-alpha.6", "ORM explicit save mode",
        "since 13.0.0-alpha.6 a model declares exactly one save mode, a `#[sqlx(skip)]` `RowSnapshot` field or `#[orm(full_row_save)]` (a Turso-primary model only the attribute); this one does not, so it no longer compiles",
        "Add `#[sqlx(skip)] pub snapshot: RowSnapshot,` (`rullst::orm::RowSnapshot`) so `save()` writes only the columns the record changed and concurrent edits of other columns persist; set `snapshot: RowSnapshot::default()` in the model's struct literals and add `#[serde(skip)]` when it derives Serde. Or add `full_row_save` to its `#[orm(...)]` to keep writing every column, which overwrites a change that another request made to another column since the record was read. Keep one of the two; a Turso-primary model takes `full_row_save` and no `RowSnapshot` field.";
    PASSKEYS_FEATURE: "V13-AUTH-PASSKEYS-FEATURE", MustChange, "13.0.0-alpha.6", "Passkeys feature",
        "this code uses Rullst's passkeys (`auth::passkey`), which since 13.0.0-alpha.6 compile only with `rullst-auth`'s `passkeys` feature, and no manifest here enables it",
        "Enable `auth-passkeys` on `rullst` (`auth-sqlite` and `auth-passkey-postgres` include it), or `passkeys` on a direct `rullst-auth` dependency (`sqlite` and `passkey-postgres` include it).";
    MEDIA_PROCESSING_MATCH: "V13-MEDIA-PROCESSING-MATCH", MustChange, "13.0.0-alpha.6", "Media processing states",
        "`rullst-media`'s `Processing` is `#[non_exhaustive]` since 13.0.0-alpha.6 and gained `Playable`, which Bunny's API status 8 (`JitPlaylistsCreated`, formerly `Processing`) now maps to; a `match` on it without a wildcard arm no longer compiles",
        "Add an arm for `Processing::Playable` and a wildcard arm (`_ => ...`). `Playable` streams in Bunny's player before encoding finishes: show it as watchable where you embed that player (`PlaybackKind::Embed`) and as processing elsewhere; `Ready` still means finished. `processing.is_playable()` and `processing.allows(kind)` answer both questions without a `match`.";
    NON_EXHAUSTIVE_MATCH: "V13-NON-EXHAUSTIVE-MATCH", MustChange, "13.0.0-alpha.6", "Non-exhaustive enums and structs",
        "this `match` names a Rullst enum that is `#[non_exhaustive]` since 13.0.0-alpha.6 (an error, status, kind, mode or backend that a 13.x release may extend) and has no wildcard arm, so it no longer compiles",
        "Add a wildcard arm (`_ => ...`): report an unknown error generically, and treat an unknown status, kind or mode as the most cautious one the code handles. The authorization and decision enums of Labs (`Action`, `Teardown`, `LeaseStatus`) and Privacy (`AgeDecision`, `AgeOutcome`, `Assurance`, `AgeDeclaration`) stay exhaustive on purpose, so a new variant there will be a compile error to handle.";
    NON_EXHAUSTIVE_LITERAL: "V13-NON-EXHAUSTIVE-LITERAL", MustChange, "13.0.0-alpha.6", "Non-exhaustive enums and structs",
        "this struct literal builds a Rullst struct that is `#[non_exhaustive]` since 13.0.0-alpha.6, which code outside its crate cannot build with a literal (also with `..Default::default()`), so it no longer compiles",
        "Build it with its constructor and set optional fields on the value: `FieldMeta::new(name, label, kind).hidden().readonly()`, `Message::new().to(..).subject(..)` (Mail), `ExchangeParams::new(code).with_code_verifier(verifier)`, `ConnectUser::new(id, name, access_token)`, `HttpResponse::new(status, body)`, `WebhookEvent::new(..)`, `WebhookEventRecord::new(..)`, `RevenueMetrics::new(..)`, `QueuedJob::new(..)`, `QueuedJobDetail::new(..)`, `TraceSpan::new(..)`, `ToolParam::new(..)`, `Passkey::new(..)`, `PaginationResult::new(..)`, `PrivacyReport::new(..)`, `RegistryEntry::new(..)`, `NexusState::new(..)`, `TimingGuardConfig::new(..)`, `RagRetrievalError::new(..)`; start an `AuthCallback` or `ClaimedOutboxEvent` from `default()`, and `ProviderCapabilities` from `PORTABLE` or `NONE`, then assign fields. Reports, events and snapshots that Rullst returns are read, not built: a test deserializes one or takes it from the call that returns it.";
    NON_EXHAUSTIVE_LITERAL_GLOB: "V13-NON-EXHAUSTIVE-LITERAL-GLOB", Review, "13.0.0-alpha.6", "Non-exhaustive enums and structs",
        "this struct literal has the name and fields of a Rullst struct that is `#[non_exhaustive]` since 13.0.0-alpha.6, in a file that does not name the struct's crate but imports a glob (`use super::*;`); if the glob brings in Rullst's type, the literal no longer compiles",
        "If the compiler reports E0639 here, the type is Rullst's: build it with its constructor, as `V13-NON-EXHAUSTIVE-LITERAL` describes (`HttpResponse::new(status, body)`, `FieldMeta::new(name, label, kind)`, `ExchangeParams::new(code)`, …). An application type of the same name needs no change; mark the line `rullst-upgrade: reviewed V13-NON-EXHAUSTIVE-LITERAL-GLOB`.";
    SCHEDULER_CRON_FIELDS: "V13-SCHEDULER-CRON-FIELDS", Review, "13.0.0-alpha.6", "Core scheduler cron fields",
        "since 13.0.0-alpha.6 `Scheduler` evaluates cron expressions with Core's own evaluator, which checks each of the five fields on its own; this expression has a field that starts or ends with `,`, `-` or `/`, so registering it now fails with `SchedulerError::InvalidCron`",
        "Remove the stray space (`1, 2 * * * *` becomes `1,2 * * * *`) and check that the schedule still means what you intended: the `cron` crate joined such a field with its neighbour, so `1, 2 * * *` ran at minutes 1 and 2 of every hour from January to July only. `InvalidCron` names the field.";
    CSRF_TOKEN_COMPARED: "V13-CSRF-TOKEN-COMPARED", Review, "13.0.0-alpha.6", "CSRF token masked per response",
        "since 13.0.0-alpha.6 the CSRF token a page embeds (`CsrfToken::as_str()`) is masked with a fresh mask in every response and no longer equals the `rullst_csrf` cookie; this code checks a token against the cookie's value (`==`, `assert_eq!`, `.contains(..)` or `.ct_eq(..)`)",
        "Submit the page's value and check that it is accepted (not `403`) instead of comparing it with the cookie; it is 86 characters long for the 32-character cookie token. A check of your own that compares a submitted token with the cookie refuses the masked value: leave that check to the CSRF middleware, which accepts both. Code that only sends the cookie value in `X-CSRF-Token` or `_token` needs no change.";
    JSONWEBTOKEN_12: "V13-JSONWEBTOKEN-12", Review, "13.0.0-alpha.6", "Dependency majors: `ed25519-dalek` 3, `jsonwebtoken` 12, `jsonschema` 0.61",
        "Rullst's Connect and Auth (`jwt`) use `jsonwebtoken` 12 since 13.0.0-alpha.6, whose `EncodingKey::from_secret` and `DecodingKey::from_secret` return a `Result`; this project declares `jsonwebtoken` 11 or earlier, which keeps building beside Rullst's 12, and this call uses the infallible form",
        "Nothing breaks while the requirement stays. To build one copy, declare `jsonwebtoken = { version = \"12\", default-features = false, features = [\"aws_lc_rs\"] }` and handle the `Result` of `from_secret` (an empty key is refused), as the `make:cors-jwt` middleware generated since 13.0.0-alpha.6 does: `let Ok(key) = DecodingKey::from_secret(secret) else { return StatusCode::INTERNAL_SERVER_ERROR.into_response(); };`.";
    ED25519_DALEK_3: "V13-ED25519-DALEK-3", Review, "13.0.0-alpha.6", "Dependency majors: `ed25519-dalek` 3, `jsonwebtoken` 12, `jsonschema` 0.61",
        "Rullst's IoT and Connect use `ed25519-dalek` 3 since 13.0.0-alpha.6; this manifest declares `ed25519-dalek` 2 or earlier, so Cargo builds a second copy, and the features it enables, such as the precomputed tables of `fast`, no longer reach Rullst's copy",
        "Declare `ed25519-dalek = { version = \"3\", default-features = false, features = [\"fast\"] }`, with the other features the application uses, to keep one copy and the faster signature checks; code that calls `ed25519-dalek` itself may need its 3.x API.";
    SQLITE_WAL_AVAILABLE: "V13-SQLITE-WAL-AVAILABLE", Review, "13.0.0-alpha.6", "SQLite WAL journal",
        "since 13.0.0-alpha.6 SQLite projects can opt in to the WAL journal with `sqlite_journal_mode = \"wal\"` under `[database]` in `Rullst.toml`, which new projects set so that readers stop locking the file for every statement; this project uses SQLite without the setting and keeps the journal mode recorded in its file",
        "Nothing changes unless you opt in. Before adding `sqlite_journal_mode = \"wal\"`, back up with `sqlite3 <file> \".backup <copy>\"` instead of copying the file, keep `<file>-wal` and `<file>-shm` on the same volume, and keep the database on a local disk of the one host that runs the application (never NFS, SMB or Azure Files). To keep the rollback journal, set `sqlite_journal_mode = \"delete\"` or mark the line `rullst-upgrade: reviewed V13-SQLITE-WAL-AVAILABLE`.";
    AUTH_SESSION_REGISTRY: "V13-AUTH-SESSION-REGISTRY", Review, "13.0.0-alpha.6", "Server-side login sessions",
        "since 13.0.0-alpha.6 `cargo rullst auth` and the SaaS starter record every sign-in in a server-side session registry, so signing out ends every copy of the cookie; this file, generated before alpha.6, issues or accepts session-epoch cookies (`make_login_cookie_with_epoch`, `decrypt_session_claims`) and no manifest enables a session registry feature, so a copied cookie stays valid until it expires",
        "Nothing breaks if you keep epoch cookies. To adopt the registry, follow the session step of the alpha.5 to alpha.6 checklist (https://rullst.github.io/Rullst/book/migration-v13.html#upgrading-an-alpha5-application-to-alpha6): enable `auth-session-registry-sqlite`, `-postgres` or `-mysql`, add the `rullst_auth_sessions` migration, sign in with `start_session` and check requests with `authenticate_request` in `src/middlewares/auth_middleware.rs`, sign out with `sign_out`, and revoke the account's sessions where the epoch changes; a project that the alpha.6 CLI generates with the same starter and database has every file to compare with.";
    TELEMETRY_LAYER_TYPE: "V13-TELEMETRY-LAYER-TYPE", MustChange, "13.0.0-alpha.6", "Distributed tracing layer type",
        "`DistributedTelemetry::layer` returns an opaque `impl Layer<S>` since 13.0.0-alpha.6, not `tracing_opentelemetry::OpenTelemetryLayer`; code that names that type or calls its own methods on the layer no longer compiles",
        "Name the layer `impl tracing_subscriber::Layer<S>`, or box it (`.boxed()`, `Box<dyn Layer<S> + Send + Sync>`) where a type must be written, and drop `OpenTelemetryLayer`'s own builder calls (`with_location`, `with_threads`, `with_level`, `with_target`, …): the minimized pipeline exports no span attributes or events. `Layer` methods such as `.with_filter(..)` keep working.";
    VALIDATION_FIELD_ERRORS: "V13-VALIDATION-FIELD-ERRORS", MustChange, "13.0.0-alpha.6", "Validation field errors",
        "`ValidationError::ValidationError`'s `errors` is `rullst::validation::FieldErrors` since 13.0.0-alpha.6, not `validator::ValidationErrors`; code that reads it with `validator`'s API or builds the variant from `validator` errors no longer compiles",
        "Read the failures with `errors.messages()` or `errors.codes()` (field path to messages or codes) and `errors.is_empty()`; reach `validator`'s own errors with `errors.as_any().downcast_ref::<validator::ValidationErrors>()`. Build the variant from `FieldErrors::validate(&payload)` or `FieldErrors::default()` and `add(field, code, message)`.";
    STUDIO_OPENAPI_INFERRED: "V13-STUDIO-OPENAPI-INFERRED", MustChange, "13.0.0-alpha.6", "Studio OpenAPI documents",
        "`Studio::with_openapi` and `api_playground::router` take any `serde::Serialize` document since 13.0.0-alpha.6, so an argument whose type was inferred from the former `utoipa::openapi::OpenApi` parameter no longer compiles",
        "Name the document's type: `utoipa::openapi::OpenApi::default()`, `serde_json::from_str::<serde_json::Value>(..)?` or `ApiDoc::openapi()`; a value of a named type, of any utoipa version, needs no change.";
    ORM_CHRONO_TYPES: "V13-ORM-CHRONO-TYPES", MustChange, "13.0.0-alpha.6", "`chrono` replaced by `jiff`",
        "Rullst's ORM no longer uses chrono since 13.0.0-alpha.6 (RUSTSEC-2026-0543 marks it unmaintained): `rullst::orm::chrono` is gone, and model fields, Nexus models and `RullstValue` take `rullst::orm::Timestamp`, `DateTime`, `Date` and `Time`, built on jiff, instead of chrono's `DateTime<Utc>`, `NaiveDateTime`, `NaiveDate` and `NaiveTime`; this code uses chrono there, so it no longer compiles",
        "Replace `DateTime<Utc>` with `rullst::orm::Timestamp`, `NaiveDateTime` with `rullst::orm::DateTime`, `NaiveDate` with `rullst::orm::Date` and `NaiveTime` with `rullst::orm::Time`, inside `Option` too: they store the same bytes in the same columns, so no data migration is needed. Build them from jiff (`rullst::orm::jiff`): `Timestamp::now()` for `Utc::now()`, `jiff::civil::date(2026, 11, 17).into()` for a date, `.parse()` for text; they dereference to jiff's types (`at.as_second()` for `.timestamp()`). Pass them as query values directly; `RullstValue::Timestamp`, `CivilDateTime`, `CivilDate` and `CivilTime` replace the chrono variants. Code that keeps chrono for its own work declares `chrono` itself and converts where it meets Rullst; its own SQLx queries that bind or read chrono values (and `sqlx::types::chrono`) need the `chrono` feature on its own `sqlx` dependency.";
    LOCALE_JIFF_DATES: "V13-LOCALE-JIFF-DATES", MustChange, "13.0.0-alpha.6", "`chrono` replaced by `jiff`",
        "`Locale::format_date`, `format_time` and `format_datetime` take a jiff value since 13.0.0-alpha.6 (`impl Into<jiff::civil::Date>`, `Time` or `DateTime`) instead of a reference to a chrono `Datelike` or `Timelike`; this call passes a chrono value, so it no longer compiles",
        "Pass a jiff civil value by value, or `&zoned` for a `jiff::ZonedDateTime`: `locale.format_date(date, DateStyle::Short)`, `locale.format_datetime(&zoned, DateStyle::Long, TimeStyle::Short)`. Convert a UTC `Timestamp` to the visitor's zone first (`at.in_tz(\"America/Sao_Paulo\")?` or `at.to_zoned_datetime(zone)`). A chrono value the application keeps converts with `jiff::civil::date(d.year() as i16, d.month() as i8, d.day() as i8)`; the formatted text is unchanged.";
    MAIL_JIFF_TIMESTAMPS: "V13-MAIL-JIFF-TIMESTAMPS", MustChange, "13.0.0-alpha.6", "`chrono` replaced by `jiff`",
        "Mail's `Message::send_at` and its `send_at` field, `MailAssertion::with_scheduled_at` and the `expires_at` of `AccountEvent::PasswordResetAt` and `EmailLoginAt` hold a `jiff::Timestamp` since 13.0.0-alpha.6 instead of chrono's `DateTime<Utc>`; this code passes or reads one, so it no longer compiles",
        "Pass a `jiff::Timestamp` (`rullst::orm::jiff` or your own `jiff = \"1\"`) or anything that converts into one, such as `rullst::orm::Timestamp`, so a model's field passes directly: `Timestamp::now()` for `Utc::now()`, `Timestamp::now() + SignedDuration::from_mins(5)` (or `checked_add`) for `Utc::now() + Duration::minutes(5)`, and read it with jiff's API (`at.as_second()` for `.timestamp()`). A queued message's JSON is unchanged, so messages queued before the upgrade still deliver at their time.";
    CAPITAL_INVOICE_JIFF_DATE: "V13-CAPITAL-INVOICE-JIFF-DATE", MustChange, "13.0.0-alpha.6", "`chrono` replaced by `jiff`",
        "Capital's `Invoice::new` takes the invoice date as `impl Into<jiff::Timestamp>` since 13.0.0-alpha.6, and `Invoice.date` holds a `jiff::Timestamp`, instead of chrono's `DateTime<Utc>`; this call passes a chrono value, so it no longer compiles",
        "Pass `jiff::Timestamp::now()` (or the instant the payment settled) instead of `Utc::now()`; a `rullst::orm::Timestamp`, such as a model's field, converts directly. Read `invoice.date` with jiff's API (`invoice.date.as_second()` for `.timestamp()`, `invoice.date.strftime(\"%d/%m/%Y\")` for `.format(..)`). The invoice's JSON and rendered receipt are unchanged.";
    MESSAGING_CRON_POSIX: "V13-MESSAGING-CRON-POSIX", MustChange, "13.0.0-alpha.6", "Messaging schedules use the scheduler's cron syntax",
        "`RecurringDefinition::new` reads a new definition's cron expression with Core's POSIX syntax since 13.0.0-alpha.6, as `Scheduler` does: weekdays 0 to 7 (0 and 7 are Sunday) instead of 1 (Sunday) to 7 (Saturday), a day that matches either day field when both are restricted instead of both, and each field valid on its own; definitions stored by an earlier version keep their old meaning. Review the schedule this call creates",
        "If an earlier version already stored this schedule (its `RecurringMetadata::syntax()`, from `create` or `schedules(after, limit)`, is `CronSyntax::Legacy`), leave its expression exactly as it is: the stored definition keeps its old meaning (deprecated until 14.0), and a changed expression under the same name makes `PostgresRecurringStore::create` fail with `RecurringError::LegacyDefinition`. To migrate it, cancel it and create the POSIX expression under a new name (`RecurringDefinition::posix_equivalent()` of the stored definition, deserialized from its JSON, gives it when one exists); a cancelled name is never reused. For a schedule first created after the upgrade, write POSIX: move each weekday number down by one (`2-6`, Monday to Friday, becomes `1-5`; names such as `MON-FRI` stay), and where both the day of the month and the weekday are restricted, restrict one and check the other when the message is handled, since a day that matches either now runs it. A field that starts or ends with `,`, `-` or `/` (`1, 2 * * *`, `/5 * * * *`) or runs a number into a name (`1jan`) only the old syntax read: `RecurringDefinition::new` refuses it with `RecurringError::InvalidInput`, also for a stored schedule, so stop creating that one in code or recreate it under a new name. A string literal's note gives its POSIX form when the rewrite is mechanical.";
    LABS_VARIANTS_RENAMED: "V13-LABS-VARIANTS-RENAMED", MustChange, "13.0.0-rc.1", "Labs pinned-runner names",
        "`rullst-labs` renamed `ExecutionProfile::LinuxExperimental` to `ExecutionProfile::PinnedRunner` and `ResultEvidence::Experimental` to `ResultEvidence::SignedReceipt` in 13.0.0-rc.1; this code names an old variant, so it no longer compiles",
        "Rename the path: `ExecutionProfile::PinnedRunner { tools, receipt_key }` and `ResultEvidence::SignedReceipt { receipt, observations }`; fields and behaviour are unchanged. Change only Rust code: both variants are still serialized as `LinuxExperimental` and `Experimental`, so existing stores, jobs, digests and receipts keep working, and a JSON or TOML configuration that names the profile keeps `\"LinuxExperimental\"`.";
    IOT_SIMULATORS_REMOVED: "V13-IOT-SIMULATORS-REMOVED", MustChange, "13.0.0-rc.1", "IoT simulator fixtures removed",
        "`rullst-iot` removed the `experimental-simulators` feature and its fixtures (`SimulatedHsmDevice`, `SimulatedHsmProfile`, `SimulatedPqcFixture`, `SimulatedMqttPayloadFormatter`) in 13.0.0-rc.1; Cargo refuses the feature, and this code no longer compiles",
        "Remove `experimental-simulators` from the `rullst-iot` dependency or `[features]` entry, and give a test that used a fixture its own fixed bytes: a SHA-256 of a fixed seed (`sha2::Sha256::digest(seed)`) for the HSM and PQC fixtures, `telemetry.value.to_string()` for the formatter. They were never an HSM, post-quantum cryptography or an MQTT client.";
}

/// Review rules that name APIs v13 removed: code using them no longer
/// compiles, but a flagged name can also be an application's own type, so
/// they are not must-change. A failed upgrade gate repeats their guidance.
pub(crate) const REMOVED_API_RULES: &[&Rule] = &[
    &CAPITAL_REMOVED,
    &MAIL_REMOVED,
    &HIDDEN_INTERNALS,
    &STORE_COLLECTIONS,
    &SECURITY_GLOBALS,
    &CONNECT_JWKS,
    &IOT_OTA_LEGACY_REMOVED,
    &NEXUS_INFALLIBLE_BUILD_REMOVED,
    &AI_DEPRECATED_REMOVED,
    &QUEUE_PURGE_COMPLETED_REMOVED,
];
