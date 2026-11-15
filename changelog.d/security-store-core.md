### Changed

- Studio and Nexus no longer depend on `rullst-security`. `SecurityStore` and
  `LiveSecurityEvent`, with `BannedIpRecord`, `TelemetrySnapshot`,
  `normalize_ip` and `SECURITY_EVENT_SCHEMA_VERSION`, are now defined in
  `rullst_core::security`, and `rullst-security` re-exports them at the same
  paths, so applications change nothing: the `rullst-security` layers and the
  Studio and Nexus security pages still share one `SecurityStore::global()`.
  Each generated starter's `Cargo.lock` lists 49 fewer packages (SaaS 398 to
  349, Blank and API 374 to 325), and a `rullst-studio` dependency with its
  default features builds 48 fewer (`jsonschema`, `ammonia` and the rest of
  `rullst-security`'s graph). The facade's `rullst::security` module also
  names these types without the `security` feature.
