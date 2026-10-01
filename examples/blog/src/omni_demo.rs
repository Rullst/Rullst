//! Omni-Channel (Mobile & Desktop) export guide for Rullst.
//! Points to the CLI-generated, CI-tested Tauri shells for desktop and mobile.

use axum::response::{Html, IntoResponse};
use rullst::html;

use crate::showcase_nav::{render_head_assets, render_showcase_nav};

/// Handler for the Omni-Channel page (`/omni`).
pub async fn omni_page() -> impl IntoResponse {
    let nav = render_showcase_nav("/omni");
    let head_assets = render_head_assets();

    Html(html! {
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <title>"Rullst Omni - Desktop and Mobile Shells"</title>
                { rullst::html::RawHtml(head_assets) }
            </head>
            <body>
                { rullst::html::RawHtml(nav) }
                <div class="container">
                    <div class="card">
                        <h1 class="card-title">
                            "📱 Rullst-Omni: Cross-Platform Native Exporter"
                            <span class="feature-tag tag-orm">"rullst-omni"</span>
                        </h1>
                        <p class="lead flush">
                            "Generate a Tauri development shell for a responsive Rullst application on desktop, Android, or iOS. Native plugins, offline synchronization, signing, store publication, and release artifacts remain explicit application work."
                        </p>
                    </div>

                    <div class="omni-grid">
                        <div class="platform-card">
                            <div class="platform-card-header">
                                <span class="platform-badge badge-desktop">"🖥️ Desktop Target (Windows / Linux / macOS)"</span>
                                <span class="platform-engine">"Tauri System WebView"</span>
                            </div>
                            <h3>"Standalone Native Desktop Binary (Wry / Tauri Engine)"</h3>
                            <p>
                                "The generated desktop shell can start the Rullst backend and open its responsive UI in the operating system webview. Final size, memory use, installers, updates, and platform dependencies must be measured on each release target."
                            </p>

                            <div class="code-block">
                                "# Generate the Tauri shell:\n"
                                "cargo rullst make:omni\n\n"
                                "# Run the desktop development client:\n"
                                "cargo rullst omni desktop"
                            </div>

                            <div class="platform-note">
                                <strong>"Requirements: "</strong>
                                "Rust, Tauri prerequisites, the platform webview, and either cargo-tauri or npm/npx."
                            </div>
                        </div>

                        <div class="platform-card">
                            <div class="platform-card-header">
                                <span class="platform-badge badge-mobile">"📱 Mobile Target (Android APK / iOS IPA)"</span>
                                <span class="platform-badge badge-req">"SDK / NDK Setup"</span>
                            </div>
                            <h3>"Native Mobile Application (Android APK & iOS)"</h3>
                            <p>
                                "Initializes and runs the Tauri mobile development target around the responsive web interface. Camera, SQLite, biometrics, push notifications, secure offline storage, deep links, and background playback require reviewed Tauri plugins and application code."
                            </p>

                            <div class="code-block">
                                "# 1. Generate and select Android during setup:\n"
                                "cargo rullst make:omni\n\n"
                                "# 2. Run the Android development client:\n"
                                "cargo rullst omni android\n\n"
                                "# Release signing/build remains a Tauri/Android gate."
                            </div>

                            <div class="platform-note">
                                <strong class="warn">"Do I need Android Studio installed? "</strong><br />
                                "Use the Android SDK, NDK, Java toolchain, platform tools, and a configured emulator/device. A release also needs application identity, signing keys, network policy, store metadata, and Play Console validation."
                            </div>
                        </div>
                    </div>

                    <div class="card">
                        <h2 class="card-title">"📐 Checking the responsive layout"</h2>
                        <p class="muted">
                            "The production Content Security Policy sends frame-ancestors 'none' and X-Frame-Options: DENY, so this showcase no longer embeds itself in a simulated phone frame. Open any page below with your browser's device toolbar, or run the generated mobile shell on an emulator or device."
                        </p>
                        <div class="link-list">
                            <a href="/" class="btn">"Landing page"</a>
                            <a href="/live-feed" class="btn">"LiveView"</a>
                            <a href="/pricing" class="btn">"Pricing"</a>
                        </div>
                    </div>
                </div>
            </body>
        </html>
    })
}
