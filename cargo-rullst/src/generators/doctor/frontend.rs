//! The Tailwind stylesheet of `cargo rullst add tailwind` is a build output:
//! `cargo rullst dev` and `cargo rullst build` rebuild it, a plain `cargo build`
//! or `cargo run` never does. This check reports a `static/app.css` that is
//! missing or older than its input or the files its `@source` lines scan.
use super::report::Check;
use crate::generators::frontend::{TAILWIND_INPUT, TAILWIND_OUTPUT, has_managed_tailwind};
use crate::generators::source_walk;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const ID: &str = "project.tailwind";
const TITLE: &str = "Tailwind stylesheet";
const FIX: &str = "Run `cargo rullst dev` (it rebuilds static/app.css on every change) or `cargo rullst build`; a plain `cargo build` or `cargo run` does not run Tailwind.";

/// `None` when the project has no managed Tailwind setup.
pub(crate) fn tailwind_check(root: &Path) -> Option<Check> {
    // One spelling for every path (macOS reaches temporary directories
    // through the /var -> /private/var link).
    let root = &root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    if !has_managed_tailwind(root) {
        return None;
    }
    let Some(built) = modified(&root.join(TAILWIND_OUTPUT)) else {
        return Some(Check::warn(
            ID,
            TITLE,
            format!("{TAILWIND_OUTPUT} has not been built"),
            FIX,
        ));
    };
    let newer = sources(root)
        .into_iter()
        .filter_map(|path| Some((modified(&path)?, path)))
        .filter(|(time, _)| *time > built)
        .max_by_key(|(time, _)| *time);
    Some(match newer {
        Some((_, path)) => Check::warn(
            ID,
            TITLE,
            format!(
                "{TAILWIND_OUTPUT} is older than {}: classes added since are missing",
                relative(root, &path)
            ),
            FIX,
        ),
        None => Check::pass(
            ID,
            TITLE,
            format!("{TAILWIND_OUTPUT} is newer than its sources"),
        ),
    })
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

fn relative(root: &Path, path: &Path) -> String {
    crate::generators::slash_path(path.strip_prefix(root).unwrap_or(path))
}

/// The input stylesheet and the files under each `@source "<dir>/**/*.<ext>"`
/// it names (relative to the stylesheet), found with the bounded walk that
/// never follows links. Other `@source` forms add nothing.
fn sources(root: &Path) -> Vec<PathBuf> {
    let input = root.join(TAILWIND_INPUT);
    let mut found = vec![input.clone()];
    let Ok(css) = std::fs::read_to_string(&input) else {
        return found;
    };
    let base = input.parent().unwrap_or(root);
    for line in css.lines() {
        let Some(glob) = line.trim().strip_prefix("@source").map(|rest| {
            rest.trim()
                .trim_end_matches(';')
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
        }) else {
            continue;
        };
        let Some((directory, extension)) = glob.split_once("/**/*.").filter(|(_, extension)| {
            !extension.is_empty() && extension.bytes().all(|byte| byte.is_ascii_alphanumeric())
        }) else {
            continue;
        };
        // Only directories inside the project are walked.
        let inside = base
            .join(directory)
            .canonicalize()
            .ok()
            .filter(|path| path.starts_with(root));
        if let Some(directory) = inside {
            found.extend(source_walk::files_with_extensions(&directory, &[extension]).files);
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generators::doctor::report::Status;
    use std::fs;
    use std::time::Duration;

    fn touch(path: &Path, seconds_ago: u64) {
        let file = fs::File::options().write(true).open(path).unwrap();
        file.set_modified(SystemTime::now() - Duration::from_secs(seconds_ago))
            .unwrap();
    }

    fn project() -> tempfile::TempDir {
        let project = tempfile::tempdir().unwrap();
        let root = project.path();
        for (path, content) in [
            (
                "assets/css/app.css",
                "@import \"tailwindcss\" source(none);\n@source \"../../src/**/*.rs\";\n",
            ),
            ("src/main.rs", "fn main() {}\n"),
            ("src/pages/home.rs", "// class=\"bg-slate-900\"\n"),
            ("static/app.css", "/* built */\n"),
        ] {
            let path = root.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, content).unwrap();
        }
        touch(&root.join("assets/css/app.css"), 300);
        touch(&root.join("src/main.rs"), 300);
        touch(&root.join("src/pages/home.rs"), 300);
        touch(&root.join("static/app.css"), 60);
        project
    }

    #[test]
    fn a_stylesheet_older_than_a_scanned_source_is_reported() {
        let project = project();
        let root = project.path();
        let check = tailwind_check(root).unwrap();
        assert_eq!(check.status, Status::Pass, "{}", check.detail);

        touch(&root.join("src/pages/home.rs"), 0);
        let check = tailwind_check(root).unwrap();
        assert_eq!(check.status, Status::Warn);
        assert_eq!(
            check.detail,
            "static/app.css is older than src/pages/home.rs: classes added since are missing"
        );
        assert!(
            check
                .fix
                .unwrap()
                .contains("plain `cargo build` or `cargo run` does not run Tailwind")
        );

        touch(&root.join("src/pages/home.rs"), 300);
        touch(&root.join("assets/css/app.css"), 0);
        let check = tailwind_check(root).unwrap();
        assert!(
            check.detail.contains("older than assets/css/app.css"),
            "{}",
            check.detail
        );
        touch(&root.join("assets/css/app.css"), 300);
        // A source outside the project is never walked.
        fs::write(
            root.join("assets/css/app.css"),
            "@import \"tailwindcss\";\n@source \"../../../**/*.rs\";\n",
        )
        .unwrap();
        touch(&root.join("assets/css/app.css"), 300);
        assert_eq!(tailwind_check(root).unwrap().status, Status::Pass);

        fs::remove_file(root.join("static/app.css")).unwrap();
        let check = tailwind_check(root).unwrap();
        assert_eq!(
            (check.status, check.detail.as_str()),
            (Status::Warn, "static/app.css has not been built")
        );
    }

    #[test]
    fn only_a_managed_tailwind_setup_is_checked() {
        let project = project();
        let root = project.path();
        fs::write(root.join("assets/css/app.css"), "body { color: red; }\n").unwrap();
        assert!(tailwind_check(root).is_none());
        // A text file outside the @source extension does not count.
        let project = self::project();
        let root = project.path();
        fs::write(root.join("src/notes.md"), "new\n").unwrap();
        assert_eq!(tailwind_check(root).unwrap().status, Status::Pass);
    }
}
