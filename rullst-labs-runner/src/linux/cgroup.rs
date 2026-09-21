use super::probe::{CPU, MEMORY, PIDS, read_text};
use rullst_labs::{LabError as Error, Reference};
use std::{
    io::Write,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub(super) struct Group {
    path: PathBuf,
    closed: bool,
}
impl Group {
    pub fn create(root: &Path, nonce: &Reference) -> Result<Self, Error> {
        validate_root(root)?;
        let name = group_name(nonce)?;
        let path = root.join(name);
        std::fs::create_dir(&path).map_err(|_| Error::Conflict)?;
        let group = Self {
            path,
            closed: false,
        };
        for (name, value) in [
            ("memory.max", MEMORY.to_string()),
            ("memory.swap.max", "0".into()),
            ("memory.oom.group", "1".into()),
            ("pids.max", PIDS.to_string()),
            ("cpu.max", CPU.into()),
            ("cgroup.max.depth", "0".into()),
            ("cgroup.max.descendants", "0".into()),
        ] {
            group.write(name, &value)?;
            if read_text(group.path.join(name), 64)?.trim() != value {
                return Err(Error::Unsupported);
            }
        }
        if !group.path.join("cgroup.kill").is_file() {
            return Err(Error::Unsupported);
        }
        Ok(group)
    }
    /// Reconcile only the controller's persisted nonce under its delegated root.
    /// The caller must already have fenced the corresponding job lease.
    pub fn recover(root: &Path, nonce: &Reference) -> Result<Option<Self>, Error> {
        validate_root(root)?;
        let path = root.join(group_name(nonce)?);
        match std::fs::symlink_metadata(&path) {
            Ok(meta) if meta.is_dir() && meta.uid() == rustix::process::getuid().as_raw() => {
                Ok(Some(Self {
                    path,
                    closed: false,
                }))
            }
            Ok(_) => Err(Error::Unsupported),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err(Error::Uncertain),
        }
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn attach(&self, pid: u32) -> Result<(), Error> {
        if pid == std::process::id() || pid <= 1 {
            return Err(Error::Configuration);
        }
        self.write("cgroup.procs", &pid.to_string())?;
        if !read_text(self.path.join("cgroup.procs"), 4096)?
            .lines()
            .any(|value| value == pid.to_string())
        {
            return Err(Error::Unsupported);
        }
        Ok(())
    }
    pub fn exhausted(&self) -> Result<bool, Error> {
        let memory = read_text(self.path.join("memory.events"), 4096)?;
        let pids = read_text(self.path.join("pids.events"), 4096)?;
        for (text, keys) in [
            (&memory, &["oom", "oom_kill", "oom_group_kill"][..]),
            (&pids, &["max"][..]),
        ] {
            for line in text.lines() {
                let (name, value) = line.split_once(' ').ok_or(Error::Unsupported)?;
                if keys.contains(&name) && value.parse::<u64>().map_err(|_| Error::Unsupported)? > 0
                {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
    pub fn close(mut self) -> Result<(), Error> {
        self.write("cgroup.kill", "1")?;
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let events = read_text(self.path.join("cgroup.events"), 4096)?;
            if events.lines().any(|line| line == "populated 0") {
                break;
            }
            if Instant::now() >= deadline {
                return Err(Error::Uncertain);
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        std::fs::remove_dir(&self.path).map_err(|_| Error::Uncertain)?;
        self.closed = true;
        Ok(())
    }
    fn write(&self, name: &str, value: &str) -> Result<(), Error> {
        std::fs::OpenOptions::new()
            .write(true)
            .open(self.path.join(name))
            .and_then(|mut file| file.write_all(value.as_bytes()))
            .map_err(|_| Error::Unsupported)
    }
}
impl Drop for Group {
    fn drop(&mut self) {
        if !self.closed {
            // Best-effort containment only. Only successful explicit close can
            // produce a confirmed teardown receipt; a retained group is recovery.
            let _ = self.write("cgroup.kill", "1");
            let _ = std::fs::remove_dir(&self.path);
        }
    }
}
pub(super) fn validate_root(path: &Path) -> Result<(), Error> {
    let canonical = path.canonicalize().map_err(|_| Error::Unsupported)?;
    if canonical != path
        || !canonical.starts_with("/sys/fs/cgroup")
        || canonical == Path::new("/sys/fs/cgroup")
        || rustix::process::getuid().is_root()
    {
        return Err(Error::Unsupported);
    }
    let meta = std::fs::symlink_metadata(path).map_err(|_| Error::Unsupported)?;
    if !meta.is_dir()
        || meta.uid() != rustix::process::getuid().as_raw()
        || rustix::fs::statfs(path)
            .map_err(|_| Error::Unsupported)?
            .f_type
            != libc::CGROUP2_SUPER_MAGIC
    {
        return Err(Error::Unsupported);
    }
    if !read_text(path.join("cgroup.procs"), 4096)?
        .trim()
        .is_empty()
        || read_text(path.join("cgroup.type"), 64)?.trim() != "domain"
    {
        return Err(Error::Unsupported);
    }
    let controllers = read_text(path.join("cgroup.subtree_control"), 1024)?;
    if ["memory", "pids", "cpu"]
        .iter()
        .any(|required| !controllers.split_whitespace().any(|v| v == *required))
    {
        return Err(Error::Unsupported);
    }
    let mut count = 0;
    for entry in std::fs::read_dir(path).map_err(|_| Error::Unsupported)? {
        if entry
            .map_err(|_| Error::Unsupported)?
            .file_type()
            .map_err(|_| Error::Unsupported)?
            .is_dir()
        {
            count += 1;
        }
        if count > 32 {
            return Err(Error::Capacity);
        }
    }
    Ok(())
}
fn group_name(nonce: &Reference) -> Result<String, Error> {
    if nonce.as_str().len() != 48
        || !nonce
            .as_str()
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::Configuration);
    }
    Ok(format!("rullst-labs-{}", nonce.as_str()))
}
