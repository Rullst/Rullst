use super::{
    cgroup::Group,
    config::LinuxConfig,
    probe::{self, Observation},
    transport,
};
use rullst_labs::{
    Clock, ContentHash, ExecutionFailure, ExecutionProfile, LabError as Error, Reference,
    WorkerOutput,
    sqlite::{LeaseStatus, LeasedJob, SqliteLabs},
};
use std::{process::Stdio, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
    time::{Instant, timeout, timeout_at},
};

pub(super) struct CompletedAttempt {
    pub output: WorkerOutput,
    pub observations: ContentHash,
}
/// Launches the inert controller-owned bootstrap with no inherited environment.
/// Source is withheld until namespace/filesystem/network/resource observations
/// have been checked. Every exit path kills/reaps the entire owned job group.
pub(super) async fn execute<C: Clock>(
    config: &LinuxConfig,
    store: &SqliteLabs<C>,
    job: &LeasedJob,
) -> Result<CompletedAttempt, Error> {
    let mut session = Session::start(config, &job.input().binding().nonce).await?;
    let deadline =
        Instant::now() + Duration::from_secs(u64::from(job.input().limits().wall_seconds()));
    let work = async {
        let observations = session.observe(config).await?;
        if store.lease_status(job).await? != LeaseStatus::Active {
            return Err(Error::Conflict);
        }
        transport::write_frame(&mut session.stdin, job.input()).await?;
        session
            .stdin
            .shutdown()
            .await
            .map_err(|_| Error::Protocol)?;
        let mut read = Box::pin(transport::read_frame::<WorkerOutput>(
            &mut session.stdout,
            20_480,
        ));
        let mut interval = tokio::time::interval(Duration::from_millis(200));
        let output = loop {
            tokio::select! {
                output=&mut read=>break output?,
                _=interval.tick()=>if store.lease_status(job).await?!=LeaseStatus::Active {return Err(Error::Conflict);},
            }
        };
        drop(read);
        output.validate()?;
        if output.binding != *job.input().binding() {
            return Err(Error::Protocol);
        }
        let mut extra = [0u8; 1];
        if session
            .stdout
            .read(&mut extra)
            .await
            .map_err(|_| Error::Protocol)?
            != 0
        {
            return Err(Error::Protocol);
        }
        if !session
            .child
            .wait()
            .await
            .map_err(|_| Error::Uncertain)?
            .success()
        {
            return Err(Error::Protocol);
        }
        Ok(CompletedAttempt {
            output,
            observations,
        })
    };
    let outcome = timeout_at(deadline, work)
        .await
        .map_err(|_| Error::Expired)
        .and_then(|v| v);
    let exhausted = session.group.as_ref().ok_or(Error::Uncertain)?.exhausted();
    session.close().await?;
    let mut result = outcome?;
    if exhausted? {
        result.output.outcome =
            rullst_labs::WorkerOutcome::Rejected(ExecutionFailure::ResourceLimit);
    }
    Ok(result)
}
/// Real no-source preflight. A simulation/configuration boolean cannot satisfy
/// this check; success requires a live worker's restrictions and verified teardown.
pub(super) async fn preflight(
    config: &LinuxConfig,
    nonce: &Reference,
) -> Result<ContentHash, Error> {
    let mut session = Session::start(config, nonce).await?;
    let result = timeout(Duration::from_secs(5), session.observe(config))
        .await
        .map_err(|_| Error::Unsupported)
        .and_then(|v| v);
    session.close().await?;
    result
}
struct Session {
    group: Option<Group>,
    child: tokio::process::Child,
    stdin: tokio::process::ChildStdin,
    stdout: tokio::process::ChildStdout,
    stderr: Option<tokio::task::JoinHandle<Result<(), Error>>>,
    host_namespaces: std::collections::BTreeMap<String, String>,
}
impl Session {
    async fn start(config: &LinuxConfig, nonce: &Reference) -> Result<Self, Error> {
        let group = Group::create(&config.cgroups, nonce)?;
        let host_namespaces = probe::namespaces()?;
        let mut child = Command::new(std::env::current_exe().map_err(|_| Error::Configuration)?)
            .arg("__bootstrap")
            .arg(&config.launcher)
            .arg(&config.rootfs)
            .arg(group.path())
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| Error::Unsupported)?;
        let pid = child.id().ok_or(Error::Unsupported)?;
        if let Err(error) = group.attach(pid) {
            let _ = child.kill().await;
            return Err(error);
        }
        let stdin = child.stdin.take().ok_or(Error::Unsupported)?;
        let stdout = child.stdout.take().ok_or(Error::Unsupported)?;
        let stderr = child.stderr.take().ok_or(Error::Unsupported)?;
        let stderr = tokio::spawn(async move {
            // Never forward raw worker stderr into controller logs.
            let mut bytes = zeroize::Zeroizing::new(Vec::new());
            stderr
                .take(8193)
                .read_to_end(&mut bytes)
                .await
                .map_err(|_| Error::Protocol)?;
            if bytes.len() > 8192 {
                return Err(Error::Capacity);
            }
            Ok(())
        });
        Ok(Self {
            group: Some(group),
            child,
            stdin,
            stdout,
            stderr: Some(stderr),
            host_namespaces,
        })
    }
    async fn observe(&mut self, config: &LinuxConfig) -> Result<ContentHash, Error> {
        use tokio::io::AsyncWriteExt;
        self.stdin
            .write_all(b"G")
            .await
            .map_err(|_| Error::Unsupported)?;
        self.stdin.flush().await.map_err(|_| Error::Unsupported)?;
        let observation: Observation = transport::read_frame(&mut self.stdout, 4096).await?;
        let ExecutionProfile::LinuxExperimental { tools, .. } = &config.profile else {
            return Err(Error::Unsupported);
        };
        if observation.namespaces.len() != self.host_namespaces.len()
            || self.host_namespaces.iter().any(|(name, host)| {
                observation
                    .namespaces
                    .get(name)
                    .is_none_or(|worker| worker == host)
            })
            || observation.syscall_policy != tools.syscall_policy
            || observation.filesystem_policy.as_ref() != Some(&tools.filesystem_policy)
            || observation.memory_max != probe::MEMORY
            || observation.swap_max != 0
            || observation.pids_max != probe::PIDS
            || observation.cpu_max != probe::CPU
            || !observation.compiler_version.starts_with("rustc 1.96.0 ")
        {
            return Err(Error::Unsupported);
        }
        Ok(ContentHash::of(
            &serde_json::to_vec(&observation).map_err(|_| Error::Protocol)?,
        ))
    }
    async fn close(mut self) -> Result<(), Error> {
        let _ = self.child.start_kill();
        let cleanup = self.group.take().ok_or(Error::Uncertain)?.close();
        let wait = timeout(Duration::from_secs(2), self.child.wait()).await;
        let stderr = self.stderr.take().ok_or(Error::Uncertain)?;
        let stderr = timeout(Duration::from_secs(2), stderr)
            .await
            .map_err(|_| Error::Uncertain)?
            .map_err(|_| Error::Uncertain)?;
        cleanup?;
        wait.map_err(|_| Error::Uncertain)?
            .map_err(|_| Error::Uncertain)?;
        stderr
    }
}
