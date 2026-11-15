### Removed

- **Breaking:** the infallible `new` constructors of the `rullst-connect`
  providers, deprecated in 12.0.0, were removed: `AppleProvider`,
  `Auth0Provider`, `CognitoProvider`, `DiscordProvider`, `FacebookProvider`,
  `GithubProvider`, `GoogleProvider`, `LinkedinProvider`, `MicrosoftProvider`
  and `XProvider`, and the `new` that `define_provider!` generated for a
  provider of the application's own. On invalid configuration they returned
  a disabled provider whose redirect was `about:blank` and whose requests
  failed. Call `try_new` with the same arguments and handle its
  `ConnectError::InvalidConfiguration`. `CredentialMode::Invalid` and
  `CredentialMode::is_invalid`, which only such a provider reported, were
  removed with them; `CredentialMode` is `#[non_exhaustive]`, so a `match`
  with a wildcard arm keeps compiling. `cargo rullst upgrade` reports the
  built-in providers' `new` and `CredentialMode::Invalid`
  (`V13-CONNECT-INFALLIBLE-NEW-REMOVED`); the compiler names the others.
