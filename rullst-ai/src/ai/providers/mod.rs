/// Anthropic AI client provider.
pub mod anthropic;
/// Google Gemini client provider.
pub mod gemini;
/// OpenAI client provider.
pub mod openai;
/// Capability-declared OpenAI-compatible local/cloud provider, with the
/// DeepSeek and Ollama presets.
pub mod openai_compatible;

mod http_error;
#[cfg(test)]
mod http_error_tests;
mod sse;
#[cfg(test)]
mod stream_tests;
mod support;

#[cfg(test)]
mod transport_tests;

#[cfg(test)]
mod capability_tests {
    use super::{
        anthropic::AnthropicProvider,
        gemini::GeminiProvider,
        openai::OpenAiProvider,
        openai_compatible::{OpenAiCompatibleCapabilities, OpenAiCompatibleProvider},
    };
    use crate::ai::{AiProvider, JsonCapability, ProviderCapabilities};

    const FULL_GENERATION: ProviderCapabilities = ProviderCapabilities {
        text: true,
        chat: true,
        embeddings: true,
        vision: true,
        json: JsonCapability::NativeMode,
        json_schema: true,
        streaming: false,
        tools: false,
        request_timeout: true,
        retries: false,
        explicit_cancellation: false,
    };

    #[test]
    fn built_in_capability_contract_matches_the_implemented_transports() {
        assert_eq!(OpenAiProvider::new("").capabilities(), FULL_GENERATION);
        // Native SSE streaming with explicit cancellation (v13).
        assert_eq!(
            GeminiProvider::new("").capabilities(),
            ProviderCapabilities {
                streaming: true,
                explicit_cancellation: true,
                ..FULL_GENERATION
            }
        );
        // The presets stream with usage and an explicit cancellation.
        assert_eq!(
            OpenAiCompatibleProvider::ollama("", "mock-model")
                .expect("offline Ollama preset")
                .capabilities(),
            ProviderCapabilities {
                streaming: true,
                explicit_cancellation: true,
                ..FULL_GENERATION
            }
        );
        assert_eq!(
            OpenAiCompatibleProvider::mock("mock-model")
                .expect("valid compatible fixture")
                .with_capabilities(
                    OpenAiCompatibleCapabilities::chat_only()
                        .with_embeddings()
                        .with_vision()
                        .with_json_schema(),
                )
                .capabilities(),
            FULL_GENERATION
        );

        assert_eq!(
            AnthropicProvider::new("").capabilities(),
            ProviderCapabilities {
                embeddings: false,
                json: JsonCapability::PromptOnly,
                json_schema: false,
                streaming: true,
                explicit_cancellation: true,
                ..FULL_GENERATION
            }
        );
        let deepseek = OpenAiCompatibleProvider::deepseek("").expect("offline DeepSeek preset");
        assert_eq!(
            deepseek.capabilities(),
            ProviderCapabilities {
                embeddings: false,
                vision: false,
                streaming: true,
                explicit_cancellation: true,
                ..FULL_GENERATION
            }
        );
        assert!(
            !deepseek
                .try_with_model("deepseek-chat")
                .expect("valid model")
                .capabilities()
                .json_schema
        );
    }
}
