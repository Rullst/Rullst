#![cfg(all(
    feature = "privacy-challenge-tokens",
    feature = "privacy-sqlite",
    feature = "privacy-consent-sqlite"
))]
include!("../../.github/fixtures/privacy-facade.rs");
