//! The providers `make:oauth` can generate.

/// A provider `make:oauth` can generate.
pub(super) struct ProviderSpec {
    /// Route segment, cookie path, `oauth_identities.provider` and module name.
    pub(super) name: &'static str,
    /// Human-readable name in the login link and messages.
    pub(super) label: &'static str,
    /// Prefix of its settings, such as `GOOGLE` in `GOOGLE_CLIENT_ID`.
    pub(super) env: &'static str,
    /// Its settings after the prefix, with the value that selects the offline
    /// mock (or, for a non-secret setting, a working development value).
    pub(super) settings: &'static [(&'static str, &'static str)],
    /// `.env.example` comment lines describing the settings.
    pub(super) env_notes: &'static str,
    /// The provider POSTs the callback from its own site
    /// (`response_mode=form_post`), so the callback route also accepts POST
    /// and is exempt from the CSRF token.
    pub(super) form_post: bool,
    /// What the developer must also know, printed after the install.
    pub(super) note: Option<&'static str>,
    /// The generated `src/controllers/oauth/<name>.rs`.
    pub(super) template: &'static str,
}

impl ProviderSpec {
    /// The setting names, such as `GOOGLE_CLIENT_ID`.
    pub(super) fn setting_names(&self) -> Vec<String> {
        self.settings
            .iter()
            .map(|(setting, _)| format!("{}_{setting}", self.env))
            .collect()
    }

    /// The settings that select the offline provider, as `NAME=value` pairs.
    pub(super) fn offline_settings(&self) -> String {
        self.settings
            .iter()
            .map(|(setting, value)| format!("{}_{setting}={value}", self.env))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

pub(super) static PROVIDERS: [ProviderSpec; 4] = [
    ProviderSpec {
        name: "google",
        label: "Google",
        env: "GOOGLE",
        settings: &[
            ("CLIENT_ID", "mock_google"),
            ("CLIENT_SECRET", "mock_google"),
        ],
        env_notes: "# Leave both empty to turn it off. `mock_*` values select the offline\n# provider and need `--features oauth-mock` (development and tests only).\n",
        form_post: false,
        note: None,
        template: include_str!("google.rs.template"),
    },
    ProviderSpec {
        name: "github",
        label: "GitHub",
        env: "GITHUB",
        settings: &[
            ("CLIENT_ID", "mock_github"),
            ("CLIENT_SECRET", "mock_github"),
        ],
        env_notes: "# A GitHub OAuth App. Leave both empty to turn it off. `mock_*` values\n# select the offline provider and need `--features oauth-mock`\n# (development and tests only).\n",
        form_post: false,
        note: Some(
            "GitHub identities are keyed by the numeric account id, and only the primary email GitHub marks verified creates or links an account.",
        ),
        template: include_str!("github.rs.template"),
    },
    ProviderSpec {
        name: "microsoft",
        label: "Microsoft",
        env: "MICROSOFT",
        settings: &[
            ("TENANT", "common"),
            ("CLIENT_ID", "mock_microsoft"),
            ("CLIENT_SECRET", "mock_microsoft"),
        ],
        env_notes: "# MICROSOFT_TENANT: common, organizations, consumers or your directory\n# (tenant) ID; required with the credentials. Leave the credentials empty\n# to turn it off. `mock_*` values select the offline provider and need\n# `--features oauth-mock` (development and tests only).\n",
        form_post: false,
        note: Some(
            "Microsoft does not verify the email claim: add the optional ID-token claims email and xms_edov to the app registration (Token configuration), or a Microsoft identity cannot create or link an account. See src/controllers/oauth/microsoft.rs.",
        ),
        template: include_str!("microsoft.rs.template"),
    },
    ProviderSpec {
        name: "apple",
        label: "Apple",
        env: "APPLE",
        settings: &[
            ("CLIENT_ID", "mock_apple"),
            ("TEAM_ID", "mock_apple"),
            ("KEY_ID", "mock_apple"),
            ("PRIVATE_KEY", "mock_apple"),
        ],
        env_notes: "# The Services ID, team ID, key ID and the .p8 key's PEM text (newlines\n# as \\n). Never commit the key. Leave all four empty to turn it off.\n# `mock_*` values select the offline provider and need\n# `--features oauth-mock` (development and tests only).\n",
        form_post: true,
        note: Some(
            "Apple POSTs the callback from its own site: /auth/apple/callback is now listed in csrf_signed_webhook_paths of Rullst.toml, and the handler requires the login challenge's state instead of the CSRF token. Apple may hand out private relay emails; register the MAIL_FROM domain with Apple's Private Email Relay service.",
        ),
        template: include_str!("apple.rs.template"),
    },
];

/// The provider called `name`.
pub(super) fn find(name: &str) -> Option<&'static ProviderSpec> {
    PROVIDERS.iter().find(|provider| provider.name == name)
}

/// The names `make:oauth` accepts, in order.
pub(super) fn names() -> [&'static str; 4] {
    PROVIDERS.each_ref().map(|provider| provider.name)
}
