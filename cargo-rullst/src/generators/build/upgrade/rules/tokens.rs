//! Expression-level rules over a file's token stream, including macro bodies.
//!
//! The stream comes from the parsed file, so comments are gone; attributes
//! (and doc comments, which are `#[doc]` attributes) are skipped; a string
//! literal only ever matches the string rules. Calls, method chains and
//! macro invocations are recognised from their token shape.

use super::catalog::*;
use super::rust::FileContext;
use super::{Collector, Rule};
use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};

/// Identifiers whose presence (in a path, type, call or method) is enough.
const IDENT_RULES: &[(&str, &Rule)] = &[
    ("SqlRecoveryStore", &AUTH_RECOVERY_MIGRATE),
    ("init_telemetry", &OTLP_ENVIRONMENT),
    ("retry_failed_job", &QUEUE_SEMANTICS),
    ("job_timeout", &QUEUE_SEMANTICS),
    ("PresenceTracker", &PRESENCE_COUNTING),
    ("get_any_value_as_string", &STUDIO_TABLE_VALUES),
    ("ValidatedForm", &VALIDATION_STATUS),
    ("ValidatedJson", &VALIDATION_STATUS),
    ("MemoryFeatureDriver", &MEMORY_FEATURE_SPLITS),
    ("override_variants", &MEMORY_FEATURE_SPLITS),
    ("DbFeatureDriver", &DB_FEATURE_SPLITS),
    ("GeminiProvider", &GEMINI_STOP_REASONS),
    ("RagPipeline", &RAG_TENANT_TAGS),
    ("AnthropicProvider", &ANTHROPIC_OUTPUT),
    ("OpenAiProvider", &OPENAI_OUTPUT),
    ("with_machine_endpoints", &MACHINE_ENDPOINTS),
    ("enable_pii_masking", &PII_MASKING),
    ("DlpResponseLayer", &SECURITY_LAYERS),
    ("HoneypotLayer", &SECURITY_LAYERS),
    ("HoneypotState", &SECURITY_LAYERS),
    ("RaspSecurityLayer", &SECURITY_LAYERS),
    ("redact_secrets", &SECURITY_LAYERS),
    ("OidcProvider", &OIDC_OPTIONAL_NAME),
    ("update_partial", &ORM_PARTIAL_UPDATE),
    ("query_key", &ORM_CACHE_KEY),
    ("rollback_last", &TURSO_ROLLBACK),
    ("sorted_set_top", &REDIS_MOCK_ORDER),
    ("diagnose_sql_error", &AUTO_HEALING),
    ("force_delete", &ORM_MISSING_ROW),
    ("only_trashed", &ORM_ONLY_TRASHED),
    ("to_sql", &ORM_SQL_TEXT),
    ("compliance_schema", &PERSONAL_DATA_REPORT),
    ("has_encrypted_data", &PERSONAL_DATA_REPORT),
    ("basic_from_env", &NEXUS_DOTENV),
    ("local_development_or_basic_from_env", &NEXUS_DOTENV),
    ("parse_webhook_payload", &WISE_WEBHOOK),
    ("SqlQuotaStore", &QUOTA_KEYS),
    ("quota_request", &ZERO_TIER),
    ("OrmOutboxRelay", &OUTBOX_RELAY_KEY),
    ("LocalAttachmentInspector", &MAIL_ATTACHMENTS),
    ("ResendDriver", &MAIL_RESEND_SCHEDULE),
    ("try_with_open_tracking", &MAIL_TRACKING),
    ("try_with_click_tracking", &MAIL_TRACKING),
    ("with_open_tracking", &MAIL_TRACKING),
    ("with_click_tracking", &MAIL_TRACKING),
    ("OpenEvent", &MAIL_TRACKING),
    ("ClickEvent", &MAIL_TRACKING),
    ("strip_html_to_plain_text", &MAIL_TEXT_FALLBACK),
    ("record_progress", &LMS_RECORD_PROGRESS),
    ("store_order", &ERP_STORE_ORDER),
];

/// Keywords that end an expression chain and never start a call path.
const KEYWORDS: &[&str] = &[
    "as", "async", "break", "const", "continue", "dyn", "else", "enum", "extern", "false", "fn",
    "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
    "return", "static", "struct", "trait", "true", "type", "unsafe", "use", "where", "while",
];

/// What the token scan learned beyond its findings.
#[derive(Debug, Default)]
pub(super) struct TokenFacts {
    pub server_start: Option<usize>,
    pub mounts_health: bool,
    pub protects_router: bool,
    pub root_route: Option<usize>,
}

pub(super) fn scan(
    stream: TokenStream,
    context: &FileContext<'_>,
    out: &mut Collector,
) -> TokenFacts {
    let mut scanner = Scanner {
        context,
        out,
        facts: TokenFacts::default(),
    };
    scanner.level(stream, None);
    scanner.facts
}

struct Scanner<'a, 'b> {
    context: &'a FileContext<'a>,
    out: &'b mut Collector,
    facts: TokenFacts,
}

fn line(tree: &TokenTree) -> usize {
    tree.span().start().line
}

fn is_punct(tree: Option<&TokenTree>, expected: char) -> bool {
    matches!(tree, Some(TokenTree::Punct(punct)) if punct.as_char() == expected)
}

fn is_group(tree: Option<&TokenTree>, delimiter: Delimiter) -> bool {
    matches!(tree, Some(TokenTree::Group(group)) if group.delimiter() == delimiter)
}

/// The value of a string literal token, if it is one.
fn string_value(tree: Option<&TokenTree>) -> Option<String> {
    let Some(TokenTree::Literal(literal)) = tree else {
        return None;
    };
    match syn::Lit::new(literal.clone()) {
        syn::Lit::Str(value) => Some(value.value()),
        _ => None,
    }
}

/// `::` at `index` (two joined colons).
fn is_path_separator(tokens: &[TokenTree], index: usize) -> bool {
    matches!(tokens.get(index), Some(TokenTree::Punct(punct)) if punct.as_char() == ':' && punct.spacing() == Spacing::Joint)
        && is_punct(tokens.get(index + 1), ':')
}

/// Attributes whose value runs as JavaScript, as `html!` decides.
pub(super) fn is_event_handler_attribute(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    let htmx = name.strip_prefix("data-").unwrap_or(&name);
    (name.len() > 2 && name.starts_with("on") && !name.contains('-'))
        || htmx == "hx-on"
        || htmx.starts_with("hx-on-")
}

/// An `==`, `!=` or `=>` that touches the path at `start..end`.
fn compared(tokens: &[TokenTree], start: usize, end: usize) -> bool {
    let before = start
        .checked_sub(1)
        .and_then(|index| tokens.get(index))
        .is_some_and(|tree| matches!(tree, TokenTree::Punct(p) if p.as_char() == '='))
        && start.checked_sub(2).is_some_and(|index| {
            is_punct(tokens.get(index), '=') || is_punct(tokens.get(index), '!')
        });
    let after = (is_punct(tokens.get(end), '=')
        && (is_punct(tokens.get(end + 1), '=') || is_punct(tokens.get(end + 1), '>')))
        || (is_punct(tokens.get(end), '!') && is_punct(tokens.get(end + 1), '='))
        || is_punct(tokens.get(end), '|');
    before || after
}

impl Scanner<'_, '_> {
    fn hit(&mut self, rule: &'static Rule, line: usize) {
        self.out.add(rule, line);
    }

    fn level(&mut self, stream: TokenStream, macro_name: Option<&str>) {
        let tokens: Vec<TokenTree> = stream.into_iter().collect();
        match macro_name {
            Some("html") => self.html_handlers(&tokens),
            Some("routes") => self.root_routes(&tokens),
            _ => {}
        }
        let comparison_macro = matches!(
            macro_name,
            Some("matches" | "assert_eq" | "assert_ne" | "debug_assert_eq" | "debug_assert_ne")
        );
        let mut chain: Vec<String> = Vec::new();
        let mut after_fn = false;
        let mut index = 0;
        while index < tokens.len() {
            let tree = &tokens[index];
            match tree {
                TokenTree::Punct(punct) if punct.as_char() == '#' => {
                    let mut next = index + 1;
                    if is_punct(tokens.get(next), '!') {
                        next += 1;
                    }
                    if is_group(tokens.get(next), Delimiter::Bracket) {
                        index = next + 1;
                        continue;
                    }
                    chain.clear();
                    index += 1;
                }
                TokenTree::Punct(punct) => {
                    if !matches!(punct.as_char(), '.' | '?') {
                        chain.clear();
                    }
                    index += 1;
                }
                TokenTree::Literal(_) => {
                    if let Some(value) = string_value(Some(tree)) {
                        self.string(&value, line(tree));
                    }
                    chain.clear();
                    index += 1;
                }
                TokenTree::Group(group) => {
                    self.level(group.stream(), None);
                    chain.clear();
                    index += 1;
                }
                TokenTree::Ident(ident) => {
                    let name = ident.to_string();
                    if is_punct(index.checked_sub(1).and_then(|i| tokens.get(i)), '.') {
                        self.ident(&name, line(tree));
                        if let Some(TokenTree::Group(args)) = tokens.get(index + 1)
                            && args.delimiter() == Delimiter::Parenthesis
                        {
                            self.method(&name, &chain, args.stream(), line(tree));
                            chain.push(name);
                            self.level(args.stream(), None);
                            index += 2;
                        } else {
                            chain.push(name);
                            index += 1;
                        }
                        continue;
                    }
                    if KEYWORDS.contains(&name.as_str()) {
                        self.ident(&name, line(tree));
                        after_fn = name == "fn";
                        chain.clear();
                        index += 1;
                        continue;
                    }
                    index = self.path(&tokens, index, &mut chain, after_fn, comparison_macro);
                    after_fn = false;
                }
            }
        }
    }

    /// A path starting at `start`; returns the index after what it consumed.
    fn path(
        &mut self,
        tokens: &[TokenTree],
        start: usize,
        chain: &mut Vec<String>,
        after_fn: bool,
        comparison_macro: bool,
    ) -> usize {
        let mut segments = Vec::new();
        let mut end = start;
        while let Some(TokenTree::Ident(ident)) = tokens.get(end) {
            let name = ident.to_string();
            self.ident(&name, ident.span().start().line);
            segments.push(name);
            end += 1;
            if is_path_separator(tokens, end)
                && matches!(tokens.get(end + 2), Some(TokenTree::Ident(_)))
            {
                end += 2;
            } else {
                break;
            }
        }
        let at = line(&tokens[start]);
        self.path_pairs(&segments, tokens, start, end, comparison_macro, at);
        let last = segments.last().cloned().unwrap_or_default();
        // A macro invocation: `name!(...)`, `name![...]` or `name! {...}`.
        if is_punct(tokens.get(end), '!')
            && let Some(TokenTree::Group(body)) = tokens.get(end + 1)
        {
            self.level(body.stream(), Some(&last));
            chain.clear();
            return end + 2;
        }
        if let Some(TokenTree::Group(args)) = tokens.get(end)
            && args.delimiter() == Delimiter::Parenthesis
        {
            if !after_fn {
                self.call(&segments, args.stream(), tokens, end + 1, at);
            }
            self.level(args.stream(), None);
            *chain = vec![last];
            return end + 1;
        }
        *chain = vec![last];
        end
    }

    fn ident(&mut self, name: &str, at: usize) {
        for (ident, rule) in IDENT_RULES {
            if *ident == name {
                self.hit(rule, at);
            }
        }
        match name {
            "health_router" | "health_router_with_lifecycle" => self.facts.mounts_health = true,
            "protect_router" => self.facts.protects_router = true,
            _ => {}
        }
    }

    fn path_pairs(
        &mut self,
        segments: &[String],
        tokens: &[TokenTree],
        start: usize,
        end: usize,
        comparison_macro: bool,
        at: usize,
    ) {
        for pair in segments.windows(2) {
            match (pair[0].as_str(), pair[1].as_str()) {
                ("FieldKind", "Number") if comparison_macro || compared(tokens, start, end) => {
                    self.hit(&NEXUS_FIELD_KIND_NUMBER, at);
                }
                ("FieldKind", "Url") if self.context.text.contains("captions_url") => {
                    self.hit(&LMS_MEDIA_KINDS, at);
                }
                ("QuotaError", "InvalidRequest") => self.hit(&ZERO_TIER, at),
                _ => {}
            }
        }
    }

    fn call(
        &mut self,
        segments: &[String],
        args: TokenStream,
        tokens: &[TokenTree],
        after: usize,
        at: usize,
    ) {
        let names: Vec<&str> = segments.iter().map(String::as_str).collect();
        let args: Vec<TokenTree> = args.into_iter().collect();
        let first_string = string_value(args.first());
        match names.as_slice() {
            [.., "Storage", "r2"] => self.hit(&R2_PUBLIC_URL, at),
            [.., "Scheduler", "new"] => self.hit(&SCHEDULER_WEEKDAYS, at),
            [.., "Server", "new" | "new_hot"] => {
                self.facts.server_start.get_or_insert(at);
            }
            [.., "render_page"] => self.hit(&PAGE_LANGUAGE, at),
            [.., "Mail", _] => self.hit(&MAIL_FACADE, at),
            [.., "Profile", "find"] if args.len() == 1 && args[0].to_string() == "1" => {
                self.hit(&PROFILE_FIND, at);
            }
            [
                ..,
                "Schema",
                "create" | "drop_if_exists" | "drop" | "table" | "alter",
            ] if first_string
                .as_deref()
                .is_some_and(|table| table.bytes().any(|byte| byte.is_ascii_uppercase())) =>
            {
                self.hit(&SCHEMA_TABLE_CASE, at);
            }
            [.., "env", "var" | "var_os"]
                if first_string
                    .as_deref()
                    .is_some_and(|key| key.starts_with("BILLING_")) =>
            {
                self.hit(&BILLING_SETTINGS, at);
            }
            [.., "hash_password"] if !self.context.text.contains("credential_rate_limit") => {
                self.hit(&CREDENTIAL_RATE_LIMIT, at);
            }
            [.., model, "all"]
                if args.is_empty()
                    && model.starts_with(|c: char| c.is_ascii_uppercase())
                    && is_punct(tokens.get(after), '.')
                    && matches!(tokens.get(after + 1), Some(TokenTree::Ident(i)) if i == "await") =>
            {
                self.hit(&MODEL_ALL, at);
            }
            _ => {}
        }
    }

    fn method(&mut self, name: &str, chain: &[String], args: TokenStream, at: usize) {
        let has = |names: &[&str]| chain.iter().any(|link| names.contains(&link.as_str()));
        match name {
            "remember"
                if has(&["query"])
                    || chain
                        .iter()
                        .any(|link| link.starts_with("where") || link.starts_with("filter")) =>
            {
                self.hit(&ORM_CACHE_PREFIX, at);
            }
            "delete_all" if has(&["with_trashed", "only_trashed"]) => {
                self.hit(&ORM_TRASHED_DELETE_ALL, at);
            }
            "chunk" | "chunk_with_tx" | "chunk_by_id" | "chunk_by_id_with_tx"
                if has(&["limit", "offset"]) =>
            {
                self.hit(&ORM_CHUNK_BOUNDS, at);
            }
            "restore" if args.is_empty() => self.hit(&ORM_MISSING_ROW, at),
            "with_openapi" if self.context.facts.utoipa_below_6 => self.hit(&UTOIPA_6, at),
            "paginate" if !self.context.text.contains("MAX_PAGE") => self.hit(&PAGE_BOUNDS, at),
            _ => {}
        }
    }

    fn string(&mut self, value: &str, at: usize) {
        if value.eq_ignore_ascii_case("no-referrer") {
            self.hit(&REFERRER_POLICY, at);
        }
        if value.contains("datetime('now')") {
            self.hit(&SQLITE_SEED_TIME, at);
        }
        if value.starts_with("/_rullst/") {
            self.hit(&DEV_TELEMETRY_ROUTE, at);
        }
        if value.contains("progress:{") && value.contains(":next") {
            self.hit(&LMS_PROGRESS_KEY, at);
        }
        // Relative sitemap URLs in a robots.txt body or a sitemap document.
        if value.contains("Sitemap: /") || value.contains("<loc>/") {
            self.hit(&BLOG_SITEMAP, at);
        }
        if value.starts_with("OTEL_EXPORTER_OTLP_") {
            self.hit(&OTLP_ENVIRONMENT, at);
        }
    }

    /// `name={expr}` attributes at the top level of an `html!` body.
    fn html_handlers(&mut self, tokens: &[TokenTree]) {
        for (index, tree) in tokens.iter().enumerate() {
            let TokenTree::Punct(equals) = tree else {
                continue;
            };
            if equals.as_char() != '='
                || equals.spacing() != Spacing::Alone
                || !is_group(tokens.get(index + 1), Delimiter::Brace)
            {
                continue;
            }
            let Some(mut first) = index.checked_sub(1) else {
                continue;
            };
            let TokenTree::Ident(last) = &tokens[first] else {
                continue;
            };
            let mut parts = vec![last.to_string()];
            while first >= 2
                && is_punct(tokens.get(first - 1), '-')
                && let TokenTree::Ident(part) = &tokens[first - 2]
            {
                parts.insert(0, part.to_string());
                first -= 2;
            }
            if is_event_handler_attribute(&parts.join("-")) {
                self.hit(&HTML_EVENT_HANDLER, line(&tokens[first]));
            }
        }
    }

    /// `get("/" => ...)` at the top level of a `routes!` body.
    fn root_routes(&mut self, tokens: &[TokenTree]) {
        for pair in tokens.windows(2) {
            if let (TokenTree::Ident(method), TokenTree::Group(args)) = (&pair[0], &pair[1])
                && method == "get"
                && args.delimiter() == Delimiter::Parenthesis
                && string_value(args.stream().into_iter().next().as_ref()).as_deref() == Some("/")
            {
                self.facts.root_route.get_or_insert(line(&pair[0]));
            }
        }
    }
}
