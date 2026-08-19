#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// The top-level response sent to the browser to begin a WebAuthn credential registration ceremony.
/// Wraps [`PublicKeyCredentialCreationOptions`] under the `publicKey` JSON key as required by the W3C spec.
pub struct CreationChallengeResponse {
    #[serde(rename = "publicKey")]
    /// The full set of options passed to `navigator.credentials.create()` on the client.
    pub public_key: PublicKeyCredentialCreationOptions,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// Options passed to `navigator.credentials.create()` to register a new public-key credential.
/// Serialized under the `publicKey` field of the JSON challenge response.
pub struct PublicKeyCredentialCreationOptions {
    /// Base64url-encoded random challenge used to prevent replay attacks.
    pub challenge: String,
    /// Information about the Relying Party (name and ID).
    pub rp: RelyingPartyInfo,
    /// Information about the user account being registered.
    pub user: UserInfo,
    #[serde(rename = "pubKeyCredParams")]
    /// Ordered list of supported credential types and cryptographic algorithms.
    pub pub_key_cred_params: Vec<PubKeyCredParam>,
    /// Maximum time (in milliseconds) that the browser should wait for the user to respond.
    pub timeout: u32,
    #[serde(rename = "authenticatorSelection")]
    /// Constraints on the authenticator used for credential creation.
    pub authenticator_selection: AuthenticatorSelection,
    /// Attestation conveyance preference (`"none"`, `"indirect"`, or `"direct"`).
    pub attestation: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// Identifies the Relying Party to the authenticator.
pub struct RelyingPartyInfo {
    /// Human-readable Relying Party name shown to the user (e.g. `"My App"`).
    pub name: String,
    /// Effective domain that scopes the credential (e.g. `"example.com"`).
    pub id: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// Identifies the user account being registered to the authenticator.
pub struct UserInfo {
    /// Base64url-encoded unique user identifier (16-byte buffer).
    pub id: String,
    /// Machine-readable username (e.g. `"alice"`).
    pub name: String,
    /// Human-readable display name shown in credential pickers (e.g. `"Alice Smith"`).
    pub display_name: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// Specifies a cryptographic algorithm and credential type accepted by the Relying Party.
pub struct PubKeyCredParam {
    /// Credential type, always `"public-key"`.
    pub r#type: String,
    /// COSE algorithm identifier (e.g. `-7` for ES256 / ECDSA P-256 with SHA-256).
    pub alg: i32,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// Controls authenticator selection criteria during credential creation.
pub struct AuthenticatorSelection {
    /// User verification requirement (`"required"`, `"preferred"`, or `"discouraged"`).
    pub user_verification: String,
    /// Resident key requirement (`"required"`, `"preferred"`, or `"discouraged"`).
    pub resident_key: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// The top-level response sent to the browser to begin a WebAuthn assertion (login) ceremony.
/// Wraps [`PublicKeyCredentialRequestOptions`] under the `publicKey` JSON key as required by the W3C spec.
pub struct RequestChallengeResponse {
    #[serde(rename = "publicKey")]
    /// The options passed to `navigator.credentials.get()` on the client.
    pub public_key: PublicKeyCredentialRequestOptions,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// Options passed to `navigator.credentials.get()` to authenticate using an existing public-key credential.
pub struct PublicKeyCredentialRequestOptions {
    /// Base64url-encoded random challenge to prevent replay attacks.
    pub challenge: String,
    /// Maximum time (in milliseconds) that the browser should wait for the user to respond.
    pub timeout: u32,
    /// Relying Party identifier scoping the assertion (e.g. `"example.com"`).
    pub rp_id: String,
    /// List of credentials allowed to fulfill this request.
    pub allow_credentials: Vec<AllowCredential>,
    /// User verification requirement (`"required"`, `"preferred"`, or `"discouraged"`).
    pub user_verification: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// Identifies an existing credential allowed to satisfy an assertion request.
pub struct AllowCredential {
    /// Credential type, always `"public-key"`.
    pub r#type: String,
    /// Base64url-encoded credential ID previously registered for this account.
    pub id: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// The JSON payload returned by `navigator.credentials.create()` during registration.
pub struct RegisterPublicKeyCredential {
    /// Base64url-encoded credential ID.
    pub id: String,
    /// Base64url-encoded raw binary credential ID.
    pub raw_id: String,
    /// Credential type (`"public-key"`).
    pub r#type: String,
    /// Attestation response data returned by the authenticator.
    pub response: AuthenticatorAttestationResponse,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// Attestation details produced by an authenticator during credential creation.
pub struct AuthenticatorAttestationResponse {
    /// Base64url-encoded CBOR-encoded attestation object containing authData and public key.
    pub attestation_object: String,
    /// Base64url-encoded JSON client data containing the challenge, origin, and ceremony type.
    pub client_data_json: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// The JSON payload returned by `navigator.credentials.get()` during authentication.
pub struct PublicKeyCredential {
    /// Base64url-encoded credential ID.
    pub id: String,
    /// Base64url-encoded raw binary credential ID.
    pub raw_id: String,
    /// Credential type (`"public-key"`).
    pub r#type: String,
    /// Assertion response data returned by the authenticator.
    pub response: AuthenticatorAssertionResponse,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// Assertion details produced by an authenticator during authentication.
pub struct AuthenticatorAssertionResponse {
    /// Base64url-encoded authenticator data containing flags, sign count, and rpIdHash.
    pub authenticator_data: String,
    /// Base64url-encoded ECDSA signature over `authenticatorData || SHA-256(clientDataJSON)`.
    pub signature: String,
    /// Base64url-encoded JSON client data containing the challenge, origin, and ceremony type.
    pub client_data_json: String,
}

#[derive(Clone, Debug)]
/// Persisted in the database after successful WebAuthn registration.
pub struct Passkey {
    /// The credential ID returned by the authenticator, used to match credentials during authentication.
    pub credential_id: Vec<u8>,
    /// The raw DER-encoded COSE public key extracted from the attestation object.
    pub public_key: Vec<u8>,
    /// Monotonically increasing signature counter used to detect authenticator cloning.
    pub sign_count: u32,
}
