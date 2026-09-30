use crate::nexus::assets::{HTMX_PATH, SCRIPT_PATH, STYLESHEET_PATH};
use crate::nexus::types::NexusState;

pub(crate) fn safe_icon_html(icon: &str) -> String {
    let mut decoded = String::with_capacity(icon.len());
    let mut remainder = icon;

    while let Some(start) = remainder.find("&#") {
        decoded.push_str(&remainder[..start]);
        let entity = &remainder[start + 2..];
        let Some(end) = entity.find(';') else {
            decoded.push_str(&remainder[start..]);
            remainder = "";
            break;
        };
        let digits = &entity[..end];
        match digits.parse::<u32>().ok().and_then(char::from_u32) {
            Some(character) => decoded.push(character),
            None => decoded.push_str(&remainder[start..start + end + 3]),
        }
        remainder = &entity[end + 1..];
    }
    decoded.push_str(remainder);

    rullst_core::html::escape_str(&decoded).into_owned()
}

pub fn render_sidebar(state: &NexusState, active_table: Option<&str>) -> String {
    let mut out = String::new();
    for m in state.registry.iter() {
        let is_active = active_table == Some(m.table);
        let active_class = if is_active { " nexus-nav-active" } else { "" };
        let table_path = urlencoding::encode(m.table);
        let label = rullst_core::html::escape_str(m.label);
        let icon = safe_icon_html(m.icon);
        let _ = std::fmt::Write::write_fmt(
            &mut out,
            format_args!(
                "<a href=\"/nexus/table/{table_path}\" class=\"nexus-nav-link{active_class}\" \
             hx-get=\"/nexus/table/{table_path}\" hx-target=\"#nexus-content\" hx-push-url=\"true\">\
             <span class=\"nexus-nav-icon\">{icon}</span><span>{label}</span></a>"
            ),
        );
    }
    out.push_str("<div class=\"nexus-nav-divider\"></div>");
    let ai_active = if active_table == Some("chat") {
        " nexus-nav-active"
    } else {
        ""
    };
    let sec_active = if active_table == Some("security") {
        " nexus-nav-active"
    } else {
        ""
    };
    let tel_active = if active_table == Some("telemetry") {
        " nexus-nav-active"
    } else {
        ""
    };
    let _ = std::fmt::Write::write_fmt(
        &mut out,
        format_args!(
            "<a href=\"/nexus/chat\" class=\"nexus-nav-link nexus-nav-ai{ai_active}\" \
             hx-get=\"/nexus/chat\" hx-target=\"#nexus-content\" hx-push-url=\"true\">\
             <span class=\"nexus-nav-icon\">&#129302;</span><span>AI Assistant</span></a>\
             <a href=\"/nexus/security\" class=\"nexus-nav-link nexus-nav-sec{sec_active}\" \
             hx-get=\"/nexus/security\" hx-target=\"#nexus-content\" hx-push-url=\"true\">\
             <span class=\"nexus-nav-icon\">&#128737;</span><span>Threat Radar (SOC)</span></a>\
             <a href=\"/nexus/telemetry\" class=\"nexus-nav-link nexus-nav-tel{tel_active}\" \
             hx-get=\"/nexus/telemetry\" hx-target=\"#nexus-content\" hx-push-url=\"true\">\
             <span class=\"nexus-nav-icon\">&#9889;</span><span>Telemetry &amp; Metrics</span></a>"
        ),
    );
    out
}

pub fn render_shell(state: &NexusState, sidebar: &str, content: &str) -> String {
    let brand = rullst_core::html::escape_str(state.brand.as_str());
    let mut out = String::new();
    out.push_str("<!DOCTYPE html>\n<html lang=\"en\" data-theme=\"dark\">\n<head>\n");
    out.push_str("<meta charset=\"UTF-8\" />\n");
    out.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\" />\n");
    let _ = std::fmt::Write::write_fmt(
        &mut out,
        format_args!("<title>{brand} &mdash; Nexus Panel</title>\n"),
    );
    out.push_str("<meta name=\"description\" content=\"Rullst Nexus: Auto-Generated CMS &amp; AI Admin Panel\" />\n");
    // Same-origin assets only: the production CSP (`script-src 'self'`,
    // `style-src 'self'`, `img-src 'self' data:`) must not need relaxing.
    // htmx must not evaluate code or inject a style element under that policy.
    out.push_str("<meta name=\"htmx-config\" content='{\"allowEval\":false,\"allowScriptTags\":false,\"includeIndicatorStyles\":false}' />\n");
    out.push_str("<link rel=\"icon\" href=\"data:,\" />\n");
    let _ = std::fmt::Write::write_fmt(
        &mut out,
        format_args!(
            "<link rel=\"stylesheet\" href=\"{STYLESHEET_PATH}\" />\n\
             <script src=\"{HTMX_PATH}\" defer></script>\n\
             <script src=\"{SCRIPT_PATH}\" defer></script>\n"
        ),
    );
    out.push_str("</head>\n<body class=\"nexus-body\">\n");

    out.push_str(
        "<nav class=\"nexus-sidebar\" id=\"nexus-sidebar\" aria-label=\"Nexus navigation\">",
    );
    out.push_str("<div class=\"nexus-brand\">");
    out.push_str("<span class=\"nexus-brand-mark\" aria-hidden=\"true\">R</span>");
    let _ = std::fmt::Write::write_fmt(
        &mut out,
        format_args!("<span class=\"nexus-brand-name\">{brand}</span>"),
    );
    out.push_str("<button type=\"button\" class=\"nexus-sidebar-close\" id=\"nexus-sidebar-close\" aria-label=\"Close navigation\">&times;</button></div>");
    out.push_str("<div class=\"nexus-nav-label\">MODELS</div>");
    out.push_str(sidebar);
    out.push_str("<div class=\"nexus-sidebar-footer\">");
    out.push_str("<a href=\"/\" class=\"nexus-nav-link nexus-nav-home\"><span class=\"nexus-nav-icon\">&#127968;</span><span>Back to App</span></a>");
    let _ = std::fmt::Write::write_fmt(
        &mut out,
        format_args!(
            "<div class=\"nexus-version\">Rullst Nexus v{}</div>",
            env!("CARGO_PKG_VERSION")
        ),
    );
    out.push_str("</div></nav>");
    out.push_str("<div class=\"nexus-sidebar-backdrop\" id=\"nexus-sidebar-backdrop\" hidden aria-hidden=\"true\"></div>");

    out.push_str("<main class=\"nexus-main\">");
    out.push_str("<header class=\"nexus-topbar\">");
    out.push_str("<button type=\"button\" class=\"nexus-topbar-toggle\" aria-label=\"Open navigation\" aria-controls=\"nexus-sidebar\" aria-expanded=\"false\">&#9776;</button>");
    out.push_str("<div class=\"nexus-topbar-breadcrumb\" id=\"nexus-breadcrumb\">Dashboard</div>");
    out.push_str("<div class=\"nexus-topbar-actions\">");
    out.push_str("<div class=\"nexus-htmx-indicator\" id=\"nexus-htmx-indicator\">");
    out.push_str("<span class=\"nexus-spinner\"></span>Loading...");
    out.push_str("</div></div></header>");
    out.push_str(
        "<div class=\"nexus-content\" id=\"nexus-content\" hx-indicator=\"#nexus-htmx-indicator\">",
    );
    out.push_str(content);
    out.push_str("</div><div id=\"nexus-toast\"></div></main>\n</body>\n</html>");
    out
}

/// The Nexus stylesheet, served same-origin at `/nexus/assets/nexus.css`.
pub const NEXUS_CSS: &str = include_str!("../../assets/nexus.css");

#[cfg(test)]
mod icon_tests {
    use super::safe_icon_html;

    #[test]
    fn icon_renderer_decodes_numeric_entities_and_escapes_markup() {
        assert_eq!(safe_icon_html("&#128196;"), "📄");
        assert_eq!(
            safe_icon_html("<img src=x onerror=alert(1)>&#60;"),
            "&lt;img src=x onerror=alert(1)&gt;&lt;"
        );
    }
}
