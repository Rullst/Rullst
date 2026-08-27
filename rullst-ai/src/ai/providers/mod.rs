/// Anthropic AI client provider.
pub mod anthropic;
/// DeepSeek OpenAI-compatible client provider.
pub mod deepseek;
/// Google Gemini client provider.
pub mod gemini;
/// Ollama local client provider.
pub mod ollama;
/// OpenAI client provider.
pub mod openai;

mod support;

#[cfg(test)]
mod capability_tests {
    use super::{
        anthropic::AnthropicProvider, deepseek::DeepSeekProvider, gemini::GeminiProvider,
        ollama::OllamaProvider, openai::OpenAiProvider,
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
        request_timeout: false,
        retries: false,
        explicit_cancellation: false,
    };

    #[test]
    fn built_in_capability_contract_matches_the_implemented_transports() {
        assert_eq!(OpenAiProvider::new("").capabilities(), FULL_GENERATION);
        assert_eq!(GeminiProvider::new("").capabilities(), FULL_GENERATION);
        assert_eq!(
            OllamaProvider::new("", "mock-model").capabilities(),
            FULL_GENERATION
        );

        assert_eq!(
            AnthropicProvider::new("").capabilities(),
            ProviderCapabilities {
                embeddings: false,
                json: JsonCapability::PromptOnly,
                json_schema: false,
                ..FULL_GENERATION
            }
        );
        assert_eq!(
            DeepSeekProvider::new("").capabilities(),
            ProviderCapabilities {
                embeddings: false,
                vision: false,
                ..FULL_GENERATION
            }
        );
        assert!(
            !DeepSeekProvider::new("")
                .with_model("deepseek-chat")
                .capabilities()
                .json_schema
        );
    }
}
