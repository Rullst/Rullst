/// Configuration for the WebAuthn/Passkey authentication manager.
/// Adheres to backward-compatibility guidelines via `#[non_exhaustive]`
/// and the fluent builder pattern.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct PasskeyConfig {
    /// Human-readable name of the Relying Party displayed to the user during registration (e.g. `"My App"`).
    pub rp_name: String,
    /// The effective domain of the Relying Party used to scope the credential (e.g. `"example.com"`).
    /// Must match the origin's registrable domain suffix.
    pub rp_id: String,
    /// The full origin URL of the Relying Party (e.g. `"https://example.com"`).
    /// Used to verify the `clientDataJSON.origin` during assertion.
    pub rp_origin: String,
    /// Require the authenticator to prove user verification (PIN, biometric, or equivalent).
    pub require_user_verification: bool,
    /// Maximum lifetime of a one-time registration/authentication challenge, in seconds.
    pub challenge_ttl_seconds: u64,
    /// Upper bound for in-memory outstanding challenges per `PasskeyAuth` instance.
    pub max_pending_challenges: usize,
}

impl PasskeyConfig {
    /// Creates a new `PasskeyConfig` with mandatory fields.
    pub fn new(
        rp_name: impl Into<String>,
        rp_id: impl Into<String>,
        rp_origin: impl Into<String>,
    ) -> Self {
        Self {
            rp_name: rp_name.into(),
            rp_id: rp_id.into(),
            rp_origin: rp_origin.into(),
            require_user_verification: true,
            challenge_ttl_seconds: 300,
            max_pending_challenges: 10_000,
        }
    }

    /// Builder helper to set or override the Relying Party name.
    pub fn with_rp_name(mut self, rp_name: impl Into<String>) -> Self {
        self.rp_name = rp_name.into();
        self
    }

    /// Builder helper to set or override the Relying Party ID.
    pub fn with_rp_id(mut self, rp_id: impl Into<String>) -> Self {
        self.rp_id = rp_id.into();
        self
    }

    /// Builder helper to set or override the Relying Party Origin.
    pub fn with_rp_origin(mut self, rp_origin: impl Into<String>) -> Self {
        self.rp_origin = rp_origin.into();
        self
    }

    /// Configures whether user verification is mandatory. User presence remains mandatory.
    pub fn require_user_verification(mut self, required: bool) -> Self {
        self.require_user_verification = required;
        self
    }

    /// Configures the lifetime of one-time challenges.
    pub fn with_challenge_ttl_seconds(mut self, seconds: u64) -> Self {
        self.challenge_ttl_seconds = seconds;
        self
    }

    /// Configures the maximum number of outstanding challenges retained in memory.
    pub fn with_max_pending_challenges(mut self, maximum: usize) -> Self {
        self.max_pending_challenges = maximum;
        self
    }
}
