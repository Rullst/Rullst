use colored::Colorize;
use std::fs;
use std::path::Path;

pub fn run_make_iot(device_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        format!(
            "🔌 Scaffolding IoT Edge Device Module for '{}'...",
            device_name
        )
        .bright_cyan()
        .bold()
    );

    let src_dir = Path::new("src/iot");
    if !src_dir.exists() {
        fs::create_dir_all(src_dir)?;
    }

    let file_name = format!("{}.rs", device_name.to_lowercase());
    let target_path = src_dir.join(&file_name);

    let dev_struct = format!("{}Device", device_name);
    let code = format!(
        "// =========================================================================\n\
         // Rullst IoT Device Module — {dev_name}\n\
         // =========================================================================\n\n\
         use rullst_iot::{{SensorTelemetry, MqttDriver}};\n\n\
         pub struct {dev_struct} {{\n\
             pub device_id: String,\n\
         }}\n\n\
         impl {dev_struct} {{\n\
             pub fn new(device_id: impl Into<String>) -> Self {{\n\
                 Self {{\n\
                     device_id: device_id.into(),\n\
                 }}\n\
             }}\n\n\
             pub fn read_telemetry(&self, metric: &str, value: f64) -> SensorTelemetry {{\n\
                 let ts = std::time::SystemTime::now()\n\
                     .duration_since(std::time::UNIX_EPOCH)\n\
                     .unwrap_or_default()\n\
                     .as_secs();\n\n\
                 SensorTelemetry::new(&self.device_id, metric, value, ts)\n\
             }}\n\
         }}\n",
        dev_name = device_name,
        dev_struct = dev_struct
    );

    fs::write(&target_path, code)?;

    println!(
        "{}",
        format!(
            "✅ Successfully created IoT device scaffold at '{}'!",
            target_path.display()
        )
        .green()
        .bold()
    );

    Ok(())
}
