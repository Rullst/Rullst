### Changed

- The `Caddyfile` that `cargo rullst deploy` writes compresses pages and
  other application responses (`encode zstd gzip`) now that the CSRF token
  in pages is masked per response. Until now pages were sent uncompressed
  because of BREACH. Static files keep their pre-compressed `.br`/`.zst`
  siblings, which Caddy passes through unchanged. An existing `Caddyfile`
  is not changed; `cargo rullst upgrade` reports one without `encode`
  (`V13-DEPLOY-CADDY-ENCODE`).
