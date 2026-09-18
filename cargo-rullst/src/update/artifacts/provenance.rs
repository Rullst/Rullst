//! Publisher policy uses certificate-bound gh flags, not workflow-supplied JSON.
use super::{
    ArtifactError,
    manifest::{Manifest, REPOSITORY},
};
use std::{
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

fn command(program: &Path, path: &Path, manifest: &Manifest) -> Command {
    let mut command = Command::new(program);
    command
        .args(["attestation", "verify"])
        .arg(path)
        .args([
            "--hostname",
            "github.com",
            "--repo",
            REPOSITORY,
            "--cert-oidc-issuer",
            "https://token.actions.githubusercontent.com",
            "--predicate-type",
            "https://slsa.dev/provenance/v1",
            "--source-ref",
            &format!("refs/tags/{}", manifest.release_tag),
            "--source-digest",
            &manifest.source_commit,
            "--signer-digest",
            &manifest.source_commit,
            "--cert-identity",
            &format!(
                "https://github.com/Rullst/Rullst/.github/workflows/release.yml@refs/tags/{}",
                manifest.release_tag
            ),
            "--deny-self-hosted-runners",
            "--limit",
            "30",
        ])
        .env("GH_PROMPT_DISABLED", "1")
        .env("GH_NO_UPDATE_NOTIFIER", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

fn verifier_path(path: &std::ffi::OsStr) -> Result<PathBuf, ArtifactError> {
    let name = if cfg!(windows) { "gh.exe" } else { "gh" };
    for directory in std::env::split_paths(path).filter(|path| path.is_absolute()) {
        let candidate = directory.join(name);
        let Ok(metadata) = candidate.metadata() else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o111 == 0 {
                continue;
            }
        }
        return Ok(candidate.canonicalize()?);
    }
    Err(ArtifactError::Invalid(
        "install GitHub CLI on an absolute trusted PATH entry; current-directory lookup is disabled",
    ))
}

struct Verifier(Child);
impl Drop for Verifier {
    fn drop(&mut self) {
        // Reap the direct trusted verifier on errors/timeouts. It does not
        // execute downloaded files; arbitrary verifier descendants are outside
        // this contract, as are a compromised PATH and hostile same-user code.
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub(super) fn verify(path: &Path, manifest: &Manifest) -> Result<(), ArtifactError> {
    // Resolve an absolute PATH entry ourselves. In particular, Windows must
    // not implicitly execute a gh.exe from the current/download directory.
    let program = verifier_path(&std::env::var_os("PATH").unwrap_or_default())?;
    verify_command(command(&program, path, manifest), Duration::from_secs(90))
}

fn verify_command(mut command: Command, timeout: Duration) -> Result<(), ArtifactError> {
    let mut verifier = Verifier(command.spawn().map_err(|_| ArtifactError::Invalid(
        "cannot start GitHub CLI; install gh with attestation verification support and configure its access"))?);
    let start = Instant::now();
    loop {
        if let Some(status) = verifier.0.try_wait()? {
            return if status.success() {
                Ok(())
            } else {
                Err(ArtifactError::Invalid(
                    "GitHub attestation verification failed; check gh access, supported policy flags and official release provenance",
                ))
            };
        }
        if start.elapsed() >= timeout {
            return Err(ArtifactError::Invalid(
                "GitHub attestation verification timed out",
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn certificate_policy_pins_all_trust_boundaries() {
        let manifest = super::super::tests::fixture_manifest();
        let command = command(
            Path::new("gh"),
            Path::new("/private/manifest.json"),
            &manifest,
        );
        let args: Vec<_> = command
            .get_args()
            .map(|v| v.to_string_lossy().into_owned())
            .collect();
        for pair in [
            ["--hostname", "github.com"],
            ["--repo", "Rullst/Rullst"],
            ["--source-ref", "refs/tags/v12.1.0"],
            ["--source-digest", &manifest.source_commit],
            ["--signer-digest", &manifest.source_commit],
            [
                "--cert-identity",
                "https://github.com/Rullst/Rullst/.github/workflows/release.yml@refs/tags/v12.1.0",
            ],
            ["--predicate-type", "https://slsa.dev/provenance/v1"],
            [
                "--cert-oidc-issuer",
                "https://token.actions.githubusercontent.com",
            ],
        ] {
            assert!(args.windows(2).any(|v| v == pair));
        }
        assert!(args.iter().any(|v| v == "--deny-self-hosted-runners"));
        assert!(!args.iter().any(|v| v == "--custom-trusted-root"));
        // gh makes these selectors mutually exclusive. The exact certificate
        // identity above binds both the workflow and tag more narrowly.
        for other in [
            "--signer-workflow",
            "--signer-repo",
            "--cert-identity-regex",
        ] {
            assert!(!args.iter().any(|v| v == other));
        }
    }

    #[test]
    fn verifier_resolution_never_uses_implicit_current_directory_lookup() {
        assert!(verifier_path(std::ffi::OsStr::new("")).is_err());
        assert!(verifier_path(std::ffi::OsStr::new(".")).is_err());
        let temp = tempfile::tempdir().unwrap();
        let path = temp
            .path()
            .join(if cfg!(windows) { "gh.exe" } else { "gh" });
        std::fs::write(&path, "fixture; never executed").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert!(verifier_path(temp.path().as_os_str()).is_err());
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        assert_eq!(
            verifier_path(temp.path().as_os_str()).unwrap(),
            path.canonicalize().unwrap()
        );
    }

    #[cfg(unix)]
    #[test]
    fn verifier_failure_and_timeout_are_rejections() {
        let mut rejected = Command::new("sh");
        rejected.args(["-c", "exit 1"]);
        assert!(verify_command(rejected, Duration::from_secs(1)).is_err());
        let mut blocked = Command::new("sh");
        blocked.args(["-c", "exec sleep 5"]);
        let start = Instant::now();
        assert!(verify_command(blocked, Duration::from_millis(50)).is_err());
        assert!(start.elapsed() < Duration::from_secs(2));
    }
}
