### Changed

- In staging and production, `AiClient::auto()` no longer answers with the
  offline OpenAI fixture when no AI provider is configured. With no
  `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GEMINI_API_KEY`, `DEEPSEEK_API_KEY`,
  `GROQ_API_KEY` or `OLLAMA_HOST` (empty values count as unset), it returned
  fixture text for every prompt as if a model had replied. It now returns
  `AiError::ConfigError` naming those variables, so a misconfigured staging or
  production application fails at start-up, or wherever it builds the client.
  A client built with `AutoAiConfig::into_client()` from such a snapshot has no
  provider, so each request fails with `AiError::ConfigError`;
  `AutoAiConfig::from_env()` still succeeds with no provider names, which Nexus
  reports as no provider. Provider variables are read from the process
  environment; the environment is resolved as `Server` resolves it, from the
  working directory: the process `RULLST_ENV`, then `APP_ENV`, then either one
  from `.env`, then `[app] env` in `Rullst.toml`. An invalid value, or a `.env`
  or `Rullst.toml` that cannot be read, fails closed. Set a provider, or an
  explicit `mock_*` value to keep the offline fixture on purpose. Development
  and test keep the fallback.
