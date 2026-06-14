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

async fn fetch_backend_status() -> BackendStatus {
    let client = reqwest::Client::new();
    if let Ok(res) = client.get("http://localhost:3000/api/status").send().await {
        if let Ok(data) = res.json::<BackendStatus>().await {
            return data;
        }
    }
    BackendStatus {
        version: "1.0.5".to_string(),
        status: "Running (Offline Simulation)".to_string(),
        uptime: "2h 45m".to_string(),
    }
}

#[component]
fn Sidebar(backend_data: Signal<Option<BackendStatus>>, on_refresh: EventHandler<dioxus::events::MouseEvent>) -> Element {
    rsx! {
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
                onclick: move |evt| on_refresh.call(evt),
                "Refresh Backend Link"
            }
        }
    }
}

#[component]
fn App() -> Element {
    let mut backend_data = use_signal(|| None::<BackendStatus>);

    let fetch_status = move |_| {
        spawn(async move {
            backend_data.set(Some(fetch_backend_status().await));
        });
    };

    use_future(move || async move {
        backend_data.set(Some(fetch_backend_status().await));
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
                    Sidebar { backend_data, on_refresh: fetch_status }

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
