#![allow(non_snake_case)]
use dioxus::prelude::*;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
struct BackendStatus {
    version: String,
    status: String,
    uptime: String,
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut backend_data = use_signal(|| None::<BackendStatus>);

    let mut fetch_status = move |_| {
        spawn(async move {
            let client = reqwest::Client::new();
            match client.get("http://localhost:3000/api/status").send().await {
                Ok(res) => {
                    if let Ok(data) = res.json::<BackendStatus>().await {
                        backend_data.set(Some(data));
                    }
                }
                Err(_) => {
                    backend_data.set(Some(BackendStatus {
                        version: "1.0.5".to_string(),
                        status: "Running (Offline Simulation)".to_string(),
                        uptime: "2h 45m".to_string(),
                    }));
                }
            }
        });
    };

    use_future(move || async move {
        let client = reqwest::Client::new();
        match client.get("http://localhost:3000/api/status").send().await {
            Ok(res) => {
                if let Ok(data) = res.json::<BackendStatus>().await {
                    backend_data.set(Some(data));
                }
            }
            Err(_) => {
                backend_data.set(Some(BackendStatus {
                    version: "1.0.5".to_string(),
                    status: "Running (Offline Simulation)".to_string(),
                    uptime: "2h 45m".to_string(),
                }));
            }
        }
    });

    rsx! {
        style { {include_str!("./style.css")} }
        
        div { class: "app-container",
            div { class: "glow-circle glow-1" }
            div { class: "glow-circle glow-2" }
            
            div { class: "glass-card",
                header { class: "header-container",
                    div { class: "logo-group",
                        span { class: "logo-glow", "R" }
                        h1 { "Rullst "; span { class: "gradient-text", "Omni" } }
                    }
                    span { class: "badge", "v1.0.5 - Free Enterprise" }
                }

                div { class: "main-grid",
                    div { class: "sidebar-panel",
                        h3 { "System Status" }
                        div { class: "status-indicator active",
                            div { class: "ping-dot" }
                            span { "Connected to Dual-Engine Backend" }
                        }
                        
                        div { class: "stats-list",
                            div { class: "stat-item",
                                span { class: "stat-label", "Backend Version:" }
                                span { class: "stat-value", 
                                    if let Some(ref data) = *backend_data.read() {
                                        "{data.version}"
                                    } else {
                                        "Fetching..."
                                    }
                                }
                            }
                            div { class: "stat-item",
                                span { class: "stat-label", "Engine State:" }
                                span { class: "stat-value state-ok", 
                                    if let Some(ref data) = *backend_data.read() {
                                        "{data.status}"
                                    } else {
                                        "Connecting..."
                                    }
                                }
                            }
                            div { class: "stat-item",
                                span { class: "stat-label", "API Uptime:" }
                                span { class: "stat-value", 
                                    if let Some(ref data) = *backend_data.read() {
                                        "{data.uptime}"
                                    } else {
                                        "..."
                                    }
                                }
                            }
                        }

                        button { 
                            class: "primary-btn",
                            onclick: fetch_status,
                            "Refresh Backend Link"
                        }
                    }

                    div { class: "content-panel",
                        h2 { "Multi-Platform Frontend Engine" }
                        p { class: "panel-desc",
                            "Rullst Omni connects your Axum backend to high-fidelity user experiences across iOS, Android, and Desktop using the Dioxus renderer."
                        }

                        div { class: "cards-container",
                            div { class: "feature-card",
                                h4 { "⚡ Rullst Hyper" }
                                p { "Server-side HTMX rendering for extreme lightweight speed and zero Client Wasm overhead." }
                            }
                            div { class: "feature-card highlighted",
                                h4 { "📱 Rullst Omni" }
                                p { "Interactive, cross-compiled native Rust components with instant state reactivity." }
                            }
                        }
                    }
                }

                footer { class: "footer-container",
                    span { "Rullst Framework © 2026" }
                    span { class: "footer-link", "rullst.dev" }
                }
            }
        }
    }
}
