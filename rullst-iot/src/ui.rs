//! HTMX-compatible sensor-card string renderer.

extern crate alloc;
use crate::SensorTelemetry;
use alloc::format;
use alloc::string::String;

fn escape_html(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#x27;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

/// Generator for an HTMX-compatible sensor card.
pub struct IotDashboard;

impl IotDashboard {
    /// Renders an HTMX-compatible HTML widget card for a sensor reading.
    pub fn render_sensor_card(telemetry: &SensorTelemetry) -> String {
        let device_id = escape_html(&telemetry.device_id);
        let metric = escape_html(&telemetry.metric);
        format!(
            r#"<div class="iot-card bg-slate-900 text-white p-4 rounded-xl shadow-lg border border-slate-800">
  <div class="flex justify-between items-center">
    <h3 class="text-sm font-semibold text-slate-400">{}</h3>
    <span class="px-2 py-1 text-xs bg-slate-700 text-slate-300 rounded-full font-mono">SNAPSHOT</span>
  </div>
  <div class="mt-2 flex items-baseline gap-2">
    <span class="text-3xl font-bold font-mono">{}</span>
    <span class="text-xs text-slate-400">{}</span>
  </div>
</div>"#,
            device_id, telemetry.value, metric
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
        assert!(html.contains("SNAPSHOT"));
        assert!(!html.contains("ONLINE"));
    }

    #[test]
    fn sensor_card_escapes_untrusted_labels() {
        let telemetry = SensorTelemetry::new(
            "<img src=x onerror=alert(1)>",
            "temperature<script>alert(1)</script>",
            23.8,
            1_700_000_000,
        );
        let html = IotDashboard::render_sensor_card(&telemetry);
        assert!(!html.contains("<script>"));
        assert!(!html.contains("<img"));
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("&lt;img"));
    }
}
