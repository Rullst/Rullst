### Changed

- **Breaking:** `rullst-media`'s remaining public enums and all-public
  structs are decided for the 13.0 freeze. `PlaybackKind`, `ProviderMode`
  and `sqlite::Lifecycle` are `#[non_exhaustive]`, so a `match` on them
  outside the crate needs a wildcard arm, and so are `ProviderBinding`,
  `RemoteVideo`, `WebhookHeaders` and `sqlite::Asset`, which code outside the
  crate no longer builds with a literal. New constructors:
  `ProviderBinding::new(library, mode, environment)` and
  `RemoteVideo::new(id, library, title, description, processing,
  length_seconds, mp4_720p)` for an application-owned `VideoProvider`, and
  `WebhookHeaders::new(version, algorithm, signature)` for the route that
  passes Bunny's signature headers to `verify_notification`. `Asset` comes
  from the store and is only read. `Action` and `Scope` stay exhaustive on
  purpose: the host's authorizer must map every action, and the scope is the
  authorization boundary. `cargo rullst upgrade` reports these matches and
  literals (`V13-NON-EXHAUSTIVE-MATCH`, `V13-NON-EXHAUSTIVE-LITERAL`), and the
  public API check no longer exempts any `rullst-media` type.
