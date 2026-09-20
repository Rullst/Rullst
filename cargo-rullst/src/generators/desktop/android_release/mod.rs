//! Build one fresh APK and bind its verified signature to an application cert.
mod files;
mod process;
mod report;

use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime},
};

const SIGNING_ENV: [&str; 4] = [
    "RULLST_ANDROID_KEYSTORE",
    "RULLST_ANDROID_KEY_ALIAS",
    "RULLST_ANDROID_STORE_PASSWORD",
    "RULLST_ANDROID_KEY_PASSWORD",
];

#[derive(thiserror::Error)]
enum ReleaseError {
    #[error(
        "Android release verification requires an absolute DER certificate and trusted SDK apksigner.jar; set RULLST_ANDROID_SIGNING_CERTIFICATE and RULLST_ANDROID_APKSIGNER_JAR or their CLI options"
    )]
    Configuration,
    #[error(
        "Android release artifact is missing, linked, changed, outside its supported layout or exceeds the file/inventory bounds"
    )]
    Artifact,
    #[error(
        "expected exactly one fresh release APK; select --apk relative to gen/android/app/build/outputs/apk and move unchanged prior output aside before rebuilding"
    )]
    Selection,
    #[error(
        "Android build or signature verifier failed; inspect the reviewed local tool configuration; captured output is withheld because build tools receive signing secrets"
    )]
    Tool,
    #[error("Android tool output exceeded its bound")]
    Output,
    #[error("Android tool deadline exceeded")]
    Timeout,
    #[error("Android release verification cancelled")]
    Cancelled,
    #[error(
        "Android signing certificate does not match the expected single signer, or the verifier report is unsupported"
    )]
    Report,
    #[error("cannot access Android release inputs or outputs")]
    Io,
}

impl std::fmt::Debug for ReleaseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, formatter)
    }
}

impl From<std::io::Error> for ReleaseError {
    fn from(_: std::io::Error) -> Self {
        Self::Io
    }
}

/// Public verification inputs. The application retains custody of signing keys.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct AndroidReleaseOptions {
    certificate: PathBuf,
    apksigner_jar: PathBuf,
    apk: Option<PathBuf>,
    architecture: Option<String>,
}

impl AndroidReleaseOptions {
    pub fn new(certificate: impl Into<PathBuf>, apksigner_jar: impl Into<PathBuf>) -> Self {
        Self {
            certificate: certificate.into(),
            apksigner_jar: apksigner_jar.into(),
            apk: None,
            architecture: None,
        }
    }

    /// Select a release APK relative to gen/android/app/build/outputs/apk.
    #[must_use]
    pub fn apk(mut self, path: impl Into<PathBuf>) -> Self {
        self.apk = Some(path.into());
        self
    }

    /// Restrict Tauri's native build to aarch64, armv7, i686 or x86_64.
    #[must_use]
    pub fn architecture(mut self, value: impl Into<String>) -> Self {
        self.architecture = Some(value.into());
        self
    }

    fn environment() -> Result<Self, ReleaseError> {
        Ok(Self::new(
            std::env::var_os("RULLST_ANDROID_SIGNING_CERTIFICATE")
                .ok_or(ReleaseError::Configuration)?,
            std::env::var_os("RULLST_ANDROID_APKSIGNER_JAR").ok_or(ReleaseError::Configuration)?,
        ))
    }
}

/// Evidence for these exact local APK bytes, not store or device acceptance.
#[derive(Debug, serde::Serialize)]
#[non_exhaustive]
pub struct AndroidReleaseEvidence {
    pub schema_version: &'static str,
    pub apk: PathBuf,
    pub bytes: u64,
    pub apk_sha256: String,
    pub certificate_sha256: String,
}

pub(crate) fn command(command: clap::Command) -> clap::Command {
    command
        .arg(clap::Arg::new("release").long("release").action(clap::ArgAction::SetTrue)
            .help("Build and verify a fresh Android release APK against your certificate"))
        .args([
            clap::Arg::new("signing-certificate").long("signing-certificate").value_parser(clap::value_parser!(PathBuf))
                .help("Absolute application-owned public DER certificate (or RULLST_ANDROID_SIGNING_CERTIFICATE)"),
            clap::Arg::new("apksigner-jar").long("apksigner-jar").value_parser(clap::value_parser!(PathBuf))
                .help("Absolute trusted Android SDK lib/apksigner.jar (or RULLST_ANDROID_APKSIGNER_JAR)"),
            clap::Arg::new("apk").long("apk").value_parser(clap::value_parser!(PathBuf))
                .help("Expected relative APK within gen/android/app/build/outputs/apk; otherwise require one fresh output"),
            clap::Arg::new("android-arch").long("android-arch")
                .value_parser(["aarch64", "armv7", "i686", "x86_64"])
                .help("Restrict the build to one native architecture"),
        ].map(|argument| argument.requires("release")))
}

pub(crate) fn run(matches: &clap::ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    // Fail signing input validation before any build, verifier or output scan.
    super::signing::validate()?;
    let value = |name: &str, environment: &str| {
        matches
            .get_one::<PathBuf>(name)
            .cloned()
            .or_else(|| std::env::var_os(environment).map(PathBuf::from))
            .ok_or(ReleaseError::Configuration)
    };
    let mut options = AndroidReleaseOptions::new(
        value("signing-certificate", "RULLST_ANDROID_SIGNING_CERTIFICATE")?,
        value("apksigner-jar", "RULLST_ANDROID_APKSIGNER_JAR")?,
    );
    options.apk = matches.get_one::<PathBuf>("apk").cloned();
    options.architecture = matches.get_one::<String>("android-arch").cloned();
    print(build_android_release_with_options(&options)?)
}

fn print(evidence: AndroidReleaseEvidence) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", serde_json::to_string_pretty(&evidence)?);
    Ok(())
}

/// Build using the four existing signing variables and the two verification paths.
pub fn build_android_release() -> Result<(), Box<dyn std::error::Error>> {
    super::signing::validate()?;
    print(build_android_release_with_options(
        &AndroidReleaseOptions::environment()?,
    )?)
}

/// Build trusted local application code and verify exactly one fresh signed APK.
/// This does not publish, install on a device, or certify a signing toolchain.
pub fn build_android_release_with_options(
    options: &AndroidReleaseOptions,
) -> Result<AndroidReleaseEvidence, Box<dyn std::error::Error>> {
    super::signing::validate()?;
    Ok(build(options)?)
}

fn build(options: &AndroidReleaseOptions) -> Result<AndroidReleaseEvidence, ReleaseError> {
    if !options.certificate.is_absolute()
        || !options.apksigner_jar.is_absolute()
        || options.apksigner_jar.extension().is_none_or(|e| e != "jar")
        || !options.apksigner_jar.is_file()
        || options
            .architecture
            .as_deref()
            .is_some_and(|a| !["aarch64", "armv7", "i686", "x86_64"].contains(&a))
    {
        return Err(ReleaseError::Configuration);
    }
    if let Some(path) = &options.apk {
        files::relative(path)?;
    }
    let certificate_sha256 = hex::encode(Sha256::digest(files::bounded_file(
        &options.certificate,
        64 * 1024,
    )?));
    let jar = options.apksigner_jar.canonicalize()?;
    let java = process::tool("java")?;
    let root = Path::new("omni-app").canonicalize()?;
    let before = files::inventory(&root)?;
    let mut command = build_command(&root)?;
    command.args(["android", "build", "--apk", "--ci"]);
    if let Some(architecture) = &options.architecture {
        command.args(["--target", architecture]);
    }
    let start = SystemTime::now();
    process::capture(command, Duration::from_secs(2700), 16 * 1024 * 1024)?;
    let after = files::inventory(&root)?;
    let relative = files::select(&before, &after, options.apk.as_deref(), start)?;
    let selected = after.get(&relative).ok_or(ReleaseError::Selection)?;
    let apk = root
        .join("gen/android/app/build/outputs/apk")
        .join(relative);
    let temporary = tempfile::Builder::new()
        .prefix("rullst-apk-verification-")
        .tempdir()?;
    let snapshot = temporary.path().join("release.apk");
    files::snapshot(&apk, selected, &snapshot)?;
    let mut verify = Command::new(java);
    verify
        .arg("-jar")
        .arg(jar)
        .args(["verify", "--verbose", "--print-certs", "-Werr"])
        .arg(&snapshot);
    for variable in
        SIGNING_ENV
            .into_iter()
            .chain(["JAVA_TOOL_OPTIONS", "JDK_JAVA_OPTIONS", "_JAVA_OPTIONS"])
    {
        verify.env_remove(variable);
    }
    let output = process::capture(verify, Duration::from_secs(90), 64 * 1024)?;
    report::verify(&output, &certificate_sha256)?;
    if files::stamp(&snapshot)?.digest != selected.digest || files::stamp(&apk)? != *selected {
        return Err(ReleaseError::Artifact);
    }
    Ok(AndroidReleaseEvidence {
        schema_version: "rullst.android-release.v1",
        apk,
        bytes: selected.bytes,
        apk_sha256: selected.digest.clone(),
        certificate_sha256,
    })
}

fn build_command(root: &Path) -> Result<Command, ReleaseError> {
    if root
        .join("node_modules/@tauri-apps/cli/package.json")
        .is_file()
    {
        let name = if cfg!(windows) { "npm.cmd" } else { "npm" };
        // npm uses a batch entry on Windows; arguments below are fixed, and the
        // only appended value is the closed architecture allowlist.
        let path = std::env::var_os("PATH").unwrap_or_default();
        let executable = std::env::split_paths(&path)
            .filter(|p| p.is_absolute())
            .map(|p| p.join(name))
            .find(|p| p.is_file())
            .ok_or(ReleaseError::Tool)?;
        let mut command = Command::new(executable);
        command
            .args(["exec", "--offline", "--", "tauri"])
            .current_dir(root);
        return Ok(command);
    }
    let cargo = process::tool("cargo")?;
    let mut probe = Command::new(&cargo);
    probe.args(["tauri", "--version"]).current_dir(root);
    process::capture(probe, Duration::from_secs(30), 64 * 1024)?;
    let mut command = Command::new(cargo);
    command.arg("tauri").current_dir(root);
    Ok(command)
}
