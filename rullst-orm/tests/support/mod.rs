use std::fmt::Display;

const REQUIRE_CONTAINERS_ENV: &str = "RULLST_REQUIRE_TESTCONTAINERS";

#[track_caller]
pub fn handle_container_start_error(backend: &str, error: impl Display) {
    let required = std::env::var(REQUIRE_CONTAINERS_ENV)
        .ok()
        .is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes"
            )
        });

    if required {
        panic!(
            "{backend} testcontainer is required by {REQUIRE_CONTAINERS_ENV}, but startup failed: {error}"
        );
    }

    eprintln!(
        "Skipping {backend} matrix test because Docker is unavailable; set {REQUIRE_CONTAINERS_ENV}=true to make this fatal: {error}"
    );
}
