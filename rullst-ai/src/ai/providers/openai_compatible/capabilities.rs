//! Request shapes one OpenAI-compatible endpoint/model pair declares.

use crate::ai::{JsonCapability, ProviderCapabilities};

/// Capabilities that one configured OpenAI-compatible endpoint/model pair
/// claims to implement.
///
/// The conservative default enables text/chat only. Applications must enable
/// optional request shapes only after verifying the exact server and model.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OpenAiCompatibleCapabilities {
    pub(super) embeddings: bool,
    pub(super) vision: bool,
    pub(super) json_mode: bool,
    pub(super) json_schema: bool,
    pub(super) streaming: bool,
    pub(super) stream_usage: bool,
}

impl OpenAiCompatibleCapabilities {
    /// Creates the conservative text/chat-only capability set.
    #[must_use]
    pub const fn chat_only() -> Self {
        Self {
            embeddings: false,
            vision: false,
            json_mode: false,
            json_schema: false,
            streaming: false,
            stream_usage: false,
        }
    }

    /// Declares that the configured endpoint/model accepts `/embeddings`.
    #[must_use]
    pub const fn with_embeddings(mut self) -> Self {
        self.embeddings = true;
        self
    }

    /// Declares OpenAI-shaped image input support for the generation model.
    #[must_use]
    pub const fn with_vision(mut self) -> Self {
        self.vision = true;
        self
    }

    /// Declares native `json_object` response-mode support.
    #[must_use]
    pub const fn with_json_mode(mut self) -> Self {
        self.json_mode = true;
        self
    }

    /// Declares native `json_schema` response-mode support.
    ///
    /// Schema support also enables the weaker native JSON mode.
    #[must_use]
    pub const fn with_json_schema(mut self) -> Self {
        self.json_mode = true;
        self.json_schema = true;
        self
    }

    /// Declares support for the OpenAI `text/event-stream` chat shape.
    #[must_use]
    pub const fn with_streaming(mut self) -> Self {
        self.streaming = true;
        self
    }

    /// Declares streaming plus `stream_options.include_usage`, which makes
    /// the stream end with a usage chunk (OpenAI and DeepSeek accept it).
    /// Without it, a usage object the server sends anyway is still read (v13).
    #[must_use]
    pub const fn with_stream_usage(mut self) -> Self {
        self.streaming = true;
        self.stream_usage = true;
        self
    }

    pub(super) fn provider_capabilities(self) -> ProviderCapabilities {
        ProviderCapabilities {
            text: true,
            chat: true,
            embeddings: self.embeddings,
            vision: self.vision,
            json: if self.json_mode {
                JsonCapability::NativeMode
            } else {
                JsonCapability::Unsupported
            },
            json_schema: self.json_schema,
            streaming: self.streaming,
            tools: false,
            request_timeout: true,
            retries: false,
            explicit_cancellation: self.streaming,
        }
    }
}
