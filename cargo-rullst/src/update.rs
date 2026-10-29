//! Project update preparation (`cargo rullst update project …` and the
//! interactive `update guided`) and the daily "a newer cargo-rullst is
//! available" notice. The CLI never downloads or replaces its own binary:
//! install a new version with `cargo install cargo-rullst --version =X --locked`.
use clap::{ArgMatches, Command};

#[path = "update/cache.rs"]
mod cache;
#[path = "update/guided.rs"]
mod guided;
#[path = "update/notice.rs"]
pub(crate) mod notice;
#[path = "update/project.rs"]
mod project;

pub(crate) fn command() -> Command {
    Command::new("update")
        .about("Prepare, verify, review and apply a framework update of a project")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(project::command())
        .subcommand(guided::command())
}

pub(crate) fn run(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(matches) = matches.subcommand_matches("guided") {
        return guided::run(matches);
    }
    let Some(matches) = matches.subcommand_matches("project") else {
        return Err("unsupported update operation".into());
    };
    project::run(matches).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_project_preparation_and_the_guided_flow_remain() {
        let names: Vec<String> = command()
            .get_subcommands()
            .map(|command| command.get_name().to_string())
            .collect();
        assert_eq!(names, ["project", "guided"]);
        for removed in [
            vec!["update", "check"],
            vec!["update", "verify"],
            vec!["update", "stage"],
            vec!["update", "install", "review"],
            vec!["update", "apply"],
        ] {
            let error = command().try_get_matches_from(&removed).unwrap_err();
            assert_eq!(
                error.kind(),
                clap::error::ErrorKind::InvalidSubcommand,
                "{removed:?}"
            );
        }
    }
}
