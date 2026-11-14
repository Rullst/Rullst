//! The Dockerfile stages that build the Tailwind stylesheet of a project
//! (`cargo rullst add tailwind`) with the pinned CLI.

use super::tailwind::Platform;
use super::tailwind_release::download_url;
use crate::generators::project::docker_static;

/// Starts the stages, so an edited Dockerfile is recognised as having them.
pub(crate) const STAGES_MARKER: &str = "AS tailwind-cli-amd64";

/// The line the runtime stage gains after `COPY … static /app/static` in a
/// Dockerfile generated before 13.0.0-alpha.5.
pub(crate) const COPY_LINE: &str =
    "COPY --from=tailwind --chown=10001:10001 /css/app.css /app/static/app.css\n";

/// The line the `static-assets` stage (13.0.0-alpha.5) gains after its copy
/// of `static/`, so the built stylesheet gets its pre-compressed siblings.
pub(crate) const STATIC_STAGE_COPY_LINE: &str =
    "COPY --from=tailwind /css/app.css /static/app.css\n";

/// The stages placed before the runtime stage. BuildKit (`ADD --checksum`)
/// downloads the CLI for the build machine's architecture and refuses it
/// unless its SHA-256 is the pinned one; the stylesheet is then minified on
/// the build platform, since CSS does not depend on the target.
pub(crate) fn stages() -> String {
    let asset = |platform: Platform| {
        platform
            .asset()
            .map(|asset| (download_url(asset), asset.sha256))
    };
    let mut out = String::from(
        "# Tailwind CSS (`cargo rullst add tailwind`): BuildKit downloads the pinned\n\
         # standalone CLI for the build machine's architecture (amd64 or arm64) and\n\
         # refuses it unless its SHA-256 matches; then the stylesheet is minified.\n\
         # Needs BuildKit: Docker 25+, `docker buildx` or Podman 4.8+.\n",
    );
    for (arch, platform) in [
        ("amd64", Platform::LinuxX64),
        ("arm64", Platform::LinuxArm64),
    ] {
        if let Some((url, sha256)) = asset(platform) {
            out.push_str(&format!(
                "FROM scratch AS tailwind-cli-{arch}\n\
                 ADD --chmod=755 --checksum=sha256:{sha256} \\\n    {url} /tailwindcss\n"
            ));
        }
    }
    out.push_str(
        "FROM tailwind-cli-${BUILDARCH} AS tailwind-cli\n\n\
         FROM --platform=$BUILDPLATFORM rust:1.99.1-slim-bookworm AS tailwind\n\
         WORKDIR /app\n\
         COPY --from=tailwind-cli /tailwindcss /usr/local/bin/tailwindcss\n\
         COPY . .\n\
         RUN tailwindcss --input assets/css/app.css --output /css/app.css --minify\n\n",
    );
    out
}

/// Adds the stages and the copy line to a Dockerfile `cargo rullst new`,
/// `deploy` or `dockerize` generated, or `None` when the file is not
/// recognised (or already has them).
pub(crate) fn add_to(dockerfile: &str) -> Option<String> {
    if dockerfile.contains(STAGES_MARKER) {
        return None;
    }
    if let Some(stage) = docker_static::stage_start(dockerfile) {
        return add_before_static_stage(dockerfile, stage);
    }
    let runtime = dockerfile
        .find("# Debian 12 glibc runtime")
        .or_else(|| dockerfile.find("FROM gcr.io/distroless/cc-debian12:nonroot"))?;
    let static_copy = "COPY --chown=10001:10001 static /app/static\n";
    let static_at = dockerfile.find(static_copy)?;
    if static_at < runtime {
        return None;
    }
    let copy_at = static_at + static_copy.len();
    let mut out = String::with_capacity(dockerfile.len() + 1024);
    out.push_str(&dockerfile[..runtime]);
    out.push_str(&stages());
    out.push_str(&dockerfile[runtime..copy_at]);
    out.push_str(COPY_LINE);
    out.push_str(&dockerfile[copy_at..]);
    Some(out)
}

/// [`add_to`] for a Dockerfile with the `static-assets` stage starting at
/// `stage`: the Tailwind stages go before it and the stylesheet into it, so
/// it is compressed with the other static files.
fn add_before_static_stage(dockerfile: &str, stage: usize) -> Option<String> {
    let body = stage
        + dockerfile[stage..].find(docker_static::STAGE_MARKER)?
        + docker_static::STAGE_MARKER.len();
    let end = dockerfile[body..]
        .find("\nFROM ")
        .map_or(dockerfile.len(), |at| body + at);
    let copy_at = body
        + dockerfile[body..end].find(docker_static::STATIC_COPY)?
        + docker_static::STATIC_COPY.len();
    let mut out = String::with_capacity(dockerfile.len() + 1024);
    out.push_str(&dockerfile[..stage]);
    out.push_str(&stages());
    out.push_str(&dockerfile[stage..copy_at]);
    out.push_str(STATIC_STAGE_COPY_LINE);
    out.push_str(&dockerfile[copy_at..]);
    Some(out)
}
