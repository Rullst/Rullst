use super::{
    CURRENT_CLIENT_CONTRACT_VERSION, ClientContractError, ClientRequest, ClientVersionOffer,
    ContractVersion, DEFAULT_CLIENT_CONTRACT_BODY_BYTES, MAX_CLIENT_CONTRACT_BODY_BYTES,
    ServerFailure, ServerResponse,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::io::Write;

/// Server policy for version negotiation and bounded JSON encoding/decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ClientContractPolicy {
    minimum_version: ContractVersion,
    current_version: ContractVersion,
    max_body_bytes: usize,
}

impl ClientContractPolicy {
    /// Creates a policy with an inclusive supported version range and body limit.
    pub const fn new(
        minimum_version: ContractVersion,
        current_version: ContractVersion,
        max_body_bytes: usize,
    ) -> Result<Self, ClientContractError> {
        if minimum_version.get() > current_version.get()
            || max_body_bytes == 0
            || max_body_bytes > MAX_CLIENT_CONTRACT_BODY_BYTES
        {
            return Err(ClientContractError::InvalidPolicy);
        }
        Ok(Self {
            minimum_version,
            current_version,
            max_body_bytes,
        })
    }

    /// Returns the minimum accepted wire version.
    pub const fn minimum_version(self) -> ContractVersion {
        self.minimum_version
    }

    /// Returns the latest emitted wire version.
    pub const fn current_version(self) -> ContractVersion {
        self.current_version
    }

    /// Returns the configured encoded-body ceiling.
    pub const fn max_body_bytes(self) -> usize {
        self.max_body_bytes
    }

    /// Decodes a bounded client version offer.
    pub fn decode_offer(self, body: &[u8]) -> Result<ClientVersionOffer, ClientContractError> {
        self.validate_body_size(body.len())?;
        serde_json::from_slice(body).map_err(ClientContractError::InvalidJson)
    }

    /// Encodes a client version offer within the configured ceiling.
    pub fn encode_offer(self, offer: &ClientVersionOffer) -> Result<Vec<u8>, ClientContractError> {
        self.encode_bounded(offer)
    }

    /// Selects the highest mutually supported version.
    pub fn negotiate(
        self,
        offer: &ClientVersionOffer,
    ) -> Result<ContractVersion, ClientContractError> {
        offer
            .supported_versions()
            .iter()
            .rev()
            .copied()
            .find(|version| *version >= self.minimum_version && *version <= self.current_version)
            .ok_or(ClientContractError::NoMutualVersion)
    }

    /// Verifies that a version falls inside the configured inclusive range.
    pub const fn validate_version(
        self,
        version: ContractVersion,
    ) -> Result<(), ClientContractError> {
        if version.get() < self.minimum_version.get() || version.get() > self.current_version.get()
        {
            Err(ClientContractError::UnsupportedVersion {
                received: version.get(),
                minimum: self.minimum_version.get(),
                current: self.current_version.get(),
            })
        } else {
            Ok(())
        }
    }

    /// Decodes and validates a bounded client request.
    pub fn decode_request<T>(self, body: &[u8]) -> Result<ClientRequest<T>, ClientContractError>
    where
        T: DeserializeOwned,
    {
        self.validate_body_size(body.len())?;
        let request = serde_json::from_slice::<ClientRequest<T>>(body)
            .map_err(ClientContractError::InvalidJson)?;
        self.validate_version(request.version())?;
        Ok(request)
    }

    /// Encodes a validated client request within the configured ceiling.
    pub fn encode_request<T>(
        self,
        request: &ClientRequest<T>,
    ) -> Result<Vec<u8>, ClientContractError>
    where
        T: Serialize,
    {
        self.validate_version(request.version())?;
        self.encode_bounded(request)
    }

    /// Decodes and validates a bounded success response.
    pub fn decode_response<T>(self, body: &[u8]) -> Result<ServerResponse<T>, ClientContractError>
    where
        T: DeserializeOwned,
    {
        self.validate_body_size(body.len())?;
        let response = serde_json::from_slice::<ServerResponse<T>>(body)
            .map_err(ClientContractError::InvalidJson)?;
        self.validate_version(response.version())?;
        Ok(response)
    }

    /// Encodes a success response within the configured ceiling.
    pub fn encode_response<T>(
        self,
        response: &ServerResponse<T>,
    ) -> Result<Vec<u8>, ClientContractError>
    where
        T: Serialize,
    {
        self.validate_version(response.version())?;
        self.encode_bounded(response)
    }

    /// Decodes and validates a bounded failure response.
    pub fn decode_failure(self, body: &[u8]) -> Result<ServerFailure, ClientContractError> {
        self.validate_body_size(body.len())?;
        let response = serde_json::from_slice::<ServerFailure>(body)
            .map_err(ClientContractError::InvalidJson)?;
        self.validate_version(response.version())?;
        Ok(response)
    }

    /// Encodes a failure response within the configured ceiling.
    pub fn encode_failure(self, response: &ServerFailure) -> Result<Vec<u8>, ClientContractError> {
        self.validate_version(response.version())?;
        self.encode_bounded(response)
    }

    fn encode_bounded<T>(self, value: &T) -> Result<Vec<u8>, ClientContractError>
    where
        T: Serialize,
    {
        let mut writer = BoundedJsonWriter::new(self.max_body_bytes);
        match serde_json::to_writer(&mut writer, value) {
            Ok(()) => Ok(writer.into_bytes()),
            Err(_) if writer.exceeded() => Err(ClientContractError::BodyTooLarge {
                maximum: self.max_body_bytes,
            }),
            Err(error) => Err(ClientContractError::EncodeJson(error)),
        }
    }

    const fn validate_body_size(self, received: usize) -> Result<(), ClientContractError> {
        if received > self.max_body_bytes {
            Err(ClientContractError::BodyTooLarge {
                maximum: self.max_body_bytes,
            })
        } else {
            Ok(())
        }
    }
}

struct BoundedJsonWriter {
    bytes: Vec<u8>,
    maximum: usize,
    exceeded: bool,
}

impl BoundedJsonWriter {
    fn new(maximum: usize) -> Self {
        Self {
            bytes: Vec::new(),
            maximum,
            exceeded: false,
        }
    }

    fn exceeded(&self) -> bool {
        self.exceeded
    }

    fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

impl Write for BoundedJsonWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let remaining = self.maximum.saturating_sub(self.bytes.len());
        if buffer.len() > remaining {
            self.exceeded = true;
            return Err(std::io::Error::other("client contract body limit reached"));
        }
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Default for ClientContractPolicy {
    fn default() -> Self {
        Self {
            minimum_version: CURRENT_CLIENT_CONTRACT_VERSION,
            current_version: CURRENT_CLIENT_CONTRACT_VERSION,
            max_body_bytes: DEFAULT_CLIENT_CONTRACT_BODY_BYTES,
        }
    }
}
