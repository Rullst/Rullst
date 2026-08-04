//! Embedded Micro-Dashboard HTMX UI Generator (< 50KB).

extern crate alloc;
use crate::SensorTelemetry;
use alloc::format;
use alloc::string::String;

/// Generator for lightweight HTMX micro-dashboards.
pub struct IotDashboard;

impl IotDashboard {
    /// Renders an HTMX-compatible HTML widget card for a sensor reading.
    pub fn render_sensor_card(telemetry: &SensorTelemetry) -> String {
        format!(
            r#"<div class="iot-card bg-slate-900 text-white p-4 rounded-xl shadow-lg border border-slate-800">
  <div class="flex justify-between items-center">
    <h3 class="text-sm font-semibold text-slate-400">{}</h3>
    <span class="px-2 py-1 text-xs bg-emerald-500/20 text-emerald-400 rounded-full font-mono">ONLINE</span>
  </div>
  <div class="mt-2 flex items-baseline gap-2">
    <span class="text-3xl font-bold font-mono">{}</span>
    <span class="text-xs text-slate-400">{}</span>
  </div>
</div>"#,
            telemetry.device_id, telemetry.value, telemetry.metric
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_sensor_card() {
        let t = SensorTelemetry::new("esp32-node", "temperature", 23.8, 1700000000);
        let html = IotDashboard::render_sensor_card(&t);
        assert!(html.contains("esp32-node"));
        assert!(html.contains("23.8"));
        assert!(html.contains("temperature"));
    }
}
