use rullst_labs::{
    Diagnostic, ExecutionFailure, LabError as Error, MAX_DIAGNOSTIC_BYTES, MAX_WASM_BYTES,
    WorkerInput, WorkerOutcome, WorkerOutput,
};
use rustix::process::{Resource, Rlimit};
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
};

pub(super) fn run() -> Result<(), Error> {
    for (resource, maximum) in [
        (Resource::Core, 0),
        (Resource::Fsize, 33_554_432),
        (Resource::Nofile, 64),
        (Resource::Memlock, 0),
        (Resource::Stack, 8_388_608),
        (Resource::Cpu, 65),
    ] {
        let limit = Rlimit {
            current: Some(maximum),
            maximum: Some(maximum),
        };
        rustix::process::setrlimit(resource, limit).map_err(|_| Error::Unsupported)?;
        if rustix::process::getrlimit(resource) != limit {
            return Err(Error::Unsupported);
        }
    }
    super::syscalls::apply().inspect_err(|_| eprintln!("labs-preflight:seccomp"))?;
    let mut observation = super::probe::inspect()?;
    observation.filesystem_policy =
        Some(super::filesystem::enforce().inspect_err(|_| eprintln!("labs-preflight:landlock"))?);
    write_frame(&observation)?;
    // Parent verifies actual observations and cgroup placement before releasing
    // any student-controlled bytes. An EOF/invalid frame stops this worker.
    let bytes = read_frame(131_072)?;
    let input = WorkerInput::from_bytes(&bytes)?;
    let outcome = compile_and_evaluate(&input)?;
    write_frame(&WorkerOutput {
        binding: input.binding().clone(),
        outcome,
    })
}
fn compile_and_evaluate(input: &WorkerInput) -> Result<WorkerOutcome, Error> {
    let mut source = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open("/work/submission.rs")
        .map_err(|_| Error::Storage)?;
    source.write_all(b"mod learner {\n").and_then(|_|source.write_all(input.source().expose_source().as_bytes())).and_then(|_|source.write_all(b"\n}\n#[unsafe(no_mangle)] pub extern \"C\" fn solve(a:i64,b:i64)->i64 { learner::solve(a,b) }\n")).map_err(|_|Error::Storage)?;
    drop(source);
    let maximum = u64::from(input.limits().memory_pages()) * 65_536;
    let mut child=Command::new("/toolchain/bin/rustc").env_clear()
        .env("PATH","/toolchain/bin").env("LD_LIBRARY_PATH","/toolchain/lib:/lib").env("TMPDIR","/work").env("LANG","C").env("RAYON_NUM_THREADS","1")
        .args(["/work/submission.rs","--crate-name","submission","--crate-type","cdylib","--edition","2024","--target","wasm32-unknown-unknown","--sysroot","/toolchain","--color","never","--error-format","short","-C","opt-level=1","-C","panic=abort","-C","debuginfo=0","-C","strip=symbols","-C","codegen-units=1","-C","overflow-checks=on","-C","target-feature=-simd128,-relaxed-simd,-multivalue,-reference-types,-tail-call,-extended-const","-C","link-arg=-zstack-size=1048576",
            "-C",
            "link-arg=--threads=1","-C"])
        .arg(format!("link-arg=--max-memory={maximum}"))
        .args(["-o","/work/submission.wasm"])
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped()).spawn().map_err(|_|Error::Unsupported)?;
    let stderr = child.stderr.take().ok_or(Error::Unsupported)?;
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
    let status = child.wait().map_err(|_| Error::Uncertain)?;
    let diagnostics = reader
        .join()
        .map_err(|_| Error::Uncertain)?
        .map_err(|_| Error::Protocol)?;
    if diagnostics.len() > MAX_DIAGNOSTIC_BYTES {
        return Ok(WorkerOutcome::Rejected(ExecutionFailure::ResourceLimit));
    }
    if !status.success() {
        let text = String::from_utf8_lossy(&diagnostics)
            .chars()
            .map(|c| {
                if c.is_control() && !matches!(c, '\n' | '\t') {
                    '?'
                } else {
                    c
                }
            })
            .collect::<String>()
            .replace("/work/submission.rs", "submission.rs");
        return Ok(match Diagnostic::new(text) {
            Ok(diagnostics) => WorkerOutcome::CompileRejected { diagnostics },
            Err(_) => WorkerOutcome::Rejected(ExecutionFailure::Compile),
        });
    }
    let file = std::fs::File::open("/work/submission.wasm").map_err(|_| Error::Protocol)?;
    if !file.metadata().map_err(|_| Error::Protocol)?.is_file() {
        return Err(Error::Protocol);
    }
    let mut wasm = Vec::new();
    file.take(MAX_WASM_BYTES as u64 + 1)
        .read_to_end(&mut wasm)
        .map_err(|_| Error::Protocol)?;
    Ok(super::engine::evaluate(
        &wasm,
        input.inputs(),
        input.limits(),
    ))
}
fn read_frame(max: usize) -> Result<zeroize::Zeroizing<Vec<u8>>, Error> {
    let stdin = std::io::stdin();
    let mut stream = stdin.lock();
    let mut length = [0u8; 4];
    stream
        .read_exact(&mut length)
        .map_err(|_| Error::Protocol)?;
    let length = usize::try_from(u32::from_be_bytes(length)).map_err(|_| Error::Capacity)?;
    if length == 0 || length > max {
        return Err(Error::Capacity);
    }
    let mut bytes = zeroize::Zeroizing::new(vec![0; length]);
    stream.read_exact(&mut bytes).map_err(|_| Error::Protocol)?;
    Ok(bytes)
}
fn write_frame(value: &impl serde::Serialize) -> Result<(), Error> {
    let bytes = zeroize::Zeroizing::new(serde_json::to_vec(value).map_err(|_| Error::Protocol)?);
    if bytes.len() > 20_480 {
        return Err(Error::Capacity);
    }
    let length = u32::try_from(bytes.len())
        .map_err(|_| Error::Capacity)?
        .to_be_bytes();
    let stdout = std::io::stdout();
    let mut stream = stdout.lock();
    stream
        .write_all(&length)
        .and_then(|_| stream.write_all(&bytes))
        .and_then(|_| stream.flush())
        .map_err(|_| Error::Protocol)
}
