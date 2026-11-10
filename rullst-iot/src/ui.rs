//! HTMX-compatible sensor-card renderer.

extern crate alloc;
use crate::SensorTelemetry;
use alloc::format;
use alloc::string::String;
use core::fmt;

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

/// HTML that `IotDashboard` rendered, with every value inside escaped, so it
/// can be inserted into a page as it is.
///
/// `rullst-iot` does not depend on the web runtime, so this is its own type
/// rather than `rullst::html::Markup`. In a Rullst view, insert it with
/// `Markup::from_trusted(card.into_string())`: the trust is justified because
/// this crate escaped the text. A `String` never becomes `IotHtml`. It reads
/// like a `&str` (`contains`, `len`, comparisons). Added in 13.0.0-alpha.5;
/// `render_sensor_card` returned a `String` before.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IotHtml(String);

impl IotHtml {
    /// The HTML as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Takes the HTML `String` out.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for IotHtml {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl AsRef<str> for IotHtml {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl core::ops::Deref for IotHtml {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl From<IotHtml> for String {
    fn from(html: IotHtml) -> Self {
        html.0
    }
}

impl PartialEq<str> for IotHtml {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for IotHtml {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

/// The reading as shown on a card: at most two decimals without trailing
/// zeros, scientific notation with three decimals outside 0.01 to 10^9, and
/// `n/a` for NaN or infinity, so its length stays bounded.
pub(crate) fn format_reading(value: f64) -> String {
    if !value.is_finite() {
        return String::from("n/a");
    }
    let magnitude = value.abs();
    if magnitude != 0.0 && !(1e-2..1e9).contains(&magnitude) {
        return format!("{value:.3e}");
    }
    let mut text = format!("{value:.2}");
    if text.contains('.') {
        let trimmed = text.trim_end_matches('0').trim_end_matches('.').len();
        text.truncate(trimmed);
    }
    if text == "-0" {
        text = String::from("0");
    }
    text
}

/// Generator for an HTMX-compatible sensor card.
pub struct IotDashboard;

impl IotDashboard {
    /// Renders an HTMX-compatible HTML widget card for a sensor reading.
    ///
    /// The device and metric names are escaped, and the value is shown with
    /// at most two decimals (see the crate guide). Since 13.0.0-alpha.5 it
    /// returns [`IotHtml`] instead of a `String`; call
    /// [`IotHtml::into_string`] where a `String` is needed.
    #[must_use]
    pub fn render_sensor_card(telemetry: &SensorTelemetry) -> IotHtml {
        let device_id = escape_html(&telemetry.device_id);
        let metric = escape_html(&telemetry.metric);
        let value = format_reading(telemetry.value);
        IotHtml(format!(
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
            device_id, value, metric
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn test_render_sensor_card() {
        let t = SensorTelemetry::new("esp32-node", "temperature", 23.8, 1700000000);
        let html = IotDashboard::render_sensor_card(&t);
        assert!(html.contains("esp32-node"));
        assert!(html.contains(">23.8<"));
        assert!(html.contains("temperature"));
        assert!(html.contains("SNAPSHOT"));
        assert!(!html.contains("ONLINE"));
        assert_eq!(html.as_str(), html.to_string());
        assert_eq!(html.clone().into_string(), String::from(html.clone()));
        assert_eq!(html, html.as_str());
        assert_eq!(&*html, html.as_ref());
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

    #[test]
    fn sensor_card_escapes_ampersands_and_quotes() {
        let telemetry = SensorTelemetry::new(r#"a&b "c" 'd'"#, "m", 1.0, 1);
        let html = IotDashboard::render_sensor_card(&telemetry);
        assert!(html.contains("a&amp;b &quot;c&quot; &#x27;d&#x27;"));
    }

    #[test]
    fn readings_are_shown_with_bounded_precision() {
        for (value, shown) in [
            (23.8, "23.8"),
            (23.75, "23.75"),
            (1.0 / 3.0, "0.33"),
            (24.0, "24"),
            (-7.25, "-7.25"),
            (0.0, "0"),
            (-0.0, "0"),
            (-0.004, "-4.000e-3"),
            (0.001, "1.000e-3"),
            (0.01, "0.01"),
            (-0.001_9, "-1.900e-3"),
            (0.000_25, "2.500e-4"),
            (999_999_999.994, "999999999.99"),
            (1e9, "1.000e9"),
            (-1.5e300, "-1.500e300"),
            (f64::MIN_POSITIVE, "2.225e-308"),
            (f64::NAN, "n/a"),
            (f64::INFINITY, "n/a"),
            (f64::NEG_INFINITY, "n/a"),
        ] {
            assert_eq!(format_reading(value), shown, "{value:e}");
        }
        let card = IotDashboard::render_sensor_card(&SensorTelemetry::new("d", "m", 1e300, 1));
        assert!(card.contains(">1.000e300<"));
        assert!(card.len() < 600);
    }
}
