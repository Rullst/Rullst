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
}
