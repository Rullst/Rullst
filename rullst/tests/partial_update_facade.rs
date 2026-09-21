#![cfg(all(
    feature = "orm",
    not(any(feature = "strict-postgres", feature = "strict-mysql"))
))]
#![allow(clippy::expect_used, clippy::unwrap_used)]
include!("../../.github/fixtures/partial-update-facade.rs");
