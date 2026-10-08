//! Every `cargo rullst` command shown in the golden-path tutorial must parse
//! with the complete executable command tree, so the book cannot drift from
//! the existing subcommands and flags. Packaged crates do not ship the
//! documentation; the test then has nothing to check and passes.
use std::path::Path;

const TUTORIALS: &[&str] = &["tutorials/zero-to-complete-app.md"];

/// `cargo rullst ...` invocations from shell code blocks (with `\` line
/// continuations joined) and from inline code spans in the prose.
fn commands(markdown: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut shell = false;
    let mut fenced = false;
    let mut pending = String::new();
    for line in markdown.lines() {
        let trimmed = line.trim();
        if let Some(language) = trimmed.strip_prefix("```") {
            fenced = !fenced;
            shell = fenced && matches!(language, "bash" | "sh" | "shell" | "console");
            continue;
        }
        if shell {
            let line = trimmed.trim_start_matches("$ ");
            if let Some(continued) = line.strip_suffix('\\') {
                pending.push_str(continued);
                continue;
            }
            pending.push_str(line);
            let command = std::mem::take(&mut pending);
            let command = command.split(" # ").next().unwrap_or_default();
            if command.starts_with("cargo rullst ") {
                found.push(command.to_owned());
            }
        } else if !fenced {
            found.extend(
                line.split('`')
                    .skip(1)
                    .step_by(2)
                    .filter(|span| span.starts_with("cargo rullst "))
                    .map(str::to_owned),
            );
        }
    }
    found
}

fn parses(command: &str) -> Result<(), clap::Error> {
    let arguments = command.split_whitespace().skip(2);
    crate::command()
        .try_get_matches_from(std::iter::once("rullst").chain(arguments))
        .map(|_| ())
}

#[test]
fn tutorial_commands_exist_in_the_cli() {
    let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/src");
    for tutorial in TUTORIALS {
        let Ok(markdown) = std::fs::read_to_string(docs.join(tutorial)) else {
            return; // Packaged crates do not ship the repository documentation.
        };
        let commands = commands(&markdown);
        assert!(commands.len() >= 5, "{tutorial}: too few commands found");
        for command in commands {
            if let Err(error) = parses(&command) {
                panic!("{tutorial}: `{command}` is not a valid command:\n{error}");
            }
        }
    }
}

#[test]
fn unknown_subcommands_and_flags_are_rejected() {
    assert!(parses("cargo rullst audit --report").is_ok());
    assert!(parses("cargo rullst audit --no-such-flag").is_err());
    assert!(parses("cargo rullst no-such-command").is_err());
}

#[test]
fn commands_are_found_in_shell_blocks_and_inline_code() {
    let markdown = "Run `cargo rullst dev` now.\n```bash\n$ cargo rullst new app --default \\\n  --blueprint saas # comment\ncd app\n```\n```text\ncargo rullst ignored\n```\n";
    assert_eq!(
        commands(markdown),
        [
            "cargo rullst dev",
            "cargo rullst new app --default --blueprint saas"
        ]
    );
}
