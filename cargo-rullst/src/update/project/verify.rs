use super::{ProjectError, process, snapshot, state::State};
use clap::{Arg, ArgAction, ArgMatches, Command};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

pub(super) fn command() -> Command {
    Command::new("verify").about("Check and test a prepared trusted project; this executes project code")
        .arg(Arg::new("prepared").long("prepared").required(true).value_name("DIRECTORY").value_parser(clap::value_parser!(PathBuf)))
        .arg(Arg::new("allow-project-code").long("allow-project-code").action(ArgAction::SetTrue)
            .help("Authorize this invocation to execute trusted build scripts, procedural macros and tests; the copy is not a sandbox"))
        .arg(Arg::new("allow-network").long("allow-network").action(ArgAction::SetTrue))
        .arg(Arg::new("dry-run").long("dry-run").action(ArgAction::SetTrue).conflicts_with("allow-project-code")
            .help("Review validated inputs and exact Cargo commands without builds or tests"))
        .arg(Arg::new("all-features").long("all-features").action(ArgAction::SetTrue).conflicts_with("features"))
        .arg(Arg::new("features").long("features").value_name("COMMA_SEPARATED"))
        .arg(Arg::new("no-default-features").long("no-default-features").action(ArgAction::SetTrue).conflicts_with("all-features"))
        .arg(Arg::new("timeout-seconds").long("timeout-seconds").default_value("900").value_parser(clap::value_parser!(u64).range(1..=3600)))
        .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
}

pub(super) fn run(matches: &ArgMatches) -> Result<(), ProjectError> {
    if !matches.get_flag("allow-project-code") && !matches.get_flag("dry-run") {
        return Err(ProjectError::Invalid(
            "review the preparation first; --allow-project-code is required because Cargo builds and tests execute project code",
        ));
    }
    let offline = !matches.get_flag("allow-network");
    if !offline
        && std::env::var("CARGO_NET_OFFLINE")
            .is_ok_and(|value| matches!(value.as_str(), "1" | "true"))
    {
        return Err(ProjectError::Invalid(
            "--allow-network conflicts with CARGO_NET_OFFLINE",
        ));
    }
    let features = features(matches)?;
    let selected = matches
        .get_one::<PathBuf>("prepared")
        .ok_or(ProjectError::Invalid("a preparation is required"))?;
    let locked = super::super::cache::open_project(selected)?;
    let state = State::load(&locked.path)?;
    let commands = commands(&features, offline);
    if matches.get_flag("dry-run") {
        let report = serde_json::json!({"schema_version":"rullst.project-verification-plan.v1", "prepared_directory":locked.path,
            "commands":commands, "features":features, "offline":offline,
            "execution_authorized":false,"application_authorized":false,
            "execution_location":"fresh private copy; not a sandbox"});
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    let workspace = super::super::cache::project_workspace()?;
    let candidate = workspace.path().join("candidate");
    let copied = snapshot::copy(
        &state.candidate,
        &workspace.path().join("before"),
        &candidate,
        &state.paths,
    )?;
    if copied != state.candidate_records {
        return Err(ProjectError::Invalid(
            "candidate changed while copying verification inputs",
        ));
    }
    let timeout = Duration::from_secs(
        *matches
            .get_one::<u64>("timeout-seconds")
            .ok_or(ProjectError::Invalid("missing verification deadline"))?,
    );
    let mut observations = Vec::new();
    for (index, command) in commands.iter().enumerate() {
        let args = &command.args;
        if args.first().is_some_and(|arg| arg == "check") {
            crate::generators::build::validate_prepared_resolution(
                &candidate,
                &state.prepared.target,
            )
            .map_err(|error| ProjectError::Planning(error.to_string()))?;
        }
        if !matches.get_flag("json") {
            eprintln!(
                "Verifying candidate: {} {}",
                command.program,
                args.join(" ")
            );
        }
        let result = process::execute(
            command.program,
            args,
            &candidate,
            &workspace.path().join("build-output"),
            timeout,
            offline,
        );
        let (success, output, diagnostics) = match result {
            Ok(result) => result,
            Err(error) => {
                let path = workspace.retain();
                return Err(ProjectError::Planning(format!(
                    "{error}; failed verification retained at {}",
                    path.display()
                )));
            }
        };
        let stdout = format!("command-{index}.stdout");
        let stderr = format!("command-{index}.stderr");
        fs::write(workspace.path().join(&stdout), &output)?;
        fs::write(workspace.path().join(&stderr), &diagnostics)?;
        if !success {
            let path = workspace.retain();
            return Err(ProjectError::Planning(format!(
                "Cargo verification failed; private diagnostics at {}; no acceptance or application was recorded",
                path.join(stderr).display()
            )));
        }
        observations.push(
            serde_json::json!({"program":command.program,"args":args,"success":true,
            "stdout":stdout,"stdout_sha256":hex::encode(Sha256::digest(&output)),
            "stderr":stderr,"stderr_sha256":hex::encode(Sha256::digest(&diagnostics))}),
        );
    }
    state.unchanged()?;
    crate::generators::build::validate_prepared_resolution(&candidate, &state.prepared.target)
        .map_err(|error| ProjectError::Planning(error.to_string()))?;
    let files = accepted_files(&candidate, &state)?;
    let report = serde_json::json!({"schema_version":"rullst.project-verification.v1", "phase":"verified",
        "prepared_directory":locked.path,"verified_directory":workspace.path(),"verified_candidate_directory":candidate,
        "source":state.prepared.source,"target":state.prepared.target,"platform":std::env::consts::OS,
        "features":features,"offline":offline,"commands":observations,"files":files,
        "execution_authorized_for_this_invocation":true,"application_authorized":false,"production_ready":false});
    fs::write(
        workspace.path().join("verification.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    let path = workspace.retain();
    if matches.get_flag("json") {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "Candidate checks/tests passed. Review: {}",
            path.join("verification.json").display()
        );
        println!(
            "Original project files were not edited. Application and deployment remain separate steps."
        );
    }
    Ok(())
}

pub(super) fn features(matches: &ArgMatches) -> Result<Vec<String>, ProjectError> {
    let mut args = Vec::new();
    if matches.get_flag("all-features") {
        args.push("--all-features".into());
    }
    if matches.get_flag("no-default-features") {
        args.push("--no-default-features".into());
    }
    if let Some(features) = matches.get_one::<String>("features") {
        if features.len() > 2048
            || features.split(',').any(|name| {
                name.is_empty()
                    || name.len() > 128
                    || !name.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'/')
                    })
            })
        {
            return Err(ProjectError::Invalid(
                "use bounded comma-separated Cargo feature names",
            ));
        }
        args.extend(["--features".into(), features.clone()]);
    }
    Ok(args)
}

#[derive(serde::Serialize)]
pub(super) struct VerificationCommand {
    pub program: &'static str,
    pub args: Vec<String>,
}

pub(super) fn commands(features: &[String], offline: bool) -> Vec<VerificationCommand> {
    let mut commands = vec![
        VerificationCommand {
            program: "rustc",
            args: vec!["--version".into(), "--verbose".into()],
        },
        VerificationCommand {
            program: "cargo",
            args: vec!["--version".into()],
        },
    ];
    let mut resolve = vec!["generate-lockfile".into()];
    if offline {
        resolve.push("--offline".into());
    }
    commands.push(VerificationCommand {
        program: "cargo",
        args: resolve,
    });
    for operation in ["check", "test"] {
        let mut args = vec![operation.into(), "--workspace".into(), "--locked".into()];
        if operation == "check" {
            args.push("--all-targets".into());
        }
        if offline {
            args.push("--offline".into());
        }
        args.extend_from_slice(features);
        commands.push(VerificationCommand {
            program: "cargo",
            args,
        });
    }
    commands
}

fn accepted_files(root: &Path, state: &State) -> Result<Vec<snapshot::Record>, ProjectError> {
    let mut expected: BTreeSet<_> = state
        .candidate_records
        .iter()
        .filter(|file| file.sha256.is_some())
        .map(|file| file.path.clone())
        .collect();
    expected.insert("Cargo.lock".into());
    if snapshot::tree(root)? != expected {
        return Err(ProjectError::Invalid(
            "verification generated or removed unexpected source files",
        ));
    }
    let mut files = Vec::new();
    let mut total = 0u64;
    for before in &state.candidate_records {
        let body = snapshot::read(root, &before.path)?;
        let file = snapshot::record(&before.path, body.as_deref());
        total = total
            .checked_add(file.bytes)
            .ok_or(ProjectError::Invalid("verified source size overflow"))?;
        if total > snapshot::MAX_TOTAL {
            return Err(ProjectError::Invalid("verified source exceeds 512 MiB"));
        }
        if before.path != "Cargo.lock" && file != *before {
            return Err(ProjectError::Invalid(
                "project execution changed a prepared source input",
            ));
        }
        files.push(file);
    }
    Ok(files)
}
