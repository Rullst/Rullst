### Changed

- Rullst no longer has an Experimental maturity tier: every package is Core
  or Extension, and no package, adapter, feature or generator is labelled
  experimental. The opt-in `s3` adapter of `rullst-media` (AWS S3,
  Cloudflare R2, MinIO) is now an Extension feature like the rest of the
  crate, covered by the 13.x stability policy from 13.0.0; it is still tested
  offline only, and no automated test runs against a live S3, R2 or MinIO
  service. `#[memoize]` also has its package's tier, and the
  `experimental-simulators` fixtures of `rullst-iot` were removed (see
  Removed).
  The documentation states what each part is tested against, what was
  validated in real use and what never ran against a live service
  ([maturity tiers](https://rullst.github.io/Rullst/book/maturity.html)).
