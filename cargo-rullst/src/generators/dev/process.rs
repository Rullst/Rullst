use crate::ui::dash_tui::LogMsg;
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
mod group;
use group::ProcessGroup;

pub(super) struct Application {
    child: Option<Child>,
    executable: PathBuf,
    generation: String,
    group: Option<ProcessGroup>,
    exit_status: Option<ExitStatus>,
}

impl Application {
    pub(super) fn prepare(source: &Path) -> io::Result<Self> {
        let source = source.canonicalize()?;
        let directory = source
            .parent()
            .ok_or_else(|| io::Error::other("executable has no parent"))?;
        let generation = uuid::Uuid::new_v4().simple().to_string();
        let executable = directory.join(format!(
            "rullst-dev-{generation}{}",
            std::env::consts::EXE_SUFFIX
        ));
        // create_new avoids overwriting an unrelated path, including a preexisting symlink.
        let mut original = fs::File::open(&source)?;
        let mut copy = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&executable)?;
        let result = io::copy(&mut original, &mut copy)
            .and_then(|_| fs::set_permissions(&executable, original.metadata()?.permissions()));
        if let Err(error) = result {
            drop(copy);
            let _ = fs::remove_file(&executable);
            return Err(error);
        }
        Ok(Self {
            child: None,
            executable,
            generation,
            group: None,
            exit_status: None,
        })
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.executable);
        command
            .env_remove("HOT_RELOAD")
            .env_remove("RULLST_HMR_TOKEN")
            .env("RULLST_DEV_GENERATION", &self.generation);
        configure_group(&mut command);
        command
    }

    pub(super) fn start(&mut self, dashboard: bool, logs: &mpsc::Sender<LogMsg>) -> io::Result<()> {
        if self.child.is_some() {
            return Err(io::Error::other("owned application is already started"));
        }
        // Readiness identifies a process launch, not a reusable binary file.
        self.generation = uuid::Uuid::new_v4().simple().to_string();
        let mut command = self.command();
        if dashboard {
            command.stdout(Stdio::piped()).stderr(Stdio::piped());
        }
        let mut child = command.spawn()?;
        if dashboard {
            if let Some(stdout) = child.stdout.take() {
                forward(stdout, logs.clone(), false);
            }
            if let Some(stderr) = child.stderr.take() {
                forward(stderr, logs.clone(), true);
            }
        }
        self.group = Some(ProcessGroup::new(child.id()));
        self.exit_status = None;
        self.child = Some(child);
        Ok(())
    }

    pub(super) async fn migrate(
        &self,
        dashboard: bool,
        logs: &mpsc::Sender<LogMsg>,
    ) -> io::Result<()> {
        let mut command = self.command();
        command.arg("db:migrate");
        if dashboard {
            command.stdout(Stdio::piped()).stderr(Stdio::piped());
        }
        let mut command = tokio::process::Command::from(command);
        command.kill_on_drop(true);
        let mut migration = BuildChild::new(command.spawn()?)?;
        if dashboard {
            // Forward using bounded asynchronous reads; no unbounded output() capture.
            let stdout = migration
                .child
                .stdout
                .take()
                .ok_or_else(|| io::Error::other("migration stdout missing"))?;
            let stderr = migration
                .child
                .stderr
                .take()
                .ok_or_else(|| io::Error::other("migration stderr missing"))?;
            let (a, b, status) = tokio::join!(
                forward_async(stdout, logs, false),
                forward_async(stderr, logs, true),
                migration.wait()
            );
            a?;
            b?;
            if !status?.success() {
                return Err(io::Error::other(
                    "database migration failed; inspect the application logs",
                ));
            }
        } else if !migration.wait().await?.success() {
            return Err(io::Error::other("database migration failed"));
        }
        Ok(())
    }

    pub(super) fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        if self.exit_status.is_some() {
            return Ok(self.exit_status);
        }
        let (Some(child), Some(group)) = (&mut self.child, &mut self.group) else {
            return Ok(None);
        };
        #[cfg(unix)]
        {
            if !group.exit_observed()? {
                return Ok(None);
            }
            group.finish();
        }
        self.exit_status = child.try_wait()?;
        if self.exit_status.is_some() {
            group.disarm();
        }
        Ok(self.exit_status)
    }

    pub(super) async fn wait_ready(&mut self, port: u16) -> io::Result<()> {
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_millis(750))
            .build()
            .map_err(io::Error::other)?;
        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            if self.try_wait()?.is_some() {
                return Err(io::Error::other("new process exited during startup"));
            }
            if let Ok(response) = client
                .get(format!("http://127.0.0.1:{port}/_rullst/dev-generation"))
                .send()
                .await
                && response.status().is_success()
                && response
                    .headers()
                    .get("x-rullst-dev-generation")
                    .and_then(|value| value.to_str().ok())
                    == Some(self.generation.as_str())
            {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(150)).await;
        }
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "matching application generation did not become ready within 15 seconds",
        ))
    }

    pub(super) fn stop(&mut self) -> io::Result<()> {
        if let Some(mut child) = self.child.take() {
            if self.exit_status.is_none()
                && let Some(mut group) = self.group.take()
            {
                group.terminate(false);
                let deadline = Instant::now() + Duration::from_secs(2);
                #[cfg(unix)]
                while !group.exit_observed().unwrap_or(false) && Instant::now() < deadline {
                    std::thread::sleep(Duration::from_millis(20));
                }
                #[cfg(not(unix))]
                while child.try_wait()?.is_none() && Instant::now() < deadline {
                    std::thread::sleep(Duration::from_millis(20));
                }
                // Unix leader has not been reaped: even a parent-first exit
                // cannot allow PID reuse before leftover descendants are killed.
                #[cfg(not(unix))]
                if child.try_wait()?.is_some() {
                    group.disarm();
                }
                group.finish();
                if child.try_wait()?.is_none() {
                    let _ = child.kill();
                }
            }
            child.wait()?;
        }
        self.group = None;
        self.exit_status = None;
        Ok(())
    }
}

impl Drop for Application {
    fn drop(&mut self) {
        let _ = self.stop();
        let _ = fs::remove_file(&self.executable);
    }
}

pub(super) struct BuildChild {
    pub(super) child: tokio::process::Child,
    group: ProcessGroup,
}

impl BuildChild {
    pub(super) fn new(child: tokio::process::Child) -> io::Result<Self> {
        let id = child
            .id()
            .ok_or_else(|| io::Error::other("spawned child has no process ID"))?;
        Ok(Self {
            child,
            group: ProcessGroup::new(id),
        })
    }

    pub(super) async fn wait(&mut self) -> io::Result<ExitStatus> {
        #[cfg(unix)]
        {
            while !self.group.exit_observed()? {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            self.group.finish();
        }
        let status = self.child.wait().await?;
        self.group.disarm();
        Ok(status)
    }
}

impl Drop for BuildChild {
    fn drop(&mut self) {
        if self.child.id().is_some() {
            self.group.finish();
            let _ = self.child.start_kill();
            let deadline = Instant::now() + Duration::from_secs(2);
            while matches!(self.child.try_wait(), Ok(None)) && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
}

pub(super) fn configure_group(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0000_0200); // CREATE_NEW_PROCESS_GROUP
    }
}

fn forward(mut stream: impl Read + Send + 'static, logs: mpsc::Sender<LogMsg>, error: bool) {
    std::thread::spawn(move || {
        let mut buffer = [0; 4096];
        while let Ok(count) = stream.read(&mut buffer) {
            if count == 0 {
                break;
            }
            send_chunk(&buffer[..count], &logs, error);
        }
    });
}

async fn forward_async(
    mut stream: impl tokio::io::AsyncRead + Unpin,
    logs: &mpsc::Sender<LogMsg>,
    error: bool,
) -> io::Result<()> {
    use tokio::io::AsyncReadExt;
    let mut buffer = [0; 4096];
    loop {
        let count = stream.read(&mut buffer).await?;
        if count == 0 {
            return Ok(());
        }
        send_chunk(&buffer[..count], logs, error);
    }
}

fn send_chunk(bytes: &[u8], logs: &mpsc::Sender<LogMsg>, error: bool) {
    for line in String::from_utf8_lossy(bytes).lines() {
        let message = if error {
            LogMsg::AppStderr(line.into())
        } else {
            LogMsg::AppStdout(line.into())
        };
        let _ = logs.try_send(message);
    }
}

#[cfg(test)]
mod tests;
