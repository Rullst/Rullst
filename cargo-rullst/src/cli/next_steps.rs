//! Concrete next steps printed after a generator succeeds. The table lives
//! here, in one place; generators that already print their own instructions
//! (`new`, `auth`, `foundry:init`, `make:k8s`) are deliberately absent.
//! Nothing is printed for `--json` invocations.

use crate::ui::home::display_safe;
use crate::ui::style::{self, Style};
use clap::ArgMatches;
use std::io::Write;

/// At most this many hints follow one command.
const HINT_LIMIT: usize = 3;

/// One suggested command and why to run it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Hint {
    pub command: String,
    pub purpose: &'static str,
}

fn hint(command: impl Into<String>, purpose: &'static str) -> Hint {
    Hint {
        command: command.into(),
        purpose,
    }
}

fn flag(matches: &ArgMatches, id: &str) -> bool {
    matches
        .try_get_one::<bool>(id)
        .ok()
        .flatten()
        .copied()
        .unwrap_or(false)
}

fn value(matches: &ArgMatches, id: &str) -> Option<String> {
    matches
        .try_get_one::<String>(id)
        .ok()
        .flatten()
        .map(|value| display_safe(value))
}

const MIGRATE: &str = "cargo rullst db:migrate";
const DEV: &str = "cargo rullst dev";

/// The hints for a successful `name` invocation with `matches`.
pub(crate) fn hints(name: &str, matches: &ArgMatches) -> Vec<Hint> {
    let model = || value(matches, "name").unwrap_or_else(|| "<Name>".to_string());
    let mut hints = match name {
        "make:model" if flag(matches, "migration") => vec![
            hint(MIGRATE, "apply the new migration"),
            hint(
                format!("cargo rullst make:controller {}", model()),
                "serve the model over HTTP",
            ),
        ],
        "make:model" => vec![
            hint(
                "cargo rullst make:migration:auto",
                "derive its table migration from the struct",
            ),
            hint(
                format!("cargo rullst make:controller {}", model()),
                "serve the model over HTTP",
            ),
        ],
        "make:resource" => vec![
            hint(MIGRATE, "create the resource table"),
            hint("cargo rullst inspect routes", "list the routes to register"),
            hint(DEV, "run the app and try the new pages"),
        ],
        "make:controller" => vec![
            hint(
                "cargo rullst inspect routes",
                "check the routes you register",
            ),
            hint(DEV, "run the app"),
        ],
        "make:migration" => vec![hint(MIGRATE, "apply it once up/down are filled in")],
        "make:migration:auto" | "make:billing" | "make:mfa" | "make:chat-session" => {
            vec![hint(MIGRATE, "apply the generated migrations")]
        }
        "make:middleware" | "make:cors" | "make:jwt" | "make:live" | "make:worker" => {
            vec![hint(DEV, "run the app with the new code wired in")]
        }
        "make:island" => vec![hint(
            "cargo rullst build:client",
            "compile the island to WebAssembly",
        )],
        "make:scalar" => vec![hint(DEV, "then open /docs for the API playground")],
        "make:omni" => vec![hint("cargo rullst omni desktop", "start the desktop shell")],
        "dockerize" => vec![hint("docker build -t app .", "build the container image")],
        "generate:buildah" => vec![hint("./build_buildah.sh", "build the OCI image rootless")],
        "nixify" => vec![hint(
            "direnv allow",
            "load the Nix environment (or nix develop)",
        )],
        "generate:openapi" => vec![hint(
            "cargo rullst make:scalar",
            "serve an interactive API playground",
        )],
        "generate:ts" => vec![hint(
            "cargo rullst dev --ts-sync",
            "regenerate the client on every rebuild",
        )],
        _ => Vec::new(),
    };
    hints.truncate(HINT_LIMIT);
    hints
}

/// The "Next steps" block for `hints`; empty when there are none.
pub(crate) fn render(hints: &[Hint], style: Style) -> String {
    if hints.is_empty() {
        return String::new();
    }
    let width = hints
        .iter()
        .map(|hint| hint.command.chars().count())
        .max()
        .unwrap_or(0)
        + 3;
    let mut out = format!("\n{}\n", style.bold("Next steps", style::BRIGHT));
    for hint in hints {
        out.push_str(&format!(
            "  {} {}{}\n",
            style.bold("→", style::PASS),
            style.paint(&format!("{:<width$}", hint.command), style::BRIGHT),
            style.paint(hint.purpose, style::MUTED)
        ));
    }
    out
}

/// Prints the hints for the subcommand in `matches`, unless it ran with `--json`.
pub(crate) fn print_after_success(matches: &ArgMatches) {
    let Some((name, sub)) = matches.subcommand() else {
        return;
    };
    if flag(sub, "json") {
        return;
    }
    let text = render(&hints(name, sub), Style::stdout());
    if !text.is_empty() {
        let mut stdout = std::io::stdout();
        let _ = stdout.write_all(text.as_bytes());
        let _ = stdout.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matches_for(args: &[&str]) -> (String, ArgMatches) {
        let matches = crate::command()
            .try_get_matches_from(args)
            .expect("valid arguments");
        let (name, sub) = matches.subcommand().expect("a subcommand");
        (name.to_string(), sub.clone())
    }

    #[test]
    fn model_hints_follow_the_migration_flag() {
        let (name, sub) = matches_for(&["rullst", "make:model", "Post", "-m"]);
        assert_eq!(
            hints(&name, &sub),
            [
                hint(MIGRATE, "apply the new migration"),
                hint(
                    "cargo rullst make:controller Post",
                    "serve the model over HTTP"
                ),
            ]
        );
        let (name, sub) = matches_for(&["rullst", "make:model", "Post"]);
        assert_eq!(
            hints(&name, &sub)[0].command,
            "cargo rullst make:migration:auto"
        );
    }

    #[test]
    fn every_generator_has_at_most_three_hints_and_others_none() {
        for args in [
            &["rullst", "make:resource", "Product"][..],
            &["rullst", "make:island", "Chart"],
            &["rullst", "generate:ts"],
            &["rullst", "make:billing"],
        ] {
            let (name, sub) = matches_for(args);
            let hints = hints(&name, &sub);
            assert!((1..=HINT_LIMIT).contains(&hints.len()), "{args:?}");
        }
        for args in [
            &["rullst", "doctor"][..],
            &["rullst", "auth"],
            &["rullst", "db:status"],
        ] {
            let (name, sub) = matches_for(args);
            assert!(hints(&name, &sub).is_empty(), "{args:?}");
        }
    }

    #[test]
    fn the_block_is_aligned_plain_text() {
        let text = render(
            &[hint(MIGRATE, "apply it"), hint(DEV, "run the app")],
            Style::PLAIN,
        );
        assert_eq!(
            text,
            "\nNext steps\n  → cargo rullst db:migrate   apply it\n  → cargo rullst dev          run the app\n"
        );
        assert_eq!(render(&[], Style::PLAIN), "");
    }

    #[test]
    fn untrusted_names_are_made_display_safe() {
        let (name, sub) = matches_for(&["rullst", "make:model", "Evil\u{1b}[2J", "-m"]);
        assert_eq!(
            hints(&name, &sub)[1].command,
            "cargo rullst make:controller Evil?[2J"
        );
    }
}
