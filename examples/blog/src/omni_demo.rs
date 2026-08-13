//! Omni-Channel (Mobile & Desktop) Simulation and Export Hub for Rullst.
//! Demonstrates native cross-platform capabilities: Android/iOS simulation and Desktop packaging.

use axum::response::{Html, IntoResponse};
use rullst::html;

use crate::showcase_nav::{render_shared_styles, render_showcase_nav};

/// Handler for the Omni-Channel page (`/omni`).
pub async fn omni_page() -> impl IntoResponse {
    let nav = render_showcase_nav("/omni");
    let styles = render_shared_styles();

    Html(html! {
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <title>"Rullst Omni - Mobile Simulator & Desktop App Exporter"</title>
                <link rel="icon" type="image/png" href="https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png" />
                <style>{ rullst::html::RawHtml(styles) }</style>
                <style>
                    r#"
                    .omni-grid {
                        display: grid;
                        grid-template-columns: 1fr 420px;
                        gap: 2rem;
                        align-items: start;
                    }
                    @media (max-width: 1024px) {
                        .omni-grid {
                            grid-template-columns: 1fr;
                        }
                    }
                    .phone-frame {
                        background: #0b0f19;
                        border: 12px solid #1e293b;
                        border-radius: 40px;
                        box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.7), 0 0 0 1px #334155;
                        overflow: hidden;
                        width: 360px;
                        height: 720px;
                        margin: 0 auto;
                        display: flex;
                        flex-direction: column;
                        position: relative;
                    }
                    .phone-notch {
                        background: #0f172a;
                        height: 28px;
                        display: flex;
                        align-items: center;
                        justify-content: space-between;
                        padding: 0 1.25rem;
                        font-size: 0.72rem;
                        color: #94a3b8;
                        border-bottom: 1px solid #1e293b;
                        user-select: none;
                        z-index: 10;
                    }
                    .phone-screen {
                        flex-grow: 1;
                        width: 100%;
                        border: none;
                        background: #07090e;
                    }
                    .phone-bar {
                        height: 18px;
                        background: #0f172a;
                        display: flex;
                        justify-content: center;
                        align-items: center;
                    }
                    .home-pill {
                        width: 100px;
                        height: 4px;
                        background: #475569;
                        border-radius: 9999px;
                    }
                    .platform-card {
                        background: #0d121f;
                        border: 1px solid #1e293b;
                        border-radius: 0.75rem;
                        padding: 1.5rem;
                        margin-bottom: 1.5rem;
                    }
                    .platform-badge {
                        display: inline-block;
                        font-size: 0.72rem;
                        font-weight: 700;
                        text-transform: uppercase;
                        padding: 0.2rem 0.5rem;
                        border-radius: 0.25rem;
                        margin-bottom: 0.75rem;
                    }
                    .badge-desktop {
                        background: rgba(56, 189, 248, 0.15);
                        color: #38bdf8;
                        border: 1px solid rgba(56, 189, 248, 0.3);
                    }
                    .badge-mobile {
                        background: rgba(16, 185, 129, 0.15);
                        color: #34d399;
                        border: 1px solid rgba(16, 185, 129, 0.3);
                    }
                    .badge-req {
                        background: rgba(245, 158, 11, 0.15);
                        color: #fbbf24;
                        border: 1px solid rgba(245, 158, 11, 0.3);
                    }
                    "#
                </style>
            </head>
            <body>
                { rullst::html::RawHtml(nav) }
                <div class="container">
                    <div class="card" style="margin-bottom: 1.5rem;">
                        <h1 class="card-title">
                            "📱 Rullst-Omni: Cross-Platform Native Exporter"
                            <span class="feature-tag tag-orm">"rullst-omni"</span>
                        </h1>
                        <p style="color: var(--text-muted); margin-bottom: 0;">
                            "Transform your Rullst Web application into native Desktop (Windows, Linux, macOS) and Mobile (Android, iOS) binaries with shared state, native notifications, and offline-first zero-bundle performance."
                        </p>
                    </div>

                    <div class="omni-grid">
                        <div>
                            <div class="platform-card">
                                <div style="display: flex; justify-content: space-between; align-items: center;">
                                    <span class="platform-badge badge-desktop">"🖥️ Desktop Target (Windows / Linux / macOS)"</span>
                                    <span style="font-size: 0.75rem; color: #10b981; font-weight: 600;">"Zero Heavy Dependencies"</span>
                                </div>
                                <h3 style="color: #fff; margin-top: 0.25rem; font-size: 1.2rem;">"Standalone Native Desktop Binary (Wry / Tauri Engine)"</h3>
                                <p style="color: #cbd5e1; font-size: 0.9rem; line-height: 1.5;">
                                    "Wraps the Rullst Tokio server and SSR UI in a high-performance native OS webview (Edge WebView2 on Windows, WebKitGTK on Linux, WebKit on macOS). Produces lightweight ~5MB binaries without the heavy memory footprint of Electron."
                                </p>

                                <div class="code-block" style="margin: 1rem 0;">
                                    "# 1-Click Desktop Packaging CLI:\n"
                                    "cargo rullst make:desktop --release\n"
                                    "# Output: target/release/bundle/rullst-blog.exe (or .deb / .AppImage)"
                                </div>

                                <div style="background: rgba(15, 23, 42, 0.8); border: 1px solid #1e293b; border-radius: 0.5rem; padding: 1rem; font-size: 0.85rem; color: #94a3b8;">
                                    <strong style="color: #38bdf8;">"Requirements: "</strong>
                                    "Standard Rust compiler (`cargo` / `rustc`). No extra IDEs or virtual machines required."
                                </div>
                            </div>

                            <div class="platform-card">
                                <div style="display: flex; justify-content: space-between; align-items: center;">
                                    <span class="platform-badge badge-mobile">"📱 Mobile Target (Android APK / iOS IPA)"</span>
                                    <span class="platform-badge badge-req">"SDK / NDK Setup"</span>
                                </div>
                                <h3 style="color: #fff; margin-top: 0.25rem; font-size: 1.2rem;">"Native Mobile Application (Android APK & iOS)"</h3>
                                <p style="color: #cbd5e1; font-size: 0.9rem; line-height: 1.5;">
                                    "Compiles native Rust system libraries via `cargo-apk` or `tauri-mobile`, embedding the local responsive interface with hardware bridge APIs (Camera, SQLite, Biometrics, Push Notifications)."
                                </p>

                                <div class="code-block" style="margin: 1rem 0;">
                                    "# 1. Add Android Compilation Targets:\n"
                                    "rustup target add aarch64-linux-android armv7-linux-androideabi\n\n"
                                    "# 2. Build Release APK:\n"
                                    "cargo rullst make:mobile --platform android --apk\n"
                                    "# Output: target/mobile/android/app-release.apk"
                                </div>

                                <div style="background: rgba(15, 23, 42, 0.8); border: 1px solid #1e293b; border-radius: 0.5rem; padding: 1rem; font-size: 0.85rem; color: #94a3b8;">
                                    <strong style="color: #f59e0b;">"Do I need Android Studio installed? "</strong><br />
                                    "Not the full heavy Android Studio GUI! You only need the "
                                    <strong style="color: #fff;">"Android Command Line Tools (`sdkmanager` + NDK)"</strong>
                                    " or `ANDROID_HOME` configured on your machine so the Rust compiler can cross-compile and sign the APK package."
                                </div>
                            </div>
                        </div>

                        <div>
                            <div style="text-align: center; margin-bottom: 0.75rem;">
                                <span style="font-size: 0.85rem; color: #38bdf8; font-weight: 600;">
                                    "📱 Live Mobile Viewport Simulator"
                                </span>
                                <p style="color: #64748b; font-size: 0.75rem; margin: 0.2rem 0 0 0;">
                                    "Interactive live preview of the blog running at localhost:3000"
                                </p>
                            </div>

                            <div class="phone-frame">
                                <div class="phone-notch">
                                    <span>"9:41"</span>
                                    <span style="display: flex; gap: 0.35rem; align-items: center;">
                                        "5G"
                                        <span>"📶"</span>
                                        <span>"🔋"</span>
                                    </span>
                                </div>

                                <iframe src="/" class="phone-screen" title="Mobile Simulator"></iframe>

                                <div class="phone-bar">
                                    <div class="home-pill"></div>
                                </div>
                            </div>

                            <div style="display: flex; justify-content: center; gap: 0.5rem; margin-top: 1rem;">
                                <button onclick="document.querySelector('.phone-screen').src = '/'" class="btn" style="padding: 0.4rem 0.8rem; font-size: 0.8rem;">
                                    "🔄 Reload App"
                                </button>
                                <button onclick="document.querySelector('.phone-screen').src = '/editor'" class="btn" style="padding: 0.4rem 0.8rem; font-size: 0.8rem; background: #0284c7;">
                                    "🏝️ Wasm Island"
                                </button>
                                <button onclick="document.querySelector('.phone-screen').src = '/live-feed'" class="btn" style="padding: 0.4rem 0.8rem; font-size: 0.8rem; background: #e11d48;">
                                    "🔴 LiveView"
                                </button>
                            </div>
                        </div>
                    </div>
                </div>
            </body>
        </html>
    })
}
