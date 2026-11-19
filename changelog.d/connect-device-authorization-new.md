### Added

- Connect's `DeviceAuthorizationResponse::new(device_code, user_code,
  verification_uri, expires_in)`, so a custom `Provider::request_device_code`
  can build the response that became `#[non_exhaustive]` in 13.0.0-alpha.6
  without going through serde; `verification_uri_complete` and `interval` are
  public fields set after it. `ExchangeParams` documents that an optional
  nonce is assigned to its public `expected_nonce` field.
