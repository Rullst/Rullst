//! Passes the version of the `rullst` crate in Cargo.lock to the pages ("Made with Rullst <version>").

fn main() {
    println!("cargo:rerun-if-changed=Cargo.lock");
    let lock = std::fs::read_to_string("Cargo.lock").unwrap_or_default();
    let mut lines = lock.lines();
    let mut version = None;
    while let Some(line) = lines.next() {
        if line.trim() == "name = \"rullst\"" {
            version = lines.next().and_then(|next| {
                next.trim()
                    .strip_prefix("version = \"")?
                    .strip_suffix('"')
                    .map(str::to_string)
            });
            break;
        }
    }
    match version {
        Some(version) => println!("cargo:rustc-env=RULLST_VERSION={version}"),
        None => {
            println!("cargo:warning=Cargo.lock has no rullst package");
            std::process::exit(1);
        }
    }
}
