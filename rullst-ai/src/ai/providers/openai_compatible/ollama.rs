//! An Ollama server as a preset of the OpenAI-compatible adapter. Ollama
//! serves the OpenAI shapes under `/v1`
//! (<https://docs.ollama.com/api/openai-compatibility>): chat with streaming
//! and usage, JSON mode, JSON Schema output (`response_format` `json_schema`),
//! base64 image input and `/v1/embeddings`.

use super::{
    CompatibleAuthentication, EndpointScope, OFFLINE_BASE_URL, OpenAiCompatibleCapabilities,
    OpenAiCompatibleProvider, Preset,
};
use crate::ai::{AiError, mock::ProviderMode};

/// Port Ollama listens on when a scheme-less host names none.
const DEFAULT_PORT: u16 = 11434;
const DEFAULT_EMBEDDING_MODEL: &str = "nomic-embed-text";

impl OpenAiCompatibleProvider {
    /// An Ollama server at `host` with the generation model `model` and the
    /// embedding model `nomic-embed-text` (choose another with
    /// [`Self::try_with_embedding_model`]). No credential is sent.
    ///
    /// The host is read the way Ollama reads `OLLAMA_HOST`: a URL such as
    /// `http://127.0.0.1:11434` is used as written, while a scheme-less
    /// `127.0.0.1:11434`, `localhost` or `[::1]` uses `http` and port 11434
    /// unless it names a valid port; requests go to its `/v1` API. Because
    /// no credential is sent, plain HTTP is accepted for any host, such as a
    /// GPU server on the local network. An empty or `mock_*` host selects
    /// deterministic offline mode.
    ///
    /// It declares chat, streaming with usage, JSON mode, JSON Schema output,
    /// image input (JPEG, PNG or WebP) and embeddings; whether the model
    /// supports each one is up to the model.
    pub fn ollama(host: impl Into<String>, model: impl Into<String>) -> Result<Self, AiError> {
        let host = host.into();
        let mode = ProviderMode::from_credential(&host);
        let base_url = if mode.is_mock() {
            OFFLINE_BASE_URL.to_string()
        } else {
            format!("{}/v1", normalize_host(&host))
        };
        let mut provider = Self::try_build(
            base_url,
            model.into(),
            CompatibleAuthentication::None,
            mode,
            EndpointScope::AnyHost,
        )?
        .with_capabilities(
            OpenAiCompatibleCapabilities::chat_only()
                .with_embeddings()
                .with_vision()
                .with_json_schema()
                .with_stream_usage(),
        );
        DEFAULT_EMBEDDING_MODEL.clone_into(&mut provider.embedding_model);
        provider.preset = Preset::Ollama;
        Ok(provider)
    }
}

/// Mirrors Ollama's `OLLAMA_HOST` parsing: surrounding whitespace and quotes
/// are ignored, a value with `scheme://` keeps its scheme and port, and a value
/// without one uses `http` and [`DEFAULT_PORT`] (`ollama.com` alone means
/// `https://ollama.com:443`). An optional path is kept as a prefix.
pub(super) fn normalize_host(host: &str) -> String {
    let host = host.trim().trim_matches(['"', '\'']);
    if host.contains("://") {
        return host.trim_end_matches('/').to_string();
    }
    if host == "ollama.com" {
        return "https://ollama.com:443".to_string();
    }
    let (authority, path) = host.split_once('/').unwrap_or((host, ""));
    let authority = with_port(authority);
    match path.trim_end_matches('/') {
        "" => format!("http://{authority}"),
        path => format!("http://{authority}/{path}"),
    }
}

/// `host:port`, using [`DEFAULT_PORT`] when the port is absent or invalid and
/// `127.0.0.1` for an empty host. An unbracketed IPv6 address is bracketed.
fn with_port(authority: &str) -> String {
    let (name, port) = match authority
        .strip_prefix('[')
        .and_then(|rest| rest.split_once(']'))
    {
        Some((address, rest)) => (format!("[{address}]"), rest.strip_prefix(':')),
        None if authority.matches(':').count() > 1 => (format!("[{authority}]"), None),
        None => match authority.split_once(':') {
            Some((name, port)) => (name.to_string(), Some(port)),
            None => (authority.to_string(), None),
        },
    };
    let name = if name.is_empty() {
        "127.0.0.1".to_string()
    } else {
        name
    };
    let port = port
        .and_then(|port| port.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT);
    format!("{name}:{port}")
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn reads_ollama_host_values_the_way_ollama_does() {
        for (input, expected) in [
            ("127.0.0.1:11434", "http://127.0.0.1:11434"),
            ("0.0.0.0:11434", "http://0.0.0.0:11434"),
            ("localhost", "http://localhost:11434"),
            ("localhost:8080/", "http://localhost:8080"),
            (
                "gpu-box.internal:11434/ollama",
                "http://gpu-box.internal:11434/ollama",
            ),
            ("[::1]:11434", "http://[::1]:11434"),
            ("[::1]", "http://[::1]:11434"),
            ("::1", "http://[::1]:11434"),
            (":11434", "http://127.0.0.1:11434"),
            ("localhost:not-a-port", "http://localhost:11434"),
            ("localhost:70000", "http://localhost:11434"),
            ("\"127.0.0.1:11434\"", "http://127.0.0.1:11434"),
            ("ollama.com", "https://ollama.com:443"),
            (" https://ollama.example/ ", "https://ollama.example"),
            ("http://localhost:11434/", "http://localhost:11434"),
        ] {
            assert_eq!(normalize_host(input), expected, "input: {input:?}");
        }
    }

    #[test]
    fn the_preset_targets_the_v1_api_of_the_normalized_host() {
        for (host, base_url) in [
            ("127.0.0.1:11434", "http://127.0.0.1:11434/v1"),
            (
                "gpu-box.internal/ollama/",
                "http://gpu-box.internal:11434/ollama/v1",
            ),
            ("https://ollama.example", "https://ollama.example/v1"),
            ("mock_offline", OFFLINE_BASE_URL),
            ("", OFFLINE_BASE_URL),
        ] {
            let provider = OpenAiCompatibleProvider::ollama(host, "llama3").expect(host);
            assert_eq!(provider.base_url, base_url, "{host:?}");
            assert_eq!(provider.embedding_model, DEFAULT_EMBEDDING_MODEL);
            assert_eq!(provider.model, "llama3");
        }
        for host in ["ftp://127.0.0.1:21", "http://user:secret@127.0.0.1:11434"] {
            assert!(
                OpenAiCompatibleProvider::ollama(host, "llama3").is_err(),
                "{host}"
            );
        }
    }
}
