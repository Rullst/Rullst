use axum::{Router, response::Html, routing::get};

pub fn router() -> Router {
    Router::new().route("/", get(render_env_viewer))
}

async fn render_env_viewer() -> Html<String> {
    let mut vars: Vec<(String, String)> = std::env::vars().collect();
    vars.sort_by(|a, b| a.0.cmp(&b.0));

    let mut rows = String::new();
    for (key, val) in vars {
        let is_sensitive = key.contains("SECRET")
            || key.contains("PASSWORD")
            || key.contains("TOKEN")
            || key.contains("KEY")
            || key.contains("AUTH");

        let display_val = if is_sensitive {
            if val.len() > 6 {
                format!("{}••••••••", &val[0..4])
            } else {
                "••••••••".to_string()
            }
        } else {
            val
        };

        let val_html = rullst_core::html::escape_str(&display_val);

        rows.push_str(&format!(
            "<tr class=\"border-b border-slate-800 hover:bg-slate-800/50 transition-colors\">\
             <td class=\"py-3 px-4 font-semibold text-emerald-400\">{}</td>\
             <td class=\"py-3 px-4 text-slate-300 break-all\">{}</td>\
             </tr>",
            rullst_core::html::escape_str(&key),
            val_html
        ));
    }

    Html(format!(
        r#"<!DOCTYPE html>
<html lang="en" class="h-full bg-slate-950 text-slate-100">
<head>
    <meta charset="UTF-8">
    <title>Environment Viewer - Rullst Studio</title>
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <script src="https://cdn.tailwindcss.com"></script>
</head>
<body class="h-full flex flex-col font-mono p-8">
    <div class="max-w-6xl mx-auto w-full">
        <div class="flex items-center justify-between mb-8">
            <h1 class="text-3xl font-bold text-emerald-400 flex items-center gap-3">
                <a href="/studio" class="text-slate-500 hover:text-emerald-400 transition-colors">←</a>
                Environment Viewer
            </h1>
        </div>
        
        <div class="bg-slate-900 border border-slate-800 rounded-xl overflow-hidden shadow-2xl">
            <table class="w-full text-left border-collapse">
                <thead>
                    <tr class="bg-slate-950/50 border-b border-slate-800">
                        <th class="py-3 px-4 text-slate-400 font-medium">Variable Name</th>
                        <th class="py-3 px-4 text-slate-400 font-medium">Value</th>
                    </tr>
                </thead>
                <tbody>
                    {}
                </tbody>
            </table>
        </div>
    </div>
</body>
</html>"#,
        rows
    ))
}
