//! The Dockerfile stage that assembles `static/` for the runtime image and
//! writes the Brotli (`.br`) and Zstandard (`.zst`) siblings that
//! `cargo rullst build` writes locally, so the deployed server can send them.
//! The `rullst` CLI is not installed in the image: Debian's `brotli` and
//! `zstd` tools compress the same file types with the same levels.

use crate::generators::build::precompressed::COMPRESSED_EXTENSIONS;

/// Ends the stage's `FROM` line.
pub(crate) const STAGE_MARKER: &str = " AS static-assets\n";

/// The stage's copy of the project's `static/`. Files other stages build
/// (`cargo rullst add tailwind`'s stylesheet) are copied in after it, before
/// the siblings are written.
pub(crate) const STATIC_COPY: &str = "COPY static /static\n";

/// The runtime stage's copy of the assembled directory.
pub(crate) const RUNTIME_COPY: &str =
    "COPY --from=static-assets --chown=10001:10001 /static /app/static\n";

/// The stage, placed before the runtime stage.
pub(crate) fn stage() -> String {
    let names = COMPRESSED_EXTENSIONS
        .chunks(4)
        .map(|chunk| {
            chunk
                .iter()
                .map(|extension| format!("-iname '*.{extension}'"))
                .collect::<Vec<_>>()
                .join(" -o ")
        })
        .collect::<Vec<_>>()
        .join(" -o \\\n        ");
    format!(
        "# Static files with the Brotli (.br) and Zstandard (.zst) siblings that
# `cargo rullst build` writes (same file types and levels): the server sends a
# sibling to each client that accepts its encoding. Siblings already in static/
# are written again from the files this image serves.
FROM rust:1.99.1-slim-bookworm{STAGE_MARKER}RUN apt-get update \\
    && apt-get install --yes --no-install-recommends brotli zstd \\
    && rm -rf /var/lib/apt/lists/*
{STATIC_COPY}RUN find /static -type f \\( {names} \\) \\
    -exec brotli --force --quality=11 --lgwin=22 {{}} + \\
    -exec zstd --force --quiet -19 {{}} +

"
    )
}

/// Where stages that the static stage copies from go: the start of the
/// comment above its `FROM` line, or `None` without the stage.
pub(crate) fn stage_start(dockerfile: &str) -> Option<usize> {
    let marker = dockerfile.find(STAGE_MARKER)?;
    let mut start = dockerfile[..marker].rfind('\n').map_or(0, |at| at + 1);
    while start > 0 {
        let previous = dockerfile[..start - 1].rfind('\n').map_or(0, |at| at + 1);
        if !dockerfile[previous..start].starts_with('#') {
            break;
        }
        start = previous;
    }
    Some(start)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn the_stage_compresses_the_types_cargo_rullst_build_compresses() {
        let stage = stage();
        for extension in COMPRESSED_EXTENSIONS {
            assert!(
                stage.contains(&format!("-iname '*.{extension}'")),
                "{stage}"
            );
        }
        assert!(stage.contains("FROM rust:1.99.1-slim-bookworm AS static-assets\n"));
        assert!(stage.contains("brotli --force --quality=11 --lgwin=22 {} +"));
        assert!(stage.contains("zstd --force --quiet -19 {} +"));
        assert!(stage.find(STATIC_COPY) < stage.find("RUN find"));
        assert_eq!(
            stage_start(&format!("FROM a AS builder\n\n{stage}FROM b\n")),
            Some("FROM a AS builder\n\n".len())
        );
        assert_eq!(stage_start(&stage), Some(0));
        assert_eq!(
            stage_start(&format!(
                "FROM a\nRUN b\n{}",
                &stage[stage.find("FROM").unwrap()..]
            )),
            Some("FROM a\nRUN b\n".len())
        );
        assert_eq!(stage_start("FROM a AS builder\n"), None);
    }

    #[test]
    fn tailwind_adds_the_stylesheet_inside_the_static_stage() {
        use crate::generators::frontend::docker;
        let generated = format!(
            "FROM rust AS builder\nRUN build\n\n{}# Debian 12 glibc runtime (x)\nFROM gcr.io/distroless/cc-debian12:nonroot\n{RUNTIME_COPY}CMD [\"/app/demo\"]\n",
            stage()
        );
        let edited = docker::add_to(&generated).expect("recognised Dockerfile");
        // The Tailwind stages come before the static stage and its comment,
        // and the stylesheet is copied in before the siblings are written.
        assert!(edited.starts_with("FROM rust AS builder\nRUN build\n\n# Tailwind CSS"));
        assert!(edited.contains("--minify\n\n# Static files with the Brotli"));
        assert!(edited.contains(&format!("{STAGE_MARKER}RUN apt-get update")));
        assert!(edited.contains(&format!(
            "{STATIC_COPY}{}RUN find /static",
            docker::STATIC_STAGE_COPY_LINE
        )));
        assert!(!edited.contains(docker::COPY_LINE));
        assert!(edited.contains(&format!("nonroot\n{RUNTIME_COPY}CMD")));
        assert_eq!(docker::add_to(&edited), None);
        // A static copy outside the stage is not where the stylesheet goes.
        let elsewhere = generated.replacen(STATIC_COPY, "", 1) + "FROM x\n" + STATIC_COPY;
        assert_eq!(docker::add_to(&elsewhere), None);
    }

    /// Runs the stage's `find` with stand-ins for `brotli` and `zstd` that
    /// create the sibling they would write, in place of `/static`.
    #[cfg(unix)]
    #[test]
    fn the_stage_writes_siblings_of_eligible_files_only() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().expect("temporary directory");
        let tools = root.path().join("tools");
        let assets = root.path().join("static");
        fs::create_dir_all(&tools).expect("tools directory");
        fs::create_dir_all(assets.join("js")).expect("static directory");
        for (tool, suffix) in [("brotli", "br"), ("zstd", "zst")] {
            let path = tools.join(tool);
            fs::write(
                &path,
                format!(
                    "#!/bin/sh\nfor arg; do case \"$arg\" in -*) ;; *) : > \"$arg.{suffix}\" ;; esac; done\n"
                ),
            )
            .expect("stand-in tool");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("executable");
        }
        for file in [
            "app.css",
            "js/htmx.min.js",
            "INDEX.HTML",
            "photo.png",
            "old.css.br",
            "data.json.zst",
        ] {
            fs::write(assets.join(file), "x").expect("asset");
        }
        let stage = stage();
        let command = stage
            .split_once("RUN find /static")
            .expect("compression step")
            .1
            .trim_end();
        let script = format!("find '{}'{command}", assets.display());
        let path = format!(
            "{}:{}",
            tools.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(&script)
            .env("PATH", path)
            .status()
            .expect("run the compression step");
        assert!(status.success(), "{script}");
        for sibling in [
            "app.css.br",
            "app.css.zst",
            "js/htmx.min.js.br",
            "js/htmx.min.js.zst",
            "INDEX.HTML.br",
            "INDEX.HTML.zst",
        ] {
            assert!(assets.join(sibling).is_file(), "{sibling}");
        }
        for absent in [
            "photo.png.br",
            "photo.png.zst",
            "old.css.br.br",
            "old.css.br.zst",
            "data.json.zst.br",
            "data.json.zst.zst",
        ] {
            assert!(!assets.join(absent).exists(), "{absent}");
        }
    }
}
