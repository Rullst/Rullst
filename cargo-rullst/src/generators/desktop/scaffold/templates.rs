mod runtime;

use super::OmniIdentity;
use runtime::render_runtime;
use serde_json::json;
use std::fs;
use std::path::Path;

pub(super) fn write_omni_files(
    omni_dir: &Path,
    src_dir: &Path,
    backend_url: &str,
    identity: &OmniIdentity,
) -> Result<(), Box<dyn std::error::Error>> {
    let parsed_backend = reqwest::Url::parse(backend_url)?;
    let backend_origin = parsed_backend.origin().ascii_serialization();
    let backend_literal = serde_json::to_string(backend_url)?;

    fs::write(src_dir.join("index.html"), bootstrap_html())?;
    fs::write(src_dir.join("styles.css"), bootstrap_css())?;
    fs::write(
        src_dir.join("redirect.js"),
        format!(
            r#"const backendUrl = {backend_literal};
const status = document.querySelector("[data-status]");
const retry = document.querySelector("[data-retry]");

function openBackend() {{
  retry.disabled = true;
  status.textContent = "Opening the secure web application…";
  window.location.assign(backendUrl);
}}

retry.addEventListener("click", openBackend);
window.addEventListener("offline", () => {{
  retry.disabled = false;
  status.textContent = "This device is offline. Reconnect and try again.";
}});
window.addEventListener("online", openBackend);

if (navigator.onLine) {{
  openBackend();
}} else {{
  retry.disabled = false;
  status.textContent = "This device is offline. Reconnect and try again.";
}}
"#,
        ),
    )?;

    let package_json = json!({
        "name": "rullst-omni",
        "version": identity.version,
        "private": true,
        "scripts": { "tauri": "tauri" },
        "devDependencies": { "@tauri-apps/cli": "2.11.4" }
    });
    fs::write(
        omni_dir.join("package.json"),
        format!("{}\n", serde_json::to_string_pretty(&package_json)?),
    )?;

    fs::write(
        omni_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "rullst-omni"
version = "{}"
description = "Rullst Omni application shell"
authors = ["Rullst Developer"]
edition = "2021"

[lib]
name = "rullst_omni"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = {{ version = "=2.6.3", features = [] }}

[dependencies]
tauri = {{ version = "=2.11.5", features = [] }}

[workspace]
"#,
            identity.version
        ),
    )?;

    let csp = format!(
        "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' asset: http://asset.localhost blob: data:; connect-src 'self' ipc: http://ipc.localhost {backend_origin}; object-src 'none'; base-uri 'none'; frame-src 'none'"
    );
    let tauri_config = json!({
        "$schema": "https://schema.tauri.app/config/2",
        "productName": identity.product_name,
        "version": identity.version,
        "identifier": identity.identifier,
        "build": { "frontendDist": "src" },
        "app": {
            "windows": [{
                "title": identity.product_name,
                "width": 1024,
                "height": 768,
                "resizable": true
            }],
            "security": { "csp": csp }
        },
        "bundle": {
            "active": true,
            "targets": "all",
            "icon": [
                "icons/32x32.png",
                "icons/128x128.png",
                "icons/128x128@2x.png",
                "icons/icon.icns",
                "icons/icon.ico"
            ]
        }
    });
    fs::write(
        omni_dir.join("tauri.conf.json"),
        format!("{}\n", serde_json::to_string_pretty(&tauri_config)?),
    )?;

    fs::write(
        omni_dir.join("build.rs"),
        "fn main() {\n    tauri_build::build();\n}\n",
    )?;
    fs::write(src_dir.join("lib.rs"), render_runtime(&parsed_backend)?)?;
    fs::write(
        src_dir.join("main.rs"),
        "#![cfg_attr(not(debug_assertions), windows_subsystem = \"windows\")]\n\nfn main() {\n    rullst_omni::run();\n}\n",
    )?;
    fs::write(
        omni_dir.join("README.md"),
        generated_readme(identity, backend_url),
    )?;

    Ok(())
}

fn bootstrap_html() -> &'static str {
    r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta name="color-scheme" content="dark">
    <title>Starting Rullst Omni</title>
    <link rel="stylesheet" href="styles.css">
    <script defer src="redirect.js"></script>
  </head>
  <body>
    <main aria-live="polite">
      <span class="mark" aria-hidden="true">R</span>
      <h1>Starting your application</h1>
      <p data-status>Checking the connection…</p>
      <button type="button" data-retry disabled>Try again</button>
      <noscript>JavaScript is required to open this application.</noscript>
    </main>
  </body>
</html>
"#
}

fn bootstrap_css() -> &'static str {
    r#":root { font-family: system-ui, sans-serif; color: #e2e8f0; background: #020617; }
* { box-sizing: border-box; }
body { min-height: 100vh; margin: 0; display: grid; place-items: center; padding: 2rem; }
main { width: min(32rem, 100%); padding: 2rem; text-align: center; border: 1px solid #334155; border-radius: 1.5rem; background: #0f172acc; box-shadow: 0 1.5rem 5rem #0008; }
.mark { display: inline-grid; place-items: center; width: 4rem; height: 4rem; border-radius: 1rem; color: white; font-size: 2rem; font-weight: 800; background: linear-gradient(135deg, #2563eb, #7c3aed); }
h1 { margin: 1.25rem 0 .5rem; font-size: clamp(1.5rem, 6vw, 2rem); }
p, noscript { color: #94a3b8; }
button { margin-top: 1rem; padding: .75rem 1.25rem; color: white; border: 0; border-radius: .75rem; background: #2563eb; font: inherit; font-weight: 700; cursor: pointer; }
button:disabled { opacity: .45; cursor: wait; }
button:focus-visible { outline: .2rem solid #93c5fd; outline-offset: .2rem; }
"#
}

fn generated_readme(identity: &OmniIdentity, backend_url: &str) -> String {
    let identifier_note = if identity.uses_placeholder_identifier {
        "The generated desktop identifier uses the `com.example` development namespace. Replace it with an application-owned identifier before distribution."
    } else {
        "The scaffold uses the application-owned identifier supplied to `make:omni`."
    };
    format!(
        r#"# {} — Rullst Omni web shell

This directory packages the canonical Rullst web application at `{backend_url}`
for desktop, Android and iOS through Tauri. The local bootstrap contains no
application data and exposes no Tauri IPC API to the remote page. Native-side
navigation accepts only the packaged bootstrap and the exact backend origin.

{identifier_note}

## Run locally

```bash
cargo rullst omni desktop
cargo rullst omni android
cargo rullst omni ios
```

Android requires its SDK/NDK; iOS requires macOS and Xcode. Distributable apps
must use an HTTPS endpoint reachable from the real device. The web backend
remains responsible for authentication, authorization, CSP, CSRF and data.

## Intentional security boundary

- Cross-origin navigation is blocked. OAuth and external links must use a
  reviewed system-browser/deep-link integration instead of weakening the
  navigation allowlist.
- Remote web content receives no privileged Tauri commands by default.
- Offline synchronization, push, biometrics and secure device storage are not
  implied by this web-shell profile; add and test only the capabilities used.

## Distribution checklist

1. confirm the application-owned identifier, version, icons and product metadata;
2. configure platform signing, provisioning and privacy/usage declarations;
3. test the production HTTPS backend and authentication on physical devices;
4. complete accessibility, offline/error and data-retention testing;
5. use the platform beta channel before production review and publication.

The generated shell and CI compilation are packaging evidence, not proof of
App Store or Play acceptance. Signing credentials, privacy answers, native
capability policy, store publication and review remain application-owned.
"#,
        identity.product_name
    )
}

pub(super) fn generate_icon_source(icons_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(
        icons_dir.join("icon.svg"),
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024">
  <defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop stop-color="#0f172a"/><stop offset="1" stop-color="#2563eb"/></linearGradient></defs>
  <rect width="1024" height="1024" rx="224" fill="url(#g)"/>
  <path d="M280 760V264h244c142 0 230 70 230 190 0 79-43 139-115 168l137 138H610L493 642h-67v118H280zm146-244h91c58 0 91-21 91-62s-33-62-91-62h-91v124z" fill="#f8fafc"/>
</svg>
"##,
    )?;
    Ok(())
}
