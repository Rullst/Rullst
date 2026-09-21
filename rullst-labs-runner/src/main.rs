//! Separately deployed runner candidate. Unsupported profiles fail closed.
#![forbid(unsafe_code)]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod linux;
fn main() -> std::process::ExitCode {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    let result = std::env::args_os()
        .skip(1)
        .take(16)
        .map(|value| {
            value
                .into_string()
                .map_err(|_| rullst_labs::LabError::Configuration)
        })
        .collect::<Result<Vec<_>, _>>()
        .and_then(|args| linux::run(&args));
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    let result: Result<(), rullst_labs::LabError> = Err(rullst_labs::LabError::Unsupported);
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}
