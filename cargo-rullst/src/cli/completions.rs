//! `cargo rullst completions <shell>`: shell completion scripts generated
//! from the complete runtime command tree (including the subcommands that
//! are attached outside the public `Commands` enum).

use clap::{Arg, ArgMatches, Command, value_parser};
use clap_complete::Shell;
use std::io::{self, Write};

/// The executables `cargo install cargo-rullst` installs.
const BINARIES: [&str; 2] = ["rullst", "cargo-rullst"];

pub(crate) fn command() -> Command {
    Command::new("completions")
        .about("Print a shell completion script (bash, zsh, fish, powershell or elvish)")
        .arg(
            Arg::new("shell")
                .required(true)
                .value_parser(value_parser!(Shell))
                .help("Shell to generate the script for"),
        )
        .arg(
            Arg::new("bin")
                .long("bin")
                .value_name("NAME")
                .default_value("rullst")
                .value_parser(BINARIES)
                .help("Executable the script completes"),
        )
        .after_help(
            "Examples:\n  cargo rullst completions bash > ~/.local/share/bash-completion/completions/rullst\n  \
             cargo rullst completions zsh > ~/.zfunc/_rullst\n  \
             cargo rullst completions fish > ~/.config/fish/completions/rullst.fish\n  \
             cargo rullst completions powershell >> $PROFILE",
        )
}

/// The completion script for `shell`, completing the executable `bin`.
pub(crate) fn script(shell: Shell, bin: &str) -> Vec<u8> {
    let mut command = crate::command();
    let mut buffer = Vec::new();
    // Generated into memory: clap_complete panics on a failed write, and
    // stdout may be a closed pipe.
    clap_complete::generate(shell, &mut command, bin, &mut buffer);
    buffer
}

pub(crate) fn run(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let shell = matches
        .try_get_one::<Shell>("shell")?
        .copied()
        .ok_or("a shell is required: bash, zsh, fish, powershell or elvish")?;
    let bin = matches
        .try_get_one::<String>("bin")?
        .map_or("rullst", String::as_str);
    let script = script(shell, bin);
    let mut stdout = io::stdout().lock();
    // A closed pipe (`| head`) is not a failure.
    if let Err(error) = stdout.write_all(&script).and_then(|()| stdout.flush())
        && error.kind() != io::ErrorKind::BrokenPipe
    {
        return Err(error.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shell_gets_a_script_with_runtime_commands() {
        for shell in [
            Shell::Bash,
            Shell::Zsh,
            Shell::Fish,
            Shell::PowerShell,
            Shell::Elvish,
        ] {
            let script = String::from_utf8(script(shell, "rullst")).expect("UTF-8 script");
            for command in [
                "make:model",
                "doctor",
                "completions",
                "update",
                "deploy:doctor",
            ] {
                assert!(script.contains(command), "{shell}: {command}");
            }
        }
        let cargo = String::from_utf8(script(Shell::Bash, "cargo-rullst")).expect("UTF-8");
        assert!(cargo.contains("cargo-rullst"));
    }

    #[test]
    fn the_command_accepts_the_documented_shells_only() {
        let parse = |args: &[&str]| command().try_get_matches_from(args);
        for shell in ["bash", "zsh", "fish", "powershell", "elvish"] {
            assert!(parse(&["completions", shell]).is_ok(), "{shell}");
        }
        assert!(parse(&["completions", "tcsh"]).is_err());
        assert!(parse(&["completions"]).is_err());
        assert!(parse(&["completions", "bash", "--bin", "other"]).is_err());
    }
}
