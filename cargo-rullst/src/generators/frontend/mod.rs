//! The opt-in frontend toolchain behind `cargo rullst add tailwind` and
//! `cargo rullst add alpine`: a pinned Tailwind CSS standalone CLI (one
//! binary, no Node.js) verified by SHA-256 before every use, and a vendored
//! Alpine.js CSP build verified by its npm `dist.integrity`. Rullst bundles
//! no CSS compiler or JavaScript bundler; maintaining either tool is bumping
//! one pin file (`tailwind_release.rs`, `alpine_release.rs`).
//!
//! The Tailwind CLI is cached per user and verified each time `dev`, `build`
//! or `add` is about to run it. The check and the start are not atomic: a
//! process running as the same user that swaps the file in between is
//! outside this protection, as it is for the rest of the user's cache.

pub(crate) mod alpine;
pub(crate) mod alpine_release;
pub(crate) mod docker;
mod fetch;
pub(crate) mod tailwind;
pub(crate) mod tailwind_release;
#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) use fetch::{FetchError, Fixture};
pub(crate) use fetch::{Fetcher, HttpFetcher};

use std::path::Path;

/// The Tailwind input stylesheet `cargo rullst add tailwind` writes.
pub(crate) const TAILWIND_INPUT: &str = "assets/css/app.css";
/// The stylesheet the Tailwind CLI writes, served at `/static/app.css` and
/// linked with `rullst::assets::url("app.css")`.
pub(crate) const TAILWIND_OUTPUT: &str = "static/app.css";
/// The vendored Alpine.js CSP build `cargo rullst add alpine` writes.
pub(crate) const ALPINE_VENDORED: &str = "static/vendor/alpine-csp.min.js";
/// The example components file `cargo rullst add alpine` writes.
pub(crate) const ALPINE_COMPONENTS: &str = "static/js/alpine-components.js";

/// Largest stylesheet or manifest read while detecting Tailwind.
const DETECTION_LIMIT: u64 = 1024 * 1024;

fn read_small(path: &Path) -> Option<String> {
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > DETECTION_LIMIT {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

/// Whether `css` has an `@import` of `tailwindcss` (Tailwind v4's entry).
pub(crate) fn imports_tailwind(css: &str) -> bool {
    css.lines().any(|line| {
        line.trim_start()
            .strip_prefix("@import")
            .is_some_and(|rest| {
                let rest = rest.trim_start();
                rest.starts_with("\"tailwindcss\"") || rest.starts_with("'tailwindcss'")
            })
    })
}

/// Whether the project at `root` builds its stylesheet with the Tailwind CLI
/// that Rullst manages: [`TAILWIND_INPUT`] imports `tailwindcss`. `dev`,
/// `build` and the generated Dockerfile run Tailwind only then.
pub(crate) fn has_managed_tailwind(root: &Path) -> bool {
    read_small(&root.join(TAILWIND_INPUT)).is_some_and(|css| imports_tailwind(&css))
}

/// Does this project use Tailwind CSS? True for [`has_managed_tailwind`] and
/// for a project that builds Tailwind itself: a `tailwind.config.*` file or
/// a `package.json` that depends on `tailwindcss` or `@tailwindcss/*`.
/// Generators use it to choose Tailwind utility classes over their own CSS.
pub(crate) fn uses_tailwind(root: &Path) -> bool {
    if has_managed_tailwind(root) {
        return true;
    }
    let configured = ["js", "cjs", "mjs", "ts"]
        .iter()
        .any(|extension| root.join(format!("tailwind.config.{extension}")).is_file());
    configured
        || read_small(&root.join("package.json")).is_some_and(|manifest| {
            manifest.contains("\"tailwindcss\"") || manifest.contains("\"@tailwindcss/")
        })
}

/// `cargo rullst build`: when the project uses the managed Tailwind setup,
/// verifies (downloading when missing) the pinned CLI and writes the
/// minified [`TAILWIND_OUTPUT`]. A failure stops the build, so a release
/// never ships a stale stylesheet.
#[cfg_attr(mutants, mutants::skip)]
pub(crate) fn build_release_stylesheet(root: &Path) -> Result<(), tailwind::ToolError> {
    if !has_managed_tailwind(root) {
        return Ok(());
    }
    println!(
        "🎨 Building {TAILWIND_OUTPUT} with tailwindcss v{} --minify...",
        tailwind_release::VERSION
    );
    let verified = tailwind::ensure_host(&HttpFetcher)?;
    tailwind::run_build(&verified.path, root, true)
}
