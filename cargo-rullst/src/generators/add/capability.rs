//! What each `cargo rullst add <capability>` enables, documents and prints.

/// One variable appended to `.env.example`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EnvEntry {
    pub key: &'static str,
    /// A placeholder or `mock_*` value; never a real secret.
    pub value: &'static str,
    /// Written as `# KEY=value`: documented, but not set.
    pub commented: bool,
    /// Comment lines written above the entry (without `# `).
    pub note: &'static [&'static str],
    /// The application cannot start in development without it, so a `.env`
    /// missing it receives the same value.
    pub dev_required: bool,
}

/// Wiring the command never writes itself: the exact code and where it goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Snippet {
    pub place: &'static str,
    pub code: &'static str,
}

/// A facade capability `cargo rullst add` can enable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Capability {
    pub name: &'static str,
    pub summary: &'static str,
    /// The `rullst` feature enabled.
    pub feature: &'static str,
    /// Features of `rullst` that already enable `feature`.
    pub implied_by: &'static [&'static str],
    pub env_title: &'static str,
    pub env: &'static [EnvEntry],
    pub snippet: Snippet,
    /// `(command, purpose)`, at most three.
    pub hints: &'static [(&'static str, &'static str)],
}

const fn entry(key: &'static str, value: &'static str, note: &'static [&'static str]) -> EnvEntry {
    EnvEntry {
        key,
        value,
        commented: false,
        note,
        dev_required: false,
    }
}

const fn documented(key: &'static str, value: &'static str) -> EnvEntry {
    EnvEntry {
        key,
        value,
        commented: true,
        note: &[],
        dev_required: false,
    }
}

pub(crate) const CAPABILITIES: [Capability; 5] = [
    Capability {
        name: "mail",
        summary: "transactional email through the Mail facade",
        feature: "mail",
        implied_by: &[
            "account-mail-postgres",
            "account-mail-sqlite",
            "capital-mail",
            "mail-aws-ses",
            "mail-postgres",
            "mail-smtp",
            "mail-sqlite",
            "mailer",
        ],
        env_title: "Mail",
        env: &[
            entry(
                "MAIL_FROM",
                "\"Example App <no-reply@example.com>\"",
                &[
                    "Default sender for messages without `from`; use an address your provider verified.",
                ],
            ),
            EnvEntry {
                note: &[
                    "Development logs messages while MAIL_DRIVER is unset; staging and",
                    "production must choose a driver, for example:",
                ],
                ..documented("MAIL_DRIVER", "resend")
            },
            documented("RESEND_API_KEY", "mock_resend_key"),
        ],
        snippet: Snippet {
            place: "src/main.rs, in `main` before the server starts (optional fail-fast check)",
            code: "// Validates MAIL_FROM / [mail] from now instead of on the first send.\nrullst::mail::Mail::default_sender().await?;",
        },
        hints: &[
            (
                "cargo rullst make:mail Welcome",
                "scaffold a typed mailable",
            ),
            (
                "cargo rullst dev",
                "send in development; the log driver prints each message",
            ),
        ],
    },
    Capability {
        name: "auth",
        summary: "Argon2 password hashing, encrypted sessions and RBAC helpers",
        feature: "auth",
        implied_by: &[
            "account-mail-postgres",
            "account-mail-sqlite",
            "auth-api-tokens-postgres",
            "auth-api-tokens-sqlite",
            "auth-email-login-postgres",
            "auth-email-login-sqlite",
            "auth-jwt",
            "auth-passkey-postgres",
            "auth-sessions-postgres",
            "auth-sessions-sqlite",
            "auth-sqlite",
        ],
        env_title: "Auth",
        env: &[entry(
            "APP_KEY",
            "REPLACE_WITH_YOUR_32_CHAR_RANDOM_KEY",
            &[
                "Encrypts sessions and cookies. Development falls back to a local",
                "key in .rullst_dev_key; staging and production require a unique value.",
            ],
        )],
        snippet: Snippet {
            place: "the handler that stores or checks a password",
            code: "let hash = rullst::auth::hash_password_async(password).await?;\nlet valid = rullst::auth::verify_password_async(candidate, stored_hash).await;",
        },
        hints: &[
            (
                "cargo rullst auth",
                "scaffold login, registration and sessions",
            ),
            ("cargo rullst make:mfa", "add a TOTP second factor"),
        ],
    },
    Capability {
        name: "ai",
        summary: "the provider-agnostic AI client with guardrails",
        feature: "ai",
        implied_by: &["ai-sql-memory"],
        env_title: "AI",
        env: &[
            entry(
                "OPENAI_API_KEY",
                "mock_openai_key",
                &[
                    "AiClient::auto() reads the process environment and uses the first",
                    "configured provider; no key or a mock_* key selects the offline mock.",
                ],
            ),
            documented("ANTHROPIC_API_KEY", "mock_anthropic_key"),
            documented("GEMINI_API_KEY", "mock_gemini_key"),
            documented("OLLAMA_HOST", "127.0.0.1:11434"),
        ],
        snippet: Snippet {
            place: "the handler or service that calls the model",
            code: "let ai = rullst::ai::AiClient::auto()?;\nlet answer = ai.prompt(\"Summarize this ticket\").await?;",
        },
        hints: &[
            (
                "cargo rullst make:chat-session",
                "persist conversations for a chat UI",
            ),
            (
                "cargo rullst ai connect",
                "connect the terminal assistant (keys stay outside the project)",
            ),
        ],
    },
    Capability {
        name: "nexus",
        summary: "the authenticated admin CMS at /nexus",
        feature: "nexus",
        implied_by: &[],
        env_title: "Nexus admin",
        env: &[
            entry(
                "NEXUS_ADMIN_USERNAME",
                "",
                &[
                    "Debug builds allow loopback access without credentials; release",
                    "builds require both (password of at least 16 characters).",
                ],
            ),
            entry("NEXUS_ADMIN_PASSWORD", "", &[]),
        ],
        snippet: Snippet {
            place: "src/main.rs, where the router is built",
            code: "let nexus = rullst::nexus::Nexus::new()\n    .with_auth_policy(rullst::nexus::NexusAuthPolicy::local_development_or_basic_from_env()?)\n    // .register::<models::post::Post>() for each model deriving `Nexus`\n    .try_build()?;\n// …then add to the router chain:\n.nest_axum(\"/nexus\", nexus)",
        },
        hints: &[(
            "cargo rullst dev",
            "then open /nexus (loopback only in debug builds)",
        )],
    },
    Capability {
        name: "studio",
        summary: "the local developer control room on port 5555",
        feature: "studio",
        implied_by: &[],
        env_title: "Studio",
        env: &[],
        snippet: Snippet {
            place: "src/main.rs, in `main` before the server starts",
            code: "#[cfg(debug_assertions)]\nrullst::runtime::spawn(async {\n    if let Err(error) = rullst::studio::run_studio(5555).await {\n        eprintln!(\"Rullst Studio could not start: {error}\");\n    }\n});",
        },
        hints: &[(
            "cargo rullst dev",
            "then open http://127.0.0.1:5555 in a debug build",
        )],
    },
];

/// The `<capability>` values, in the order `--help` lists them.
pub(crate) const NAMES: [&str; 5] = ["mail", "auth", "ai", "nexus", "studio"];

pub(crate) fn find(name: &str) -> Option<&'static Capability> {
    CAPABILITIES
        .iter()
        .find(|capability| capability.name == name)
}

impl Capability {
    /// Whether `features` of the `rullst` dependency already enable it.
    pub(crate) fn enabled_by<'a>(&self, mut features: impl Iterator<Item = &'a str>) -> bool {
        features.any(|feature| feature == self.feature || self.implied_by.contains(&feature))
    }
}
