use rullst_labs::{LabError as Error, MAX_DIAGNOSTIC_BYTES};
use std::{
    io::{Read, Write},
    os::{fd::AsRawFd, unix::process::CommandExt},
    process::{Child, ChildStdin, ChildStdout, Command, ExitStatus, Stdio},
};

const READY: &[u8] = b"RullstLabsCompilerReady-v1\n";

/// An inert compiler peer with its own inherited OS boundary and independent
/// Landlock domain. It waits for source preparation and a fixed memory bound.
pub(super) struct Prepared {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: ChildStdout,
}
impl Prepared {
    pub(super) fn spawn() -> Result<Self, Error> {
        let mut child = Command::new("/runner")
            .args(["__compiler", &std::process::id().to_string()])
            .env_clear()
            .envs([
                ("PATH", "/toolchain/bin"),
                ("LANG", "C"),
                ("LC_ALL", "C"),
                ("TMPDIR", "/work"),
                ("LD_LIBRARY_PATH", "/toolchain/lib:/lib"),
                ("RAYON_NUM_THREADS", "1"),
                ("PWD", "/work"),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| Error::Unsupported)?;
        let stdin = Some(child.stdin.take().ok_or(Error::Protocol)?);
        let stdout = child.stdout.take().ok_or(Error::Protocol)?;
        Ok(Self {
            child,
            stdin,
            stdout,
        })
    }
    pub(super) fn control_fds(&self) -> Result<[i32; 3], Error> {
        Ok([
            self.stdin.as_ref().ok_or(Error::Protocol)?.as_raw_fd(),
            self.stdout.as_raw_fd(),
            self.child
                .stderr
                .as_ref()
                .ok_or(Error::Protocol)?
                .as_raw_fd(),
        ])
    }
    pub(super) fn ready(&mut self) -> Result<(), Error> {
        let mut bytes = [0u8; READY.len()];
        self.stdout
            .read_exact(&mut bytes)
            .map_err(|_| Error::Unsupported)?;
        if bytes != READY {
            return Err(Error::Unsupported);
        }
        Ok(())
    }
    pub(super) fn finish(mut self, memory_pages: u32) -> Result<(ExitStatus, Vec<u8>), Error> {
        let mut input = self.stdin.take().ok_or(Error::Protocol)?;
        input
            .write_all(&memory_pages.to_be_bytes())
            .map_err(|_| Error::Protocol)?;
        drop(input);
        let stderr = self.child.stderr.take().ok_or(Error::Protocol)?;
        let reader = std::thread::Builder::new()
            .name("compiler-diagnostics".into())
            .spawn(move || {
                let mut bytes = Vec::new();
                stderr
                    .take(MAX_DIAGNOSTIC_BYTES as u64 + 1)
                    .read_to_end(&mut bytes)
                    .map(|_| bytes)
            })
            .map_err(|_| Error::Unsupported)?;
        let status = self.child.wait().map_err(|_| Error::Uncertain)?;
        let diagnostics = reader
            .join()
            .map_err(|_| Error::Uncertain)?
            .map_err(|_| Error::Protocol)?;
        let mut extra = [0u8; 1];
        if self.stdout.read(&mut extra).map_err(|_| Error::Protocol)? != 0 {
            return Err(Error::Protocol);
        }
        Ok((status, diagnostics))
    }
}

pub(super) fn run(parent: u32) -> Result<(), Error> {
    super::worker::restrict_resources()?;
    super::syscalls::apply()?;
    super::probe::inspect(&[], false)?;
    super::filesystem::enforce(false)?;
    super::filesystem::deny_parent_access(parent)?;
    std::io::stdout()
        .write_all(READY)
        .map_err(|_| Error::Protocol)?;
    std::io::stdout().flush().map_err(|_| Error::Protocol)?;
    // All checks precede this fixed-size compiler-start message. No dynamic
    // command, path, environment, source bytes or compiler flags are accepted.
    let mut request = [0u8; 4];
    std::io::stdin()
        .read_exact(&mut request)
        .map_err(|_| Error::Protocol)?;
    let pages = u32::from_be_bytes(request);
    if !(32..=256).contains(&pages) {
        return Err(Error::InvalidInput);
    }
    let maximum = u64::from(pages) * 65_536;
    let mut command = Command::new("/toolchain/bin/rustc");
    command.env_clear()
        .env("PATH","/toolchain/bin").env("LD_LIBRARY_PATH","/toolchain/lib:/lib").env("TMPDIR","/work").env("LANG","C").env("RAYON_NUM_THREADS","1")
        .args(["/work/submission.rs","--crate-name","submission","--crate-type","cdylib","--edition","2024","--target","wasm32-unknown-unknown","--sysroot","/toolchain","--color","never","--error-format","short","-C","opt-level=1","-C","panic=abort","-C","debuginfo=0","-C","strip=symbols","-C","codegen-units=1","-C","overflow-checks=on","-C","target-feature=-simd128,-relaxed-simd,-multivalue,-reference-types,-tail-call,-extended-const","-C","link-arg=-zstack-size=1048576",
            "-C",
            "link-arg=--threads=1","-C"])
        .arg(format!("link-arg=--max-memory={maximum}"))
        .args(["-o","/work/submission.wasm"])
        .stdin(Stdio::null()).stdout(Stdio::null());
    // Replaces the trusted helper only after its independent domain is verified.
    // stdout cannot become a parent result channel; diagnostics are bounded.
    let _error = command.exec();
    Err(Error::Unsupported)
}
