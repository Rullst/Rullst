#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use super::*;
use crate::ui::palette::ColorDepth;
use std::io;

fn facts(message: &str) -> Facts {
    Facts {
        message: message.to_string(),
        text: message.to_lowercase(),
        ..Facts::default()
    }
}

fn boxed(error: impl Error + Send + Sync + 'static) -> Box<dyn Error> {
    Box::new(error)
}

#[test]
fn the_common_failures_get_specific_titles_fixes_and_docs() {
    let cases = [
        (
            facts("this command must be executed in the root of a valid Rullst project."),
            "Not inside a Rullst project",
            "cargo rullst new <name>",
        ),
        (
            facts("make:iot must be run inside a Rullst project"),
            "Not inside a Rullst project",
            "project root",
        ),
        (
            facts("failed to bind 127.0.0.1:3000: Address already in use (os error 98)"),
            "Port already in use",
            "port 3000",
        ),
        (
            facts("could not write src/models/user.rs: Permission denied (os error 13)"),
            "Permission denied",
            "permissions",
        ),
        (
            facts(
                "error[E0463]: can't find crate for `core`\n  = note: the `wasm32-unknown-unknown` \
                 target may not be installed",
            ),
            "Rust target not installed",
            "rustup target add wasm32-unknown-unknown",
        ),
        (
            facts("failed to run docker: No such file or directory (os error 2)"),
            "`docker` is not installed",
            "https://docs.docker.com/get-docker/",
        ),
        (
            facts("`flyctl` not found in PATH"),
            "`flyctl` is not installed",
            "fly.io",
        ),
        (
            facts("error communicating with database: Connection refused (os error 111)"),
            "Database unreachable",
            "DATABASE_URL",
        ),
        (
            facts("error sending request for url (https://index.crates.io/): dns error"),
            "Network request failed",
            "HTTPS_PROXY",
        ),
    ];
    for (facts, title, fix) in cases {
        let friendly = classify(&facts);
        assert_eq!(friendly.title, title, "{}", facts.message);
        let rendered_fix = friendly.fix.clone().unwrap_or_default();
        assert!(rendered_fix.contains(fix), "{title}: {rendered_fix}");
        assert!(friendly.docs.is_some(), "{title} has a docs link");
        assert_eq!(
            friendly.happened, facts.message,
            "the original message is kept"
        );
    }
}

#[test]
fn ambiguous_messages_do_not_match_the_wrong_category() {
    // `cargo rullst` in an unrelated not-found message is not a missing Cargo.
    let foundry = classify(&facts(
        "Foundry.toml not found; run `cargo rullst foundry:init` first",
    ));
    assert_eq!(foundry.title, "Command failed");
    // "Rullst project has neither src/lib.rs nor src/main.rs" is a project defect.
    let layout = classify(&facts(
        "Rullst project has neither src/lib.rs nor src/main.rs",
    ));
    assert_eq!(layout.title, "Command failed");
    // A refused connection outside a database command is a network failure.
    let refused = classify(&Facts {
        io_kinds: vec![io::ErrorKind::ConnectionRefused],
        ..facts("Connection refused (os error 111)")
    });
    assert_eq!(refused.title, "Network request failed");
    let database = classify(&Facts {
        io_kinds: vec![io::ErrorKind::ConnectionRefused],
        command: Some("generate:models".to_string()),
        ..facts("Connection refused (os error 111)")
    });
    assert_eq!(database.title, "Database unreachable");
}

#[test]
fn not_in_project_points_to_an_enclosing_root_when_one_exists() {
    let friendly = classify(&Facts {
        project_required: true,
        project_root: Some("../..".to_string()),
        ..facts("anything")
    });
    assert_eq!(friendly.title, "Not inside a Rullst project");
    assert_eq!(
        friendly.fix.as_deref(),
        Some("The project root is `../..`: run `cd ../..` and try again.")
    );
    let named = classify(&Facts {
        project_required: true,
        command: Some("make:model".to_string()),
        ..facts("anything")
    });
    assert_eq!(
        named.happened,
        "`cargo rullst make:model` must run at the root of a Rullst project; this directory \
         has no Cargo.toml with a `rullst` dependency."
    );
}

#[test]
fn the_generic_fallback_names_the_failed_command() {
    let friendly = classify(&Facts {
        command: Some("academy:doctor".to_string()),
        ..facts("Academy production-boundary contract is not satisfied")
    });
    assert_eq!(friendly.title, "`cargo rullst academy:doctor` failed");
    assert_eq!(
        friendly.happened,
        "Academy production-boundary contract is not satisfied"
    );
    assert!(friendly.fix.unwrap_or_default().contains("-v"));
    assert_eq!(classify(&facts("boom")).title, "Command failed");
}

#[test]
fn invalid_input_points_to_the_command_help() {
    let friendly = classify(&Facts {
        io_kinds: vec![io::ErrorKind::InvalidInput],
        command: Some("make:mail".to_string()),
        ..facts("make:mail accepts at most one template flag")
    });
    assert_eq!(friendly.title, "Invalid input");
    assert_eq!(
        friendly.fix.as_deref(),
        Some("Check the arguments and values; `cargo rullst make:mail --help` lists them.")
    );
}

#[test]
fn interrupted_prompts_render_as_cancelled() {
    let error = boxed(dialoguer::Error::IO(io::Error::new(
        io::ErrorKind::Interrupted,
        "read interrupted",
    )));
    let facts = Facts::from_error(error.as_ref(), None, None);
    let friendly = classify(&facts);
    assert_eq!(friendly.title, "Cancelled");
    assert!(friendly.happened.contains("read interrupted"));
    assert_eq!(friendly.fix, None);
}

#[test]
fn facts_follow_the_error_chain_and_typed_errors() {
    let project = boxed(ProjectRequired);
    assert!(Facts::from_error(project.as_ref(), None, None).project_required);

    let denied = boxed(io::Error::new(io::ErrorKind::PermissionDenied, "nope"));
    let facts = Facts::from_error(denied.as_ref(), Some("make:model".into()), None);
    assert_eq!(facts.io_kinds, [io::ErrorKind::PermissionDenied]);
    assert_eq!(classify(&facts).title, "Permission denied");

    let database = boxed(sqlx::Error::PoolTimedOut);
    assert!(Facts::from_error(database.as_ref(), None, None).database);

    #[derive(Debug)]
    struct Outer(io::Error);
    impl fmt::Display for Outer {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("could not start the server")
        }
    }
    impl Error for Outer {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            Some(&self.0)
        }
    }
    let outer = boxed(Outer(io::Error::new(
        io::ErrorKind::AddrInUse,
        "Address already in use (os error 98)",
    )));
    let facts = Facts::from_error(outer.as_ref(), None, None);
    assert_eq!(facts.io_kinds, [io::ErrorKind::AddrInUse]);
    assert_eq!(classify(&facts).title, "Port already in use");
    assert_eq!(
        causes(outer.as_ref()),
        ["Address already in use (os error 98)"]
    );
}

#[test]
fn plain_rendering_is_stable_and_redacted() {
    let friendly = Friendly {
        title: "Database unreachable".to_string(),
        happened: "connect postgres://app:hunter2@db:5432/x failed\nsecond line".to_string(),
        fix: Some("Start the database.".to_string()),
        docs: Some("https://example.test/docs"),
    };
    let text = render(
        &friendly,
        &["inner APP_KEY=abc".to_string()],
        None,
        false,
        Style::PLAIN,
    );
    assert_eq!(
        text,
        "error: Database unreachable\n\
         \x20 What happened  connect postgres://app:***@db:5432/x failed\n\
         \x20                second line\n\
         \x20 How to fix     Start the database.\n\
         \x20 Docs           https://example.test/docs\n\
         \x20 Run again with -v to see the underlying causes.\n"
    );
    assert!(!text.contains("hunter2"));

    let verbose = render(
        &friendly,
        &["inner APP_KEY=abc".to_string()],
        Some("Custom { password=hunter2 }"),
        true,
        Style::PLAIN,
    );
    assert!(verbose.contains("  Caused by      1. inner APP_KEY=***\n"));
    assert!(verbose.contains("  Debug          Custom { password=*** }\n"));
    assert!(!verbose.contains("hunter2") && !verbose.contains("abc"));
}

#[test]
fn coloured_rendering_marks_the_title_and_keeps_the_text() {
    let friendly = classify(&facts("boom"));
    let text = render(
        &friendly,
        &[],
        None,
        false,
        Style::with_depth(ColorDepth::Ansi256),
    );
    assert!(text.starts_with("\x1b[1m"));
    assert!(text.contains("✗"));
    assert!(text.contains("boom"));
    assert!(!text.contains("error: "));
}

#[test]
fn verbosity_and_command_names_come_from_the_arguments() {
    let args = |list: &[&str]| {
        list.iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
    };
    assert!(verbose_flag(&args(&["cargo-rullst", "doctor", "-v"])));
    assert!(verbose_flag(&args(&["cargo-rullst", "--verbose", "dev"])));
    assert!(verbose_flag(&args(&["cargo-rullst", "-vv", "dev"])));
    assert!(!verbose_flag(&args(&["cargo-rullst", "dev", "--", "-v"])));
    assert!(!verbose_flag(&args(&[
        "cargo-rullst",
        "-",
        "-x",
        "make:model"
    ])));
    assert!(!backtrace_requested(None));
    assert!(!backtrace_requested(Some(std::ffi::OsStr::new("0"))));
    assert!(backtrace_requested(Some(std::ffi::OsStr::new("1"))));

    assert_eq!(
        command_name(&args(&["cargo-rullst", "rullst", "make:model", "Post"])).as_deref(),
        Some("make:model")
    );
    assert_eq!(
        command_name(&args(&["rullst", "-v", "doctor"])).as_deref(),
        Some("doctor")
    );
    assert_eq!(command_name(&args(&["rullst"])), None);
}

#[test]
fn already_reported_failures_keep_their_status_and_print_nothing_more() {
    let error = boxed(AlreadyReported::new(USAGE_EXIT, "unknown command `x`"));
    assert_eq!(error.to_string(), "unknown command `x`");
    assert_eq!(report(error.as_ref(), &[]), 2);
    assert_eq!(AlreadyReported::new(0, "zero becomes failure").code, 1);
}
