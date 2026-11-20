### Fixed

- `AiClient::auto()` and `AutoAiConfig` built a fallback chain in which a
  leftover `mock_*` credential could sit behind a live provider; the chain
  tries the next provider after any error, so in production every failed live
  request was answered with offline fixture text as a success. In staging and
  production (`RULLST_ENV`, then `APP_ENV`, from the process environment) such
  a mixed chain now fails with `AiError::ConfigError` naming the offline
  provider. A chain of mocks only, a chain of live providers only, and
  development and test are unchanged.
