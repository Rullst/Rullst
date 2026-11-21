### Changed

- The [compatibility policy](https://github.com/Rullst/Rullst/blob/main/docs/src/compatibility-policy.md#keeping-13x-compatible)
  states the rules that keep 13.x additive: public types stay
  `#[non_exhaustive]` and gain options through options structs or builders,
  traits that applications implement gain only default methods, no new
  third-party type enters a public signature outside the listed public
  dependencies, a replaced API is deprecated with a `cargo rullst upgrade`
  rule and keeps working, stored and wire formats are versioned, and safer
  defaults reach new projects first unless safety requires otherwise.
