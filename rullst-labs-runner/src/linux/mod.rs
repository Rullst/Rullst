mod bootstrap;
mod cgroup;
mod compiler;
mod config;
mod describe;
mod engine;
mod filesystem;
mod probe;
mod service;
mod structure;
mod supervisor;
mod syscalls;
mod transport;
mod worker;

pub(crate) fn run(arguments: &[String]) -> Result<(), rullst_labs::LabError> {
    use rullst_labs::LabError as Error;
    use std::path::Path;
    match arguments {
        [mode] if mode == "__worker" => worker::run(),
        [mode, parent] if mode == "__compiler" => {
            compiler::run(parent.parse().map_err(|_| Error::Configuration)?)
        }
        [mode, rootfs, launcher, cgroups, seed] if mode == "describe-profile" => describe::run(
            Path::new(rootfs),
            Path::new(launcher),
            Path::new(cgroups),
            Path::new(seed),
        ),
        [mode, launcher, rootfs, group] if mode == "__bootstrap" => {
            bootstrap::run(Path::new(launcher), Path::new(rootfs), Path::new(group))
        }
        [mode, path] if mode == "doctor" || mode == "run-once" => {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|_| Error::Unsupported)?;
            if mode == "doctor" {
                runtime.block_on(service::doctor(Path::new(path)))
            } else {
                runtime.block_on(service::run_once(Path::new(path)))
            }
        }
        _ => Err(Error::Configuration),
    }
}

#[cfg(test)]
mod tests;
