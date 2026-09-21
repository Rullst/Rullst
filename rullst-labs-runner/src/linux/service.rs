use super::{cgroup::Group, config::LinuxConfig, probe, supervisor};
use ring::rand::{SecureRandom, SystemRandom};
use rullst_labs::{
    Clock, ContentHash, ExecutionFailure, ExecutionProfile, ExecutionReceipt, LabError as Error,
    ReceiptSigner, Reference, SystemClock, Teardown, WorkerOutcome, WorkerOutput,
    sqlite::{CleanupJob, ContentKey, SqliteLabs, StoreConfig},
};
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlaneConfig {
    database: PathBuf,
    namespace: Reference,
    max_jobs: u32,
    max_exercises: u32,
    content_key: PathBuf,
    receipt_seed: PathBuf,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RunnerConfig {
    linux: LinuxConfig,
    plane: PlaneConfig,
}
fn load(path: &Path) -> Result<RunnerConfig, Error> {
    let text = probe::read_text(path, 32768)?;
    serde_json::from_str(&text).map_err(|_| Error::Configuration)
}
pub(super) async fn doctor(path: &Path) -> Result<(), Error> {
    let config = load(path)?;
    config.linux.validate()?;
    let digest = supervisor::preflight(&config.linux, &nonce()?).await?;
    println!(
        "{}",
        serde_json::json!({"profile":rullst_labs::PROFILE,"status":"experimental-preflight-passed","observations":digest})
    );
    Ok(())
}
pub(super) async fn run_once(path: &Path) -> Result<(), Error> {
    let config = load(path)?;
    config.linux.validate()?;
    // Never consume queued submissions on an unsupported machine. This real
    // probe contains no student source, grade key or application credentials.
    supervisor::preflight(&config.linux, &nonce()?).await?;
    let content_key = read_key(&config.plane.content_key)?;
    let seed = read_key(&config.plane.receipt_seed)?;
    if content_key.as_ref() == seed.as_ref() {
        return Err(Error::Configuration);
    }
    let signer = ReceiptSigner::from_seed(seed)?;
    let ExecutionProfile::LinuxExperimental { receipt_key, .. } = &config.linux.profile else {
        return Err(Error::Unsupported);
    };
    if &signer.public_key()? != receipt_key {
        return Err(Error::Integrity);
    }
    let store = SqliteLabs::open(
        &config.plane.database,
        StoreConfig::new(
            config.plane.namespace,
            config.plane.max_jobs,
            config.plane.max_exercises,
            config.linux.profile.clone(),
        )?,
        ContentKey::new(*content_key)?,
        SystemClock,
    )
    .await?;
    let result = process(&config.linux, &store, &signer).await;
    store.close().await;
    result
}
async fn process(
    config: &LinuxConfig,
    store: &SqliteLabs,
    signer: &ReceiptSigner,
) -> Result<(), Error> {
    for cleanup in store.cleanup_candidates(32).await? {
        reconcile(config, store, signer, &cleanup).await?;
    }
    let Some(job) = store.claim_next().await? else {
        println!("{{\"status\":\"idle\"}}");
        return Ok(());
    };
    let started_at = SystemClock.now()?;
    let result = match supervisor::execute(config, store, &job).await {
        Ok(completed) => {
            let signed = signer.sign(ExecutionReceipt {
                output: completed.output,
                started_at,
                finished_at: SystemClock.now()?,
                observation_digest: completed.observations,
                teardown: Teardown::Confirmed,
            })?;
            store.complete(job.scope(), job.id(), &signed).await
        }
        Err(error) => Err(error),
    };
    match result {
        Ok(view) => {
            println!(
                "{}",
                serde_json::json!({"status":"job-finished","state":view.state,"revision":view.revision})
            );
            Ok(())
        }
        Err(error) => {
            // Fence first, then independently confirm group absence/termination.
            // Never convert a failed response, timeout or lost process to success.
            let cleanup = store.abandon_attempt(&job).await?;
            reconcile(config, store, signer, &cleanup).await?;
            Err(error)
        }
    }
}
async fn reconcile(
    config: &LinuxConfig,
    store: &SqliteLabs,
    signer: &ReceiptSigner,
    job: &CleanupJob,
) -> Result<(), Error> {
    let started_at = SystemClock.now()?;
    if let Some(group) = Group::recover(&config.cgroups, &job.binding.nonce)? {
        group.close()?;
    }
    let receipt = signer.sign(ExecutionReceipt {
        output: WorkerOutput {
            binding: job.binding.clone(),
            outcome: WorkerOutcome::Rejected(ExecutionFailure::WorkerLost),
        },
        started_at,
        finished_at: SystemClock.now()?,
        observation_digest: ContentHash::of(b"RullstLabsConfirmedGroupAbsence-v1"),
        teardown: Teardown::Confirmed,
    })?;
    // The first CLI intentionally does not automatically re-execute abandoned
    // code. The dedicated API allows one deliberate retry after real teardown.
    store
        .reconcile_cleanup(&job.scope, &job.id, &receipt, false)
        .await?;
    Ok(())
}
pub(super) fn read_key(path: &Path) -> Result<zeroize::Zeroizing<[u8; 32]>, Error> {
    let meta = std::fs::symlink_metadata(path).map_err(|_| Error::Configuration)?;
    if !meta.is_file()
        || meta.uid() != rustix::process::getuid().as_raw()
        || meta.mode() & 0o7077 != 0
        || meta.len() != 32
    {
        return Err(Error::Configuration);
    }
    let mut key = zeroize::Zeroizing::new([0u8; 32]);
    std::fs::File::open(path)
        .and_then(|mut file| file.read_exact(key.as_mut()))
        .map_err(|_| Error::Configuration)?;
    Ok(key)
}
pub(super) fn nonce() -> Result<Reference, Error> {
    let mut bytes = [0u8; 24];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| Error::Unsupported)?;
    Reference::new(hex::encode(bytes))
}
