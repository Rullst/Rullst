//! Explicit schema-first generation; route scanning cannot infer wire contracts.
use super::consumer_files::{self, Edit};
use clap::{Arg, ArgAction, ArgMatches, Command};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};
mod input;
mod operations;
mod render_rust;
mod render_ts;
mod schema;
#[cfg(test)]
mod tests;

const MAX_SOURCE: usize = 128 * 1024;
const MAX_OUTPUT: usize = 512 * 1024;
const MARKER: &str = "// rullst.api.v1; generated file\n";
#[derive(Debug, thiserror::Error)]
pub(crate) enum ApiError {
    #[error("invalid API contract; check required fields, unique keys and names")]
    Invalid,
    #[error("unsupported API contract shape or keyword; see the rullst.api.v1 profile")]
    Unsupported,
    #[error("API contract exceeds a byte, depth, count or value budget")]
    Limit,
    #[error("API generation refuses links, special files or unrecognized output")]
    Path,
    #[error("generated API is missing, changed or stale; regenerate it")]
    Stale,
    #[error("API generation filesystem operation failed")]
    Io(#[from] std::io::Error),
    #[error("API contract encoding failed")]
    Encoding,
}
pub(crate) fn command() -> Command {
    Command::new("generate:api")
        .about("Generate explicit Rust and TypeScript contracts from the bounded OpenAPI profile")
        .arg(Arg::new("schema").long("schema").required(true))
        .arg(Arg::new("output").long("output").required(true))
        .arg(Arg::new("check").long("check").action(ArgAction::SetTrue))
}
pub(crate) fn run(matches: &ArgMatches) -> Result<(), ApiError> {
    let source = matches
        .get_one::<String>("schema")
        .ok_or(ApiError::Invalid)?;
    let output = matches
        .get_one::<String>("output")
        .ok_or(ApiError::Invalid)?;
    generate(
        Path::new(source),
        Path::new(output),
        matches.get_flag("check"),
    )?;
    println!(
        "API contract outputs match the explicit schema. Review and mount the server codecs with application authorization."
    );
    Ok(())
}
fn generate(source: &Path, output: &Path, check: bool) -> Result<(), ApiError> {
    // Resolve OS aliases once; descendant output links remain forbidden.
    let root = fs::canonicalize(".")?;
    let source = root.join(source);
    let output = root.join(output);
    input::regular(&output)?;
    let content = input::read(&source, MAX_SOURCE)?;
    let doc = operations::Document::parse(input::parse(content.as_bytes())?)?;
    let canonical =
        serde_json::to_string_pretty(&doc.source).map_err(|_| ApiError::Encoding)? + "\n";
    let digest = hex::encode(Sha256::digest(canonical.as_bytes()));
    let mut published_schema = doc.source.clone();
    published_schema["x-rullst-generated"] = serde_json::json!({
        "generator": "cargo-rullst", "profile": "rullst.api.v1",
        "version": env!("CARGO_PKG_VERSION"), "schema_sha256": digest,
    });
    let published_schema =
        serde_json::to_string_pretty(&published_schema).map_err(|_| ApiError::Encoding)? + "\n";
    let header = format!(
        "{MARKER}// schema-sha256: {digest}\n// generator-version: {}\n",
        env!("CARGO_PKG_VERSION")
    );
    let files = [
        ("contract.rs", header.clone() + &render_rust::render(&doc)?),
        ("client.ts", header + &render_ts::render(&doc)?),
        ("openapi.json", published_schema),
    ];
    let mut edits = Vec::new();
    for (name, content) in files {
        if content.len() > MAX_OUTPUT {
            return Err(ApiError::Limit);
        }
        let path = output.join(name);
        input::regular(&path)?;
        match fs::symlink_metadata(&path) {
            Ok(_) => {
                let old = input::read(&path, MAX_OUTPUT)?;
                if old == content {
                    continue;
                }
                if check {
                    return Err(ApiError::Stale);
                }
                let recognized = if name == "openapi.json" {
                    serde_json::from_str::<serde_json::Value>(&old).is_ok_and(|v| {
                        v["x-rullst-generated"]["generator"] == "cargo-rullst"
                            && v["x-rullst-generated"]["profile"] == "rullst.api.v1"
                    })
                } else {
                    old.starts_with(MARKER)
                };
                if !recognized {
                    return Err(ApiError::Path);
                }
                edits.push(Edit::replace(path, old, content));
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if check {
                    return Err(ApiError::Stale);
                }
                edits.push(Edit::create(path, content)?);
            }
            Err(e) => return Err(e.into()),
        }
    }
    consumer_files::apply(&edits)?;
    Ok(())
}
