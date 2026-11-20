### Fixed

- `OpenAiCompatibleProvider` (and its DeepSeek and Ollama presets) had no
  connection deadline, although the README promised the five seconds the
  native adapters use: a host that never completed the connection held the
  request until its whole-request deadline. It now uses the native adapters'
  pooled HTTP client, with the five-second connect deadline, no redirects and
  no ambient proxy; `with_request_timeout` still sets the whole-request
  deadline.
