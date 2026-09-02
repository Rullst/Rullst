use super::ClientContractError;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

const MAX_OFFERED_VERSIONS: usize = 16;
const MAX_REQUEST_ID_BYTES: usize = 128;
const MIN_IDEMPOTENCY_KEY_BYTES: usize = 8;
const MAX_IDEMPOTENCY_KEY_BYTES: usize = 128;
const MAX_FAILURE_CODE_BYTES: usize = 64;

/// Positive client-contract wire version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
#[non_exhaustive]
pub struct ContractVersion(pub(super) u16);

impl ContractVersion {
    /// Creates a positive wire version.
    pub const fn new(value: u16) -> Result<Self, ClientContractError> {
        if value == 0 {
            Err(ClientContractError::InvalidVersion)
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the numeric wire version.
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl<'de> Deserialize<'de> for ContractVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u16::deserialize(deserializer)?;
        Self::new(value).map_err(D::Error::custom)
    }
}

/// Log-safe request correlation identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
#[non_exhaustive]
pub struct RequestId(String);

impl RequestId {
    /// Validates a 1–128 byte ASCII correlation identifier.
    pub fn new(value: impl Into<String>) -> Result<Self, ClientContractError> {
        let value = value.into();
        if valid_token(&value, 1, MAX_REQUEST_ID_BYTES) {
            Ok(Self(value))
        } else {
            Err(ClientContractError::InvalidRequestId)
        }
    }

    /// Returns the validated identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Constructs a framework-generated identifier. Callers outside Core must
    /// continue through [`RequestId::new`] so untrusted values are validated.
    pub(crate) fn framework(value: String) -> Self {
        Self(value)
    }
}

impl<'de> Deserialize<'de> for RequestId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(D::Error::custom)
    }
}

/// Bounded mutation replay key; persistence and atomic replay handling remain server work.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
#[non_exhaustive]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    /// Validates an 8–128 byte ASCII mutation key.
    pub fn new(value: impl Into<String>) -> Result<Self, ClientContractError> {
        let value = value.into();
        if valid_token(&value, MIN_IDEMPOTENCY_KEY_BYTES, MAX_IDEMPOTENCY_KEY_BYTES) {
            Ok(Self(value))
        } else {
            Err(ClientContractError::InvalidIdempotencyKey)
        }
    }

    /// Returns the validated key.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for IdempotencyKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(D::Error::custom)
    }
}

/// Stable, application-defined machine-readable failure code.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
#[non_exhaustive]
pub struct FailureCode(String);

impl FailureCode {
    /// Creates a lowercase dotted code such as `lesson.answer_invalid`.
    pub fn new(value: impl Into<String>) -> Result<Self, ClientContractError> {
        let value = value.into();
        let segments = value.split('.').collect::<Vec<_>>();
        let valid = value.len() <= MAX_FAILURE_CODE_BYTES
            && !value.is_empty()
            && segments.iter().all(|segment| {
                !segment.is_empty()
                    && segment
                        .chars()
                        .next()
                        .is_some_and(|first| first.is_ascii_lowercase())
                    && segment.chars().all(|character| {
                        character.is_ascii_lowercase()
                            || character.is_ascii_digit()
                            || character == '_'
                    })
            });
        if valid {
            Ok(Self(value))
        } else {
            Err(ClientContractError::InvalidFailureCode)
        }
    }

    /// Returns the validated code.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Constructs one of the framework-owned codes whose literal is reviewed
    /// with the protocol implementation. Application input must always use
    /// [`FailureCode::new`] instead.
    pub(crate) fn framework(value: &'static str) -> Self {
        Self(value.to_owned())
    }
}

impl<'de> Deserialize<'de> for FailureCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum ProtocolMarker {
    #[serde(rename = "rullst.client")]
    RullstClient,
}

/// Bounded list of protocol versions offered by a client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct ClientVersionOffer {
    contract: ProtocolMarker,
    supported_versions: Vec<ContractVersion>,
}

impl ClientVersionOffer {
    /// Creates a canonical ascending version offer with at most 16 entries.
    pub fn new<I>(versions: I) -> Result<Self, ClientContractError>
    where
        I: IntoIterator<Item = ContractVersion>,
    {
        let mut versions = versions.into_iter().collect::<Vec<_>>();
        if versions.is_empty() {
            return Err(ClientContractError::EmptyVersionOffer);
        }
        if versions.len() > MAX_OFFERED_VERSIONS {
            return Err(ClientContractError::TooManyOfferedVersions);
        }
        versions.sort_unstable();
        versions.dedup();
        Ok(Self {
            contract: ProtocolMarker::RullstClient,
            supported_versions: versions,
        })
    }

    /// Returns the canonical ascending supported versions.
    pub fn supported_versions(&self) -> &[ContractVersion] {
        &self.supported_versions
    }
}

impl<'de> Deserialize<'de> for ClientVersionOffer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct WireOffer {
            #[serde(rename = "contract")]
            _contract: ProtocolMarker,
            supported_versions: Vec<ContractVersion>,
        }

        let WireOffer {
            _contract: _,
            supported_versions,
        } = WireOffer::deserialize(deserializer)?;
        Self::new(supported_versions).map_err(D::Error::custom)
    }
}

/// Typed request envelope shared by browser, desktop and mobile clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct ClientRequest<T> {
    contract: ProtocolMarker,
    version: ContractVersion,
    request_id: RequestId,
    #[serde(skip_serializing_if = "Option::is_none")]
    idempotency_key: Option<IdempotencyKey>,
    payload: T,
}

impl<T> ClientRequest<T> {
    /// Creates a read/non-replay-sensitive request envelope.
    pub fn new(version: ContractVersion, request_id: RequestId, payload: T) -> Self {
        Self {
            contract: ProtocolMarker::RullstClient,
            version,
            request_id,
            idempotency_key: None,
            payload,
        }
    }

    /// Creates a mutation envelope carrying a validated replay key.
    pub fn mutation(
        version: ContractVersion,
        request_id: RequestId,
        idempotency_key: IdempotencyKey,
        payload: T,
    ) -> Self {
        Self {
            contract: ProtocolMarker::RullstClient,
            version,
            request_id,
            idempotency_key: Some(idempotency_key),
            payload,
        }
    }

    /// Returns the requested wire version.
    pub const fn version(&self) -> ContractVersion {
        self.version
    }

    /// Returns the correlation identifier; it is never authorization evidence.
    pub fn request_id(&self) -> &RequestId {
        &self.request_id
    }

    /// Returns the optional mutation replay key.
    pub fn idempotency_key(&self) -> Option<&IdempotencyKey> {
        self.idempotency_key.as_ref()
    }

    /// Requires a replay key before a server executes a mutation.
    pub fn require_idempotency_key(&self) -> Result<&IdempotencyKey, ClientContractError> {
        self.idempotency_key
            .as_ref()
            .ok_or(ClientContractError::MissingIdempotencyKey)
    }

    /// Returns a reference to the application payload.
    pub fn payload(&self) -> &T {
        &self.payload
    }

    /// Consumes the envelope and returns the application payload.
    pub fn into_payload(self) -> T {
        self.payload
    }
}

/// Successful response envelope carrying server time and typed application data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct ServerResponse<T> {
    contract: ProtocolMarker,
    version: ContractVersion,
    request_id: RequestId,
    server_epoch_ms: u64,
    data: T,
}

impl<T> ServerResponse<T> {
    /// Creates a server-authored success envelope.
    pub fn new(
        version: ContractVersion,
        request_id: RequestId,
        server_epoch_ms: u64,
        data: T,
    ) -> Self {
        Self {
            contract: ProtocolMarker::RullstClient,
            version,
            request_id,
            server_epoch_ms,
            data,
        }
    }

    /// Returns the response wire version.
    pub const fn version(&self) -> ContractVersion {
        self.version
    }

    /// Returns the request correlation identifier.
    pub fn request_id(&self) -> &RequestId {
        &self.request_id
    }

    /// Returns the server-authored Unix epoch timestamp in milliseconds.
    pub const fn server_epoch_ms(&self) -> u64 {
        self.server_epoch_ms
    }

    /// Returns the typed response data.
    pub fn data(&self) -> &T {
        &self.data
    }

    /// Consumes the envelope and returns its data.
    pub fn into_data(self) -> T {
        self.data
    }
}

/// Safe machine-readable error detail without arbitrary provider/debug text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct FailureDetail {
    code: FailureCode,
    retryable: bool,
}

impl FailureDetail {
    /// Creates a bounded failure detail.
    pub const fn new(code: FailureCode, retryable: bool) -> Self {
        Self { code, retryable }
    }

    /// Returns the application-defined stable code.
    pub fn code(&self) -> &FailureCode {
        &self.code
    }

    /// Reports whether retry may be appropriate; it never authorizes a retry effect.
    pub const fn retryable(&self) -> bool {
        self.retryable
    }
}

/// Failure response envelope carrying only a bounded code and retry hint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct ServerFailure {
    contract: ProtocolMarker,
    version: ContractVersion,
    request_id: RequestId,
    server_epoch_ms: u64,
    error: FailureDetail,
}

impl ServerFailure {
    /// Creates a server-authored failure envelope.
    pub const fn new(
        version: ContractVersion,
        request_id: RequestId,
        server_epoch_ms: u64,
        error: FailureDetail,
    ) -> Self {
        Self {
            contract: ProtocolMarker::RullstClient,
            version,
            request_id,
            server_epoch_ms,
            error,
        }
    }

    /// Returns the response wire version.
    pub const fn version(&self) -> ContractVersion {
        self.version
    }

    /// Returns the request correlation identifier.
    pub fn request_id(&self) -> &RequestId {
        &self.request_id
    }

    /// Returns the server-authored Unix epoch timestamp in milliseconds.
    pub const fn server_epoch_ms(&self) -> u64 {
        self.server_epoch_ms
    }

    /// Returns the bounded error detail.
    pub const fn error(&self) -> &FailureDetail {
        &self.error
    }
}

fn valid_token(value: &str, minimum: usize, maximum: usize) -> bool {
    (minimum..=maximum).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}
