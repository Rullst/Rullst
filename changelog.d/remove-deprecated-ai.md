### Removed

- **Breaking:** `rullst-ai`'s `AiClient::structured_prompt` and
  `GuardrailReport::is_safe`, deprecated in 12.0.0, were removed.
  `structured_prompt` took no schema and always returned
  `AiError::UnsupportedCapability`: call `json_prompt` for parseable JSON or
  `structured_prompt_with_schema` for output the provider checks against a
  JSON Schema. Replace `is_safe()` with `passed_heuristics()`, which returns
  the same value under a name that does not promise safety. `cargo rullst
  upgrade` reports both (`V13-AI-DEPRECATED-REMOVED`, a review finding, because
  `is_safe` is a common method name).
