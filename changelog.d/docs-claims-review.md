### Changed

- The opt-in features that earlier prereleases called candidates (email
  login, scoped API tokens, session management, S3/R2 storage and resumable
  uploads, outgoing webhooks, recurring publications, Redis Streams messaging
  and distributed tracing) have the tier of their package and its 13.x
  stability promise from 13.0.0; their documentation now says what each one
  is tested against instead of "final release admission remains separate".
- The documentation now states limits it left out: without `RULLST_ENV`
  (or `APP_ENV`, or `[app] env`) an application runs as development, where
  the WAF only logs and `Strict-Transport-Security` is sent only over HTTPS;
  a forwarder on the same machine that adds no forwarding header and speaks
  HTTP/1.1 defeats the loopback check of local Studio and of Nexus in a debug
  build, and anyone who can read the address the remote Studio prints can use
  it until it restarts; `html!` only
  HTML-escapes `<script>` and `<style>` bodies, `srcdoc` and htmx's
  `hx-vals`/`hx-headers`, so they take trusted values only; a signed-webhook
  handler that fails keeps the payload's replay claim, so the provider's
  retry gets `409` for the replay TTL; the Ollama preset accepts plain HTTP to
  any host; a client with a /50 or larger IPv6 allocation can fill the
  `rate_limit_middleware` table; and an
  empty `TURSO_DATABASE_URL` selects a local SQLite file in every
  environment. Claims that went beyond the evidence were corrected: CI denies
  `unwrap`/`expect`/`panic!`/`todo!`/`unimplemented!` rather than every
  panic, coverage is enforced on pushes to `main`, provider guides were not
  followed on real accounts, rustls offers `X25519MLKEM768` first only when
  `prefer-post-quantum` is enabled, and Google, GitHub, Resend, SMTP,
  Gemini and Bunny Stream are used by the maintainer's production
  applications without an automated live test.
