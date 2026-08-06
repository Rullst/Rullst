use crate::nexus::types::NexusState;

pub fn render_sidebar(state: &NexusState, active_table: Option<&str>) -> String {
    let mut out = String::new();
    for m in state.registry.iter() {
        let is_active = active_table == Some(m.table);
        let active_class = if is_active { " nexus-nav-active" } else { "" };
        let t = m.table;
        let lb = m.label;
        let ic = m.icon;
        let _ = std::fmt::Write::write_fmt(
            &mut out,
            format_args!(
                "<a href=\"/nexus/table/{t}\" class=\"nexus-nav-link{active_class}\" \
             hx-get=\"/nexus/table/{t}\" hx-target=\"#nexus-content\" hx-push-url=\"true\">\
             <span class=\"nexus-nav-icon\">{ic}</span><span>{lb}</span></a>"
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
    out.push_str("<link rel=\"icon\" type=\"image/svg+xml\" href=\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'%3E%3Ctext y='.9em' font-size='90'%3E🦀%3C/text%3E%3C/svg%3E\" />\n");
    out.push_str("<script src=\"https://unpkg.com/htmx.org@2.0.4\" defer></script>\n");
    out.push_str("<script>\n");
    out.push_str("document.addEventListener('htmx:configRequest', function(evt) {\n");
    out.push_str(
        "    let match = document.cookie.match(new RegExp('(^| )rullst_csrf=([^;]+)'));\n",
    );
    out.push_str("    if (match) {\n");
    out.push_str("        evt.detail.headers['X-CSRF-Token'] = match[2];\n");
    out.push_str("    }\n");
    out.push_str("});\n");
    out.push_str("</script>\n");
    out.push_str("<link rel=\"preconnect\" href=\"https://fonts.googleapis.com\">\n");
    out.push_str("<link href=\"https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&amp;family=JetBrains+Mono:wght@400;500&amp;display=swap\" rel=\"stylesheet\">\n");
    out.push_str("<style>\n");
    out.push_str(NEXUS_CSS);
    out.push_str("\n</style>\n</head>\n<body class=\"nexus-body\">\n");

    out.push_str("<nav class=\"nexus-sidebar\" id=\"nexus-sidebar\">");
    out.push_str("<div class=\"nexus-brand\">");
    out.push_str("<span class=\"nexus-brand-icon\">&#129408;</span>");
    let _ = std::fmt::Write::write_fmt(
        &mut out,
        format_args!("<span class=\"nexus-brand-name\">{brand}</span>"),
    );
    out.push_str("</div>");
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

    out.push_str("<main class=\"nexus-main\">");
    out.push_str("<header class=\"nexus-topbar\">");
    out.push_str("<button class=\"nexus-topbar-toggle\" onclick=\"document.getElementById(&quot;nexus-sidebar&quot;).classList.toggle(&quot;nexus-sidebar-open&quot;)\">&#9776;</button>");
    out.push_str("<div class=\"nexus-topbar-breadcrumb\" id=\"nexus-breadcrumb\">Dashboard</div>");
    out.push_str("<div class=\"nexus-topbar-actions\">");
    out.push_str("<div class=\"nexus-htmx-indicator\" id=\"nexus-htmx-indicator\">");
    out.push_str("<span class=\"nexus-spinner\"></span>Loading...");
    out.push_str("</div></div></header>");
    out.push_str(
        "<div class=\"nexus-content\" id=\"nexus-content\" hx-indicator=\"#nexus-htmx-indicator\">",
    );
    out.push_str(content);
    out.push_str("</div></main>\n</body>\n</html>");
    out
}

pub const NEXUS_CSS: &str = "
/* == Reset & Base ===================================================== */
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

:root {
    --bg-900:  #0b0d14;
    --bg-800:  #111520;
    --bg-700:  #171c2e;
    --bg-600:  #1e253d;
    --bg-500:  #262f4a;
    --border:  rgba(99,116,183,0.18);
    --accent:  #6366f1;
    --accent-h: #818cf8;
    --accent-glow: rgba(99,102,241,0.35);
    --text-100: #f1f5f9;
    --text-300: #94a3b8;
    --text-500: #475569;
    --green:   #10b981;
    --red:     #ef4444;
    --yellow:  #f59e0b;
    --radius:  12px;
    --radius-sm: 8px;
    --shadow:  0 8px 32px rgba(0,0,0,0.45);
    --sidebar-w: 240px;
    --topbar-h: 56px;
    --font-sans: 'Inter', -apple-system, sans-serif;
    --font-mono: 'JetBrains Mono', monospace;
    --transition: 0.2s cubic-bezier(0.4,0,0.2,1);
}
html, body { height: 100%; }
.nexus-body { font-family: var(--font-sans); background: var(--bg-900); color: var(--text-100); display: flex; height: 100vh; overflow: hidden; }

/* == Sidebar =========================================================== */
.nexus-sidebar { width: var(--sidebar-w); min-width: var(--sidebar-w); height: 100vh; background: var(--bg-800); border-right: 1px solid var(--border); display: flex; flex-direction: column; overflow-y: auto; z-index: 100; transition: transform var(--transition); padding-bottom: 16px; }
.nexus-brand { display: flex; align-items: center; gap: 10px; padding: 20px 20px 16px; border-bottom: 1px solid var(--border); margin-bottom: 12px; flex-shrink: 0; }
.nexus-brand-icon { font-size: 22px; }
.nexus-brand-name { font-size: 15px; font-weight: 700; background: linear-gradient(135deg, #818cf8, #c084fc); -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text; }
.nexus-nav-label { font-size: 10px; font-weight: 600; letter-spacing: 0.1em; color: var(--text-500); padding: 0 20px 8px; text-transform: uppercase; }
.nexus-nav-link { display: flex; align-items: center; gap: 10px; padding: 9px 20px; color: var(--text-300); text-decoration: none; font-size: 13.5px; font-weight: 500; border-left: 3px solid transparent; transition: background var(--transition), color var(--transition); cursor: pointer; }
.nexus-nav-link:hover { background: var(--bg-700); color: var(--text-100); }
.nexus-nav-active { background: linear-gradient(90deg, rgba(99,102,241,0.15), transparent); color: var(--accent-h) !important; border-left-color: var(--accent) !important; }
.nexus-nav-ai { color: #c084fc !important; }
.nexus-nav-ai:hover { background: rgba(192,132,252,0.08) !important; }
.nexus-nav-icon { font-size: 16px; width: 20px; text-align: center; }
.nexus-nav-divider { height: 1px; background: var(--border); margin: 12px 16px; }
.nexus-sidebar-footer { margin-top: auto; padding-top: 8px; border-top: 1px solid var(--border); }
.nexus-version { font-size: 10px; color: var(--text-500); text-align: center; padding: 8px; }

/* == Main Layout ======================================================= */
.nexus-main { flex: 1; display: flex; flex-direction: column; overflow: hidden; min-width: 0; }
.nexus-topbar { height: var(--topbar-h); background: var(--bg-800); border-bottom: 1px solid var(--border); display: flex; align-items: center; gap: 16px; padding: 0 24px; flex-shrink: 0; }
.nexus-topbar-toggle { background: none; border: none; color: var(--text-300); font-size: 18px; cursor: pointer; display: none; padding: 4px 8px; border-radius: 6px; }
.nexus-topbar-toggle:hover { background: var(--bg-700); color: var(--text-100); }
.nexus-topbar-breadcrumb { font-size: 13px; color: var(--text-300); flex: 1; }
.nexus-topbar-actions { display: flex; align-items: center; gap: 12px; }
.nexus-htmx-indicator { display: none; align-items: center; gap: 6px; font-size: 12px; color: var(--accent-h); }
.htmx-request .nexus-htmx-indicator { display: flex; }
.nexus-spinner { width: 14px; height: 14px; border: 2px solid rgba(99,102,241,0.3); border-top-color: var(--accent); border-radius: 50%; animation: nexus-spin 0.6s linear infinite; }
@keyframes nexus-spin { to { transform: rotate(360deg); } }
.nexus-content { flex: 1; overflow-y: auto; padding: 28px 32px; background: var(--bg-900); }

/* == Page Header ======================================================= */
.nexus-page-header { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; margin-bottom: 28px; }
.nexus-page-title { font-size: 24px; font-weight: 700; color: var(--text-100); line-height: 1.2; }
.nexus-page-subtitle { font-size: 13.5px; color: var(--text-300); margin-top: 4px; }
.nexus-page-subtitle code { font-family: var(--font-mono); background: var(--bg-600); padding: 1px 6px; border-radius: 4px; font-size: 12px; }

/* == Dashboard Cards =================================================== */
.nexus-stat-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 16px; margin-bottom: 28px; }
.nexus-stat-card { background: var(--bg-700); border: 1px solid var(--border); border-radius: var(--radius); padding: 24px 20px; text-decoration: none; color: var(--text-100); cursor: pointer; transition: all var(--transition); display: flex; flex-direction: column; gap: 8px; position: relative; overflow: hidden; }
.nexus-stat-card::before { content: ''; position: absolute; inset: 0; background: linear-gradient(135deg, var(--accent-glow), transparent); opacity: 0; transition: opacity var(--transition); }
.nexus-stat-card:hover { border-color: var(--accent); transform: translateY(-2px); box-shadow: 0 8px 24px var(--accent-glow); }
.nexus-stat-card:hover::before { opacity: 1; }
.nexus-stat-icon { font-size: 32px; }
.nexus-stat-label { font-weight: 600; font-size: 15px; }
.nexus-stat-hint { font-size: 12px; color: var(--text-300); }
.nexus-welcome-box { background: linear-gradient(135deg, var(--bg-700), var(--bg-600)); border: 1px solid var(--border); border-radius: var(--radius); padding: 32px; text-align: center; display: flex; flex-direction: column; align-items: center; gap: 12px; }
.nexus-welcome-icon { font-size: 40px; }
.nexus-welcome-box h2 { font-size: 18px; font-weight: 600; }
.nexus-welcome-box p { color: var(--text-300); max-width: 480px; font-size: 14px; }

/* == Toolbar =========================================================== */
.nexus-toolbar { display: flex; align-items: center; gap: 12px; margin-bottom: 16px; }
.nexus-search-wrap { position: relative; flex: 1; max-width: 360px; }
.nexus-search-icon { position: absolute; left: 12px; top: 50%; transform: translateY(-50%); font-size: 14px; pointer-events: none; }
.nexus-search-input { width: 100%; background: var(--bg-700); border: 1px solid var(--border); border-radius: var(--radius-sm); color: var(--text-100); font-family: var(--font-sans); font-size: 13.5px; padding: 9px 12px 9px 36px; outline: none; transition: border-color var(--transition), box-shadow var(--transition); }
.nexus-search-input:focus { border-color: var(--accent); box-shadow: 0 0 0 3px var(--accent-glow); }
.nexus-page-badge { font-size: 12px; color: var(--text-300); background: var(--bg-700); border: 1px solid var(--border); border-radius: 20px; padding: 4px 12px; }

/* == Table ============================================================= */
.nexus-table-wrap { background: var(--bg-800); border: 1px solid var(--border); border-radius: var(--radius); overflow: hidden; margin-bottom: 16px; }
.nexus-table { width: 100%; border-collapse: collapse; font-size: 13.5px; }
.nexus-thead-row { background: var(--bg-700); border-bottom: 1px solid var(--border); }
.nexus-th { text-align: left; padding: 12px 16px; font-weight: 600; font-size: 12px; letter-spacing: 0.04em; color: var(--text-300); text-transform: uppercase; white-space: nowrap; }
.nexus-th-actions { text-align: right; }
.nexus-tr { border-bottom: 1px solid var(--border); transition: background var(--transition); }
.nexus-tr:last-child { border-bottom: none; }
.nexus-tr:hover { background: var(--bg-700); }
.nexus-td { padding: 13px 16px; color: var(--text-100); vertical-align: middle; }
.nexus-td-actions { text-align: right; white-space: nowrap; }
.nexus-empty-row { padding: 32px; text-align: center; color: var(--text-500); }
.nexus-row-deleted { opacity: 0.4; }

/* == Action Buttons ==================================================== */
.nexus-action-btn { background: none; border: 1px solid var(--border); border-radius: 6px; padding: 5px 10px; cursor: pointer; font-size: 13px; color: var(--text-300); transition: all var(--transition); margin-left: 4px; }
.nexus-action-edit:hover { border-color: var(--accent); color: var(--accent-h); background: var(--accent-glow); }
.nexus-action-delete:hover { border-color: var(--red); color: var(--red); background: rgba(239,68,68,0.1); }

/* == Pagination ======================================================== */
.nexus-pagination { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-top: 16px; }
.nexus-page-indicator { font-size: 13px; color: var(--text-300); }

/* == Buttons =========================================================== */
.nexus-btn { display: inline-flex; align-items: center; gap: 6px; padding: 9px 18px; border-radius: var(--radius-sm); font-family: var(--font-sans); font-size: 13.5px; font-weight: 600; cursor: pointer; text-decoration: none; border: 1px solid transparent; transition: all var(--transition); white-space: nowrap; }
.nexus-btn-primary { background: var(--accent); color: #fff; border-color: var(--accent); }
.nexus-btn-primary:hover { background: var(--accent-h); box-shadow: 0 4px 16px var(--accent-glow); transform: translateY(-1px); }
.nexus-btn-ghost { background: transparent; color: var(--text-300); border-color: var(--border); }
.nexus-btn-ghost:hover { background: var(--bg-700); color: var(--text-100); border-color: var(--accent); }
.nexus-btn-ai { background: linear-gradient(135deg, #7c3aed, #c026d3); color: #fff; border: none; }
.nexus-btn-ai:hover { filter: brightness(1.15); box-shadow: 0 4px 20px rgba(192,38,211,0.4); transform: translateY(-1px); }

/* == Toast ============================================================= */
.nexus-toast { position: fixed; bottom: 24px; right: 24px; padding: 12px 20px; border-radius: var(--radius-sm); font-size: 13.5px; font-weight: 500; z-index: 1000; box-shadow: var(--shadow); }
.nexus-toast-success { background: rgba(16,185,129,0.15); border: 1px solid var(--green); color: var(--green); animation: nexus-toast-in 0.3s ease; }
.nexus-toast-warning { background: rgba(245,158,11,0.15); border: 1px solid var(--yellow); color: var(--yellow); animation: nexus-toast-in 0.3s ease; }
@keyframes nexus-toast-in { from { opacity: 0; transform: translateY(12px); } to { opacity: 1; transform: translateY(0); } }

/* == Modal ============================================================= */
.nexus-modal {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    margin: 0;
    background: var(--bg-800);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 0;
    color: var(--text-100);
    max-width: 500px;
    width: 90vw;
    box-shadow: 0 25px 50px -12px rgba(0,0,0,0.5);
}
.nexus-modal::backdrop { background: rgba(0,0,0,0.7); backdrop-filter: blur(4px); }
.nexus-modal-inner { padding: 28px; }
.nexus-modal-close { position: absolute; top: 16px; right: 16px; background: var(--bg-700); border: 1px solid var(--border); color: var(--text-300); border-radius: 6px; width: 28px; height: 28px; cursor: pointer; font-size: 13px; display: flex; align-items: center; justify-content: center; transition: all var(--transition); }
.nexus-modal-close:hover { background: var(--red); border-color: var(--red); color: #fff; }
.nexus-modal-title { font-size: 18px; font-weight: 700; margin-bottom: 20px; }

/* == Form ============================================================== */
.nexus-fields-grid { display: grid; grid-template-columns: 1fr; gap: 16px; margin-bottom: 24px; }
.nexus-form-group { display: flex; flex-direction: column; gap: 6px; }
.nexus-label { font-size: 12px; font-weight: 600; color: var(--text-300); text-transform: uppercase; letter-spacing: 0.04em; display: flex; align-items: center; gap: 6px; }
.nexus-input { background: var(--bg-700); border: 1px solid var(--border); border-radius: var(--radius-sm); color: var(--text-100); font-family: var(--font-sans); font-size: 13.5px; padding: 10px 12px; width: 100%; outline: none; }
.nexus-input:focus { border-color: var(--accent); box-shadow: 0 0 0 3px var(--accent-glow); }
.nexus-badge { background: var(--bg-500); border: 1px solid var(--border); color: var(--text-500); border-radius: 4px; font-size: 10px; padding: 1px 5px; }
.nexus-form-actions { display: flex; justify-content: flex-end; gap: 10px; border-top: 1px solid var(--border); padding-top: 20px; }

/* == Chat ============================================================== */
.nexus-chat-layout { display: grid; grid-template-columns: 280px 1fr; gap: 20px; height: calc(100vh - var(--topbar-h) - 160px); }
.nexus-chat-schema { background: var(--bg-800); border: 1px solid var(--border); border-radius: var(--radius); padding: 20px; overflow-y: auto; }
.nexus-schema-title { font-size: 12px; font-weight: 700; color: var(--text-500); text-transform: uppercase; letter-spacing: 0.06em; margin-bottom: 12px; }
.nexus-schema-pre { font-family: var(--font-mono); font-size: 11.5px; color: var(--text-300); white-space: pre-wrap; word-break: break-all; line-height: 1.6; }
.nexus-chat-panel { background: var(--bg-800); border: 1px solid var(--border); border-radius: var(--radius); display: flex; flex-direction: column; overflow: hidden; }
.nexus-chat-messages { flex: 1; overflow-y: auto; padding: 20px; display: flex; flex-direction: column; gap: 16px; }
.nexus-chat-bubble { display: flex; gap: 12px; align-items: flex-start; animation: nexus-bubble-in 0.25s ease; }
@keyframes nexus-bubble-in { from { opacity: 0; transform: translateY(8px); } to { opacity: 1; transform: translateY(0); } }
.nexus-chat-user { flex-direction: row-reverse; }
.nexus-chat-avatar { width: 36px; height: 36px; border-radius: 50%; background: var(--bg-600); border: 1px solid var(--border); display: flex; align-items: center; justify-content: center; font-size: 18px; flex-shrink: 0; }
.nexus-chat-text { background: var(--bg-700); border: 1px solid var(--border); border-radius: 12px; padding: 12px 16px; font-size: 13.5px; line-height: 1.6; max-width: 80%; }
.nexus-chat-user .nexus-chat-text { background: linear-gradient(135deg, rgba(99,102,241,0.25), rgba(99,102,241,0.1)); border-color: rgba(99,102,241,0.4); }
.nexus-chat-form { display: flex; gap: 10px; padding: 16px; border-top: 1px solid var(--border); background: var(--bg-900); }
.nexus-chat-input { flex: 1; background: var(--bg-700); border: 1px solid var(--border); border-radius: var(--radius-sm); color: var(--text-100); font-family: var(--font-sans); font-size: 13.5px; padding: 10px 14px; outline: none; transition: border-color var(--transition); }
.nexus-chat-input:focus { border-color: var(--accent); }
.nexus-code { font-family: var(--font-mono); background: var(--bg-900); border: 1px solid var(--border); border-radius: 6px; padding: 8px 12px; display: block; font-size: 12px; color: #a5f3fc; white-space: pre-wrap; margin: 8px 0; }

/* == Responsive ======================================================== */
@media (max-width: 900px) {
    .nexus-sidebar { position: fixed; left: 0; top: 0; bottom: 0; transform: translateX(-100%); }
    .nexus-sidebar-open { transform: translateX(0); }
    .nexus-topbar-toggle { display: flex; }
    .nexus-content { padding: 20px 16px; }
    .nexus-chat-layout { grid-template-columns: 1fr; }
    .nexus-chat-schema { max-height: 160px; }
    .nexus-fields-grid { grid-template-columns: 1fr; }
}
";
