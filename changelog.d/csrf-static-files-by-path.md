### Changed

- **Breaking:** the CSRF middleware decides which `GET` and `HEAD` requests
  are static files, which never mint a token or set the `rullst_csrf`
  cookie, by path instead of by extension. A static file is a path below the
  new `[security]` `csrf_static_prefixes` (default `/static/` and
  `/.well-known/`) or one of `csrf_static_paths` (default `/favicon.ico`,
  `/robots.txt`, `/sitemap.xml`, `/manifest.webmanifest` and `/sw.js`, the
  files generated projects serve). Until now any path ending in `.json`,
  `.xml`, `.txt`, `.js`, `.css`, an image or `.wasm` extension counted, so a
  dynamic page such as `/report.json` had no `CsrfToken` on a first visit;
  it now gets one. Files served at other paths (an application's own
  `nest_service("/assets", ServeDir::new(..))`) now set the cookie for a
  visitor who has none: list their prefix in `Rullst.toml`, or call the new
  `SecurityConfig::with_csrf_static_prefix` and `with_csrf_static_path`
  builders; `SecurityConfig::is_csrf_static_path` reports the decision, and
  `validate` rejects a prefix that does not start and end with `/`.
  Verification of state-changing requests is unchanged.
  `cargo rullst upgrade` reports such file services
  (`V13-CSRF-STATIC-PREFIXES`), and `deploy:doctor` no longer reports the two
  settings as unknown.
