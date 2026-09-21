//! Bounded inspection of local configuration snapshots, without deployment claims.
use clap::{Arg, ArgAction, ArgMatches, Command};
use serde::Serialize;
use std::io::{self, Write};

mod evaluate;
mod input;

#[derive(Debug, thiserror::Error)]
pub(crate) enum DoctorError {
    #[error("deployment configuration inspection is incomplete or found errors")]
    InspectionFailed,
    #[error("could not write deployment diagnostic")]
    Output,
}

#[derive(Serialize)]
struct Report {
    schema_version: &'static str,
    scope: &'static str,
    deployment_verified: bool,
    inspection_complete: bool,
    local_errors_found: bool,
    environment_source: &'static str,
    checks: Vec<Check>,
    not_inspected: &'static [&'static str],
}

#[derive(Serialize)]
struct Check {
    code: &'static str,
    status: &'static str,
    guidance: &'static str,
}

impl Report {
    fn new(source: &'static str) -> Self {
        Self {
            schema_version: "rullst.deployment-diagnostic.v1",
            scope: "local_configuration_snapshot",
            deployment_verified: false,
            inspection_complete: false,
            local_errors_found: false,
            environment_source: source,
            checks: Vec::new(),
            not_inspected: &[
                "actual_runtime_environment_and_application_overrides",
                "application_key_randomness_custody_and_rotation",
                "mounted_middleware_and_signed_webhook_verification",
                "authentication_tenant_and_object_authorization",
                "tls_proxy_trust_and_public_network_exposure",
                "distributed_limits_replay_and_session_state",
                "readiness_shutdown_and_websocket_lifecycle",
                "provider_credentials_and_live_integrations",
                "host_permissions_backups_and_restore",
            ],
        }
    }

    fn add(&mut self, code: &'static str, status: &'static str, guidance: &'static str) {
        self.local_errors_found |= status == "FAIL";
        self.checks.push(Check {
            code,
            status,
            guidance,
        });
    }
}

pub(crate) fn command() -> Command {
    Command::new("deploy:doctor")
        .about(
            "Inspect a local deployment configuration snapshot without network access or changes",
        )
        .arg(
            Arg::new("config").long("config").value_name("FILE").help(
                "Rullst TOML file; otherwise inspect ./Rullst.toml or Core defaults if absent",
            ),
        )
        .arg(
            Arg::new("env-file")
                .long("env-file")
                .value_name("FILE")
                .conflicts_with("process-env")
                .help("Explicit literal dotenv snapshot; never merged with process variables"),
        )
        .arg(
            Arg::new("process-env")
                .long("process-env")
                .action(ArgAction::SetTrue)
                .help("Inspect only process RULLST_ENV, APP_ENV and APP_KEY; never load dotenv"),
        )
        .arg(
            Arg::new("target")
                .long("target")
                .value_parser(["production", "staging"])
                .default_value("production")
                .help("Expected environment for this snapshot"),
        )
        .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
}

pub(crate) fn run(matches: &ArgMatches) -> Result<(), DoctorError> {
    let source = if matches.get_flag("process-env") {
        "process_allowlist"
    } else if matches.contains_id("env-file") {
        "explicit_env_file"
    } else {
        "not_selected"
    };
    let mut report = Report::new(source);
    match input::load(matches) {
        Ok(snapshot) => evaluate::inspect(&snapshot, &mut report),
        Err(error) => report.add(error.code(), "FAIL", error.guidance()),
    }
    let stdout = io::stdout();
    let mut out = stdout.lock();
    if matches.get_flag("json") {
        serde_json::to_writer_pretty(&mut out, &report).map_err(|_| DoctorError::Output)?;
        writeln!(out).map_err(|_| DoctorError::Output)?;
    } else {
        writeln!(
            out,
            "Deployment diagnostic: local configuration snapshot ({source})."
        )
        .map_err(|_| DoctorError::Output)?;
        for check in &report.checks {
            writeln!(out, "[{}] {}: {}", check.status, check.code, check.guidance)
                .map_err(|_| DoctorError::Output)?;
        }
        writeln!(
            out,
            "Deployment has not been verified. Controls not inspected:"
        )
        .map_err(|_| DoctorError::Output)?;
        for control in report.not_inspected {
            writeln!(out, "- {control}").map_err(|_| DoctorError::Output)?;
        }
    }
    if report.local_errors_found || !report.inspection_complete {
        return Err(DoctorError::InspectionFailed);
    }
    Ok(())
}
