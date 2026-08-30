//! Safe IoT telemetry-module scaffold.

use crate::generators::chat::ensure_rullst_features;
use crate::generators::{
    is_rullst_project, is_valid_rust_identifier, model_to_pascal_case, model_to_snake_case,
    register_mod_ast,
};
use colored::Colorize;
use std::fs;
use std::io::{Error as IoError, ErrorKind};
use std::path::{Path, PathBuf};

pub fn run_make_iot(device_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        return Err(IoError::new(
            ErrorKind::InvalidInput,
            "make:iot must be run inside a Rullst project",
        )
        .into());
    }

    let (module_name, type_name, source) = render_iot_device(device_name)?;
    let root = project_root_module()?;
    let target_path = Path::new("src/iot").join(format!("{module_name}.rs"));
    if target_path.exists() {
        return Err(IoError::new(
            ErrorKind::AlreadyExists,
            format!("refusing to overwrite {}", target_path.display()),
        )
        .into());
    }

    let manifest_path = Path::new("Cargo.toml");
    let manifest = fs::read_to_string(manifest_path)?;
    let updated_manifest = ensure_rullst_features(&manifest, &["iot"])?;

    println!(
        "{}",
        format!("🔌 Scaffolding IoT telemetry module for '{type_name}'...")
            .bright_cyan()
            .bold()
    );

    fs::create_dir_all("src/iot")?;
    fs::write(&target_path, source)?;
    fs::write(manifest_path, updated_manifest)?;
    register_mod_ast(Path::new("src/iot/mod.rs"), &module_name)?;
    register_mod_ast(&root, "iot")?;

    println!(
        "{}",
        format!(
            "✅ Created IoT telemetry module at '{}'.",
            target_path.display()
        )
        .green()
        .bold()
    );
    Ok(())
}

fn project_root_module() -> Result<PathBuf, IoError> {
    ["src/lib.rs", "src/main.rs"]
        .into_iter()
        .map(PathBuf::from)
        .find(|path| path.exists())
        .ok_or_else(|| {
            IoError::new(
                ErrorKind::NotFound,
                "Rullst project has neither src/lib.rs nor src/main.rs",
            )
        })
}

pub(crate) fn render_iot_device(device_name: &str) -> Result<(String, String, String), IoError> {
    let module_name = model_to_snake_case(device_name);
    let base_type = model_to_pascal_case(device_name);
    let type_name = format!("{base_type}Device");
    if !is_valid_rust_identifier(&module_name) || !is_valid_rust_identifier(&type_name) {
        return Err(IoError::new(
            ErrorKind::InvalidInput,
            "IoT device name must produce valid non-keyword Rust identifiers",
        ));
    }

    let source = format!(
        r#"use rullst::iot::SensorTelemetry;

pub struct {type_name} {{
    pub device_id: String,
}}

impl {type_name} {{
    pub fn new(device_id: impl Into<String>) -> Self {{
        Self {{ device_id: device_id.into() }}
    }}

    pub fn read_telemetry(&self, metric: &str, value: f64) -> SensorTelemetry {{
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        SensorTelemetry::new(&self.device_id, metric, value, timestamp)
    }}
}}
"#
    );
    Ok((module_name, type_name, source))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderer_emits_parseable_facade_code_and_rejects_unsafe_names() {
        let (module, type_name, source) =
            render_iot_device("TemperatureSensor").expect("valid device");
        assert_eq!(module, "temperature_sensor");
        assert_eq!(type_name, "TemperatureSensorDevice");
        assert!(source.contains("use rullst::iot::SensorTelemetry"));
        syn::parse_file(&source).expect("generated IoT module must parse");

        for invalid in ["../../escape", "type", "", "device/name", "💥"] {
            assert!(render_iot_device(invalid).is_err(), "accepted {invalid}");
        }
    }
}
