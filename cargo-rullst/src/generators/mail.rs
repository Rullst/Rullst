// src/generators/mail.rs — Mailable Struct and Email Template generator (`cargo rullst make:mail`).

use crate::generators::chat::ensure_rullst_features;
use crate::generators::{is_rullst_project, is_valid_rust_identifier, register_mod_ast};
use colored::*;
use std::fs;
use std::io::{Error as IoError, ErrorKind};
use std::path::Path;

mod extended;
mod names;
mod templates;
use names::{project_root_module, to_pascal_case, to_snake_case};

/// Template family selected by the CLI after it rejects conflicting flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MailableKind {
    Custom,
    Welcome,
    Reset,
    Otp,
    Invoice,
    PaymentReceipt,
    Dunning,
}

/// `manifest` with the `mail` feature of `rullst` unless a feature it has
/// already enables it. The generated mailable needs only `rullst::mail`; the
/// SMTP transport (`mailer`, `mail-smtp`) is the application's choice.
fn with_mail_feature(manifest: &str) -> Result<String, Box<dyn std::error::Error>> {
    let features = toml::from_str::<toml::Value>(manifest)
        .ok()
        .and_then(|manifest| {
            manifest
                .get("dependencies")?
                .get("rullst")?
                .get("features")?
                .as_array()
                .cloned()
        })
        .unwrap_or_default();
    if crate::generators::add::enabled_by("mail", features.iter().filter_map(toml::Value::as_str)) {
        return Ok(manifest.to_owned());
    }
    ensure_rullst_features(manifest, &["mail"])
}

pub(crate) fn create_new_mailable(
    name: &str,
    kind: MailableKind,
) -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        return Err(IoError::new(
            ErrorKind::InvalidInput,
            "make:mail must be run inside a Rullst project",
        )
        .into());
    }
    let pascal_name = to_pascal_case(name);
    let snake_name = to_snake_case(&pascal_name);
    if !is_valid_rust_identifier(&pascal_name) || !is_valid_rust_identifier(&snake_name) {
        return Err(IoError::new(
            ErrorKind::InvalidInput,
            "mailable name must produce valid non-keyword Rust identifiers",
        )
        .into());
    }

    let root = project_root_module()?;
    let root_content = fs::read_to_string(&root)?;
    syn::parse_file(&root_content).map_err(|error| {
        IoError::new(
            ErrorKind::InvalidData,
            format!("{} is not valid Rust: {error}", root.display()),
        )
    })?;

    let mail_dir = Path::new("src/mail");
    let mod_path = mail_dir.join("mod.rs");
    let file_path = mail_dir.join(format!("{snake_name}.rs"));
    if file_path.exists() {
        return Err(IoError::new(
            ErrorKind::AlreadyExists,
            format!("refusing to overwrite {}", file_path.display()),
        )
        .into());
    }

    let mut mod_content = if mod_path.exists() {
        fs::read_to_string(&mod_path)?
    } else {
        String::new()
    };
    syn::parse_file(&mod_content).map_err(|error| {
        IoError::new(
            ErrorKind::InvalidData,
            format!("{} is not valid Rust: {error}", mod_path.display()),
        )
    })?;
    let module_declaration = format!("pub mod {snake_name};");
    if mod_content
        .lines()
        .any(|line| line.trim() == module_declaration)
    {
        return Err(IoError::new(
            ErrorKind::AlreadyExists,
            format!("mailable module {snake_name} is already registered"),
        )
        .into());
    }
    if !mod_content.is_empty() && !mod_content.ends_with('\n') {
        mod_content.push('\n');
    }
    mod_content.push_str(&format!(
        "pub mod {snake_name};\npub use {snake_name}::{pascal_name};\n"
    ));

    let manifest_path = Path::new("Cargo.toml");
    let manifest = fs::read_to_string(manifest_path)?;
    let updated_manifest = with_mail_feature(&manifest)?;

    println!(
        "{}",
        format!("📬 Generating Mailable Rullst: {}...", pascal_name)
            .cyan()
            .bold()
    );

    let template = templates::source(kind);

    let content = template.replace("__NAME__", &pascal_name);
    syn::parse_file(&content).map_err(|error| {
        IoError::new(
            ErrorKind::InvalidData,
            format!("generated mailable is not valid Rust: {error}"),
        )
    })?;

    fs::create_dir_all(mail_dir)?;
    fs::write(&file_path, content)?;
    if updated_manifest != manifest {
        fs::write(manifest_path, updated_manifest)?;
        println!(
            "{}",
            "ℹ️ Enabled the `mail` feature of rullst in Cargo.toml, as `cargo rullst add mail` does.".cyan()
        );
    }
    fs::write(&mod_path, mod_content)?;
    register_mod_ast(&root, "mail")?;

    println!(
        "{}",
        format!(
            "✅ Mailable created successfully at: {}",
            file_path.display()
        )
        .green()
    );
    println!(
        "👉 Messages without `from` use MAIL_FROM (or `from` under [mail] in Rullst.toml); staging and production also need MAIL_DRIVER."
    );

    Ok(())
}
