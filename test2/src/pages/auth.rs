use rullst::html;
use axum::response::Html;

const PASSKEY_SCRIPT: &str = r#"<script>
    function bufferDecode(value) {
        const base64 = value.replace(/-/g, "+").replace(/_/g, "/");
        const pad = base64.length % 4;
        const padded = pad ? base64 + "=".repeat(4 - pad) : base64;
        const binary = window.atob(padded);
        const bytes = new Uint8Array(binary.length);
        for (let i = 0; i < binary.length; i++) {
            bytes[i] = binary.charCodeAt(i);
        }
        return bytes.buffer;
    }
    function bufferEncode(value) {
        const bytes = new Uint8Array(value);
        let binary = "";
        for (let i = 0; i < bytes.byteLength; i++) {
            binary += String.fromCharCode(bytes[i]);
        }
        const base64 = window.btoa(binary);
        return base64.replace(/\+/g, "-").replace(/\//g, "_").replace(/=/g, "");
    }
    document.addEventListener("DOMContentLoaded", () => {
        if (window.PublicKeyCredential) {
            document.querySelectorAll(".btn-passkey").forEach(btn => btn.style.display = "flex");
        }
    });
    async function registerPasskey() {
        try {
            const email = document.getElementById("email").value;
            const name = document.getElementById("name").value;
            if (!email || !name) { alert("Nome e email sao obrigatorios"); return; }
            const res = await fetch("/auth/passkey/register/start", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ email, name })
            });
            if (!res.ok) throw new Error(await res.text());
            const options = await res.json();
            options.publicKey.challenge = bufferDecode(options.publicKey.challenge);
            options.publicKey.user.id = bufferDecode(options.publicKey.user.id);
            const credential = await navigator.credentials.create({ publicKey: options.publicKey });
            const credentialJson = {
                id: credential.id,
                rawId: bufferEncode(credential.rawId),
                type: credential.type,
                response: {
                    attestationObject: bufferEncode(credential.response.attestationObject),
                    clientDataJSON: bufferEncode(credential.response.clientDataJSON),
                    transports: credential.response.getTransports ? credential.response.getTransports() : []
                }
            };
            const finishRes = await fetch("/auth/passkey/register/finish?email=" + encodeURIComponent(email), {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify(credentialJson)
            });
            if (finishRes.ok) { window.location.href = "/dashboard"; } else { alert("Erro: " + await finishRes.text()); }
        } catch (err) { alert("Erro: " + err.message); }
    }
    async function loginPasskey() {
        try {
            const email = document.getElementById("email").value;
            if (!email) { alert("Email obrigatorio"); return; }
            const res = await fetch("/auth/passkey/login/start", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ email })
            });
            if (!res.ok) throw new Error(await res.text());
            const options = await res.json();
            options.publicKey.challenge = bufferDecode(options.publicKey.challenge);
            if (options.publicKey.allowCredentials) {
                for (let cred of options.publicKey.allowCredentials) { cred.id = bufferDecode(cred.id); }
            }
            const credential = await navigator.credentials.get({ publicKey: options.publicKey });
            const credentialJson = {
                id: credential.id,
                rawId: bufferEncode(credential.rawId),
                type: credential.type,
                response: {
                    authenticatorData: bufferEncode(credential.response.authenticatorData),
                    clientDataJSON: bufferEncode(credential.response.clientDataJSON),
                    signature: bufferEncode(credential.response.signature),
                    userHandle: credential.response.userHandle ? bufferEncode(credential.response.userHandle) : null
                }
            };
            const finishRes = await fetch("/auth/passkey/login/finish?email=" + encodeURIComponent(email), {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify(credentialJson)
            });
            if (finishRes.ok) { window.location.href = "/dashboard"; } else { alert("Erro: " + await finishRes.text()); }
        } catch (err) { alert("Erro: " + err.message); }
    }
</script>"#;

pub fn login_page(csrf_token: &str, error: Option<&str>) -> Html<String> {
    let error_html = error.map(|err| html! {
        <div style="background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.2); color: #f87171; padding: 0.75rem 1rem; border-radius: 0.5rem; margin-bottom: 1.5rem; font-size: 0.9rem; text-align: left;">
            {err}
        </div>
    }).unwrap_or_default();

    Html(html! {
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <title>"Login - Rullst"</title>
                <style>
                    "
                    body { background-color: #0b0f19; color: #f1f5f9; font-family: system-ui, sans-serif; display: flex; align-items: center; justify-content: center; min-height: 100vh; margin: 0; }
                    .card { background: #111827; border: 1px solid #1f2937; border-radius: 1rem; padding: 2.5rem; width: 100%; max-width: 420px; box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.5); text-align: center; }
                    h1 { font-size: 2rem; margin: 0 0 0.5rem 0; background: linear-gradient(135deg, #38bdf8, #818cf8); -webkit-background-clip: text; -webkit-text-fill-color: transparent; }
                    .form-group { margin-bottom: 1.25rem; text-align: left; }
                    label { display: block; font-size: 0.85rem; color: #94a3b8; margin-bottom: 0.5rem; }
                    input { width: 100%; box-sizing: border-box; background: #1f2937; border: 1px solid #374151; border-radius: 0.5rem; padding: 0.75rem 1rem; color: #fff; }
                    button.btn-primary { width: 100%; background: linear-gradient(135deg, #6366f1, #4f46e5); color: #fff; border: none; border-radius: 0.5rem; padding: 0.85rem; font-weight: 600; cursor: pointer; }
                    .oauth-btn { width: 100%; background: #1f2937; color: #fff; border: 1px solid #374151; border-radius: 0.5rem; padding: 0.75rem; font-size: 0.9rem; cursor: pointer; display: flex; align-items: center; justify-content: center; margin-top: 1rem; }
                    "
                </style>
            </head>
            <body>
                <div class="card">
                    <h1>"Welcome Back"</h1>
                    { rullst::html::RawHtml(error_html) }
                    <form method="post" action="/login">
                        <input type="hidden" name="_token" value={csrf_token} />
                        <div class="form-group">
                            <label>"Email"</label>
                            <input type="email" id="email" name="email" required="required" />
                        </div>
                        <div class="form-group">
                            <label>"Password"</label>
                            <input type="password" id="password" name="password" required="required" />
                        </div>
                        <button type="submit" class="btn-primary">"Sign In"</button>
                    </form>
                </div>
            </body>
        </html>
    })
}

pub fn register_page(csrf_token: &str, error: Option<&str>) -> Html<String> {
    let error_html = error.map(|err| html! {
        <div style="background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.2); color: #f87171; padding: 0.75rem 1rem; border-radius: 0.5rem; margin-bottom: 1.5rem; font-size: 0.9rem; text-align: left;">
            {err}
        </div>
    }).unwrap_or_default();

    Html(html! {
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <title>"Register - Rullst"</title>
                <style>
                    "
                    body { background-color: #0b0f19; color: #f1f5f9; font-family: system-ui, sans-serif; display: flex; align-items: center; justify-content: center; min-height: 100vh; margin: 0; }
                    .card { background: #111827; border: 1px solid #1f2937; border-radius: 1rem; padding: 2.5rem; width: 100%; max-width: 420px; box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.5); text-align: center; }
                    h1 { font-size: 2rem; margin: 0 0 0.5rem 0; background: linear-gradient(135deg, #38bdf8, #818cf8); -webkit-background-clip: text; -webkit-text-fill-color: transparent; }
                    .form-group { margin-bottom: 1.25rem; text-align: left; }
                    label { display: block; font-size: 0.85rem; color: #94a3b8; margin-bottom: 0.5rem; }
                    input { width: 100%; box-sizing: border-box; background: #1f2937; border: 1px solid #374151; border-radius: 0.5rem; padding: 0.75rem 1rem; color: #fff; }
                    button.btn-primary { width: 100%; background: linear-gradient(135deg, #6366f1, #4f46e5); color: #fff; border: none; border-radius: 0.5rem; padding: 0.85rem; font-weight: 600; cursor: pointer; }
                    "
                </style>
            </head>
            <body>
                <div class="card">
                    <h1>"Create Account"</h1>
                    { rullst::html::RawHtml(error_html) }
                    <form method="post" action="/register">
                        <input type="hidden" name="_token" value={csrf_token} />
                        <div class="form-group">
                            <label>"Full Name"</label>
                            <input type="text" id="name" name="name" required="required" />
                        </div>
                        <div class="form-group">
                            <label>"Email"</label>
                            <input type="email" id="email" name="email" required="required" />
                        </div>
                        <div class="form-group">
                            <label>"Password"</label>
                            <input type="password" id="password" name="password" required="required" />
                        </div>
                        <button type="submit" class="btn-primary">"Sign Up"</button>
                    </form>
                </div>
            </body>
        </html>
    })
}

pub fn dashboard_page(user_name: &str) -> Html<String> {
    Html(html! {
        <html>
            <head>
                <title>"Dashboard - Rullst"</title>
                <style>
                    "
                    body { background-color: #0b0f19; color: #f1f5f9; font-family: system-ui, sans-serif; padding: 4rem; text-align: center; }
                    .container { max-width: 600px; margin: 0 auto; background: #111827; padding: 3rem; border-radius: 1rem; border: 1px solid #1f2937; }
                    h1 { font-size: 2.5rem; background: linear-gradient(135deg, #38bdf8, #818cf8); -webkit-background-clip: text; -webkit-text-fill-color: transparent; }
                    .btn-logout { display: inline-block; background: linear-gradient(135deg, #ef4444, #dc2626); color: white; padding: 0.75rem 2rem; border-radius: 0.5rem; text-decoration: none; margin-top: 2rem; }
                    "
                </style>
            </head>
            <body>
                <div class="container">
                    <h1>"Hello, " {user_name} "!"</h1>
                    <p>"Welcome to your secure Rullst SaaS Dashboard."</p>
                    <a href="/logout" class="btn-logout">"Sign Out"</a>
                </div>
            </body>
        </html>
    })
}
