"""Generate HTML panels (features, architecture, demo frames) for PNG/JPEG rendering."""
import pathlib
import sys

FONTS = ('<link rel="preconnect" href="https://fonts.googleapis.com">'
         '<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&family=JetBrains+Mono:wght@500;700&display=block" rel="stylesheet">')

THEME = {
    "dark": dict(bg="#070b16", bg2="#0d1428", panel="rgba(17,24,39,.78)", border="rgba(148,163,184,.16)",
                 title="#f8fafc", text="#cbd5e1", muted="#94a3b8", chip="rgba(255,255,255,.06)",
                 grid="rgba(148,163,184,.07)", glow=".38"),
    "light": dict(bg="#ffffff", bg2="#f1f5f9", panel="rgba(255,255,255,.92)", border="rgba(15,23,42,.10)",
                  title="#0f172a", text="#334155", muted="#64748b", chip="rgba(15,23,42,.05)",
                  grid="rgba(15,23,42,.05)", glow=".20"),
}

BASE_CSS = """
*{{box-sizing:border-box;margin:0;padding:0}}
html,body{{width:{w}px;height:{h}px;overflow:hidden}}
body{{font-family:Inter,system-ui,sans-serif;color:{text};
 background:radial-gradient(900px 500px at 8% 0%,rgba(255,106,26,{glow}),transparent 60%),
 radial-gradient(900px 600px at 100% 100%,rgba(34,197,94,{glow}),transparent 60%),
 radial-gradient(700px 400px at 70% 0%,rgba(59,130,246,{glow}),transparent 60%),
 linear-gradient(135deg,{bg},{bg2});position:relative}}
body:before{{content:"";position:absolute;inset:0;
 background-image:linear-gradient({grid} 1px,transparent 1px),linear-gradient(90deg,{grid} 1px,transparent 1px);
 background-size:44px 44px;-webkit-mask-image:radial-gradient(circle at 50% 45%,#000 30%,transparent 80%)}}
.wrap{{position:relative;padding:56px 64px}}
.eyebrow{{font-weight:700;letter-spacing:.32em;font-size:15px;text-transform:uppercase;color:#fb923c}}
h2{{font-size:44px;font-weight:800;color:{title};letter-spacing:-.02em;margin:10px 0 34px}}
h2 span{{background:linear-gradient(90deg,#ff6a1a,#f59e0b 50%,#22c55e);-webkit-background-clip:text;color:transparent}}
"""

FEATURES = [
    ("🔐", "Secure by default", "Argon2id, sessions, passkeys, OAuth2/OIDC, CSRF, strict headers, WAF and login jail.", "#f97316"),
    ("🗄️", "Data that stays correct", "Active Record, transactions, migrations and an outbox on SQLite, PostgreSQL and MySQL.", "#22c55e"),
    ("🧱", "Six real blueprints", "API, Blog, SaaS, LMS, Portfolio and ERP — generated as ordinary Rust you own.", "#eab308"),
    ("🤖", "Made for AI coding", "Explicit APIs, compile-time macros, typed errors and no runtime reflection.", "#a855f7"),
    ("💳", "Payments &amp; email", "Stripe billing with signed webhooks; Resend, SendGrid, Postmark and SMTP delivery.", "#14b8a6"),
    ("🧠", "AI built in", "OpenAI, Claude, Gemini, DeepSeek and Ollama with prompt-injection filtering and PII masking.", "#ec4899"),
    ("📊", "See inside your app", "Studio's live runtime telemetry and the Nexus admin with a security radar.", "#3b82f6"),
    ("🖥️", "Web first, native too", "HTMX server rendering, JSON APIs, and Tauri desktop and mobile shells via Omni.", "#06b6d4"),
]


def page(theme: str, w: int, h: int, css: str, body: str) -> str:
    t = THEME[theme]
    return (f"<!doctype html><html><head><meta charset=utf-8>{FONTS}<style>"
            + BASE_CSS.format(w=w, h=h, **t) + css.format(**t) + f"</style></head><body>{body}</body></html>")


def features(theme: str) -> str:
    cards = "".join(
        f'<div class="card" style="--c:{c}"><div class="icon">{i}</div><h3>{title}</h3><p>{desc}</p></div>'
        for i, title, desc, c in FEATURES
    )
    css = """
.grid{{display:grid;grid-template-columns:repeat(4,1fr);gap:22px}}
.card{{background:{panel};border:1px solid {border};border-radius:22px;padding:28px 26px 30px;position:relative;overflow:hidden}}
.card:before{{content:"";position:absolute;inset:0 0 auto 0;height:3px;background:linear-gradient(90deg,var(--c),transparent)}}
.card:after{{content:"";position:absolute;width:220px;height:220px;right:-90px;top:-90px;border-radius:50%;
 background:radial-gradient(var(--c),transparent 70%);opacity:.18}}
.icon{{width:58px;height:58px;border-radius:16px;display:grid;place-items:center;font-size:30px;
 background:linear-gradient(135deg,color-mix(in srgb,var(--c) 30%,transparent),color-mix(in srgb,var(--c) 8%,transparent));
 border:1px solid color-mix(in srgb,var(--c) 45%,transparent);margin-bottom:20px}}
h3{{font-size:23px;font-weight:700;color:{title};margin-bottom:10px;letter-spacing:-.01em}}
p{{font-size:17px;line-height:1.5;color:{muted}}}
"""
    body = ('<div class="wrap"><div class="eyebrow">Everything a real product needs</div>'
            '<h2>One framework. <span>The whole product.</span></h2>'
            f'<div class="grid">{cards}</div></div>')
    return page(theme, 1600, 760, css, body)


LAYERS = [
    ("Product", "#14b8a6", [("Payments", "rullst-capital"), ("Email", "rullst-mail"), ("AI &amp; RAG", "rullst-ai"),
                            ("Queues &amp; messaging", "rullst-messaging"), ("OAuth2 / OIDC", "rullst-connect")]),
    ("Trust", "#f97316", [("Identity &amp; sessions", "rullst-auth"), ("WAF · headers · CSRF · RASP", "rullst-security")]),
    ("Data", "#22c55e", [("Active Record · migrations · transactions", "rullst-orm"), ("Typed model macros", "rullst-orm-macros")]),
    ("Runtime", "#3b82f6", [("HTTP runtime · routing · lifecycle", "rullst-core"), ("html! and app macros", "rullst-macros")]),
]
TOOLS = [("CLI &amp; generators", "cargo-rullst", "🛠️"), ("Developer control room", "rullst-studio", "📊"),
         ("Admin &amp; security radar", "rullst-nexus", "🛡️")]
APPS = ["API", "Blog", "SaaS", "LMS", "Portfolio", "ERP"]


def architecture(theme: str) -> str:
    rows = []
    for name, color, boxes in LAYERS:
        items = "".join(f'<div class="box"><b>{label}</b><code>{crate}</code></div>' for label, crate in boxes)
        rows.append(f'<div class="layer" style="--c:{color}"><div class="lname">{name}</div><div class="boxes">{items}</div></div>')
    tools = "".join(f'<div class="tool"><span>{icon}</span><div><b>{label}</b><code>{crate}</code></div></div>'
                    for label, crate, icon in TOOLS)
    apps = "".join(f'<span class="app">{a}</span>' for a in APPS)
    css = """
.main{{display:grid;grid-template-columns:1fr 330px;gap:24px}}
.stack{{display:flex;flex-direction:column;gap:14px}}
.appbar{{border-radius:20px;padding:20px 24px;border:1px dashed color-mix(in srgb,#fb923c 55%,transparent);
 background:linear-gradient(90deg,rgba(255,106,26,.12),rgba(34,197,94,.10));display:flex;align-items:center;gap:14px}}
.appbar .lname{{color:{title}}}
.app{{padding:9px 16px;border-radius:999px;background:{chip};border:1px solid {border};font-weight:600;font-size:16px;color:{title}}}
.layer{{display:flex;align-items:stretch;gap:14px;border-radius:20px;padding:14px;background:{panel};border:1px solid {border};
 box-shadow:inset 4px 0 0 var(--c)}}
.lname{{width:150px;flex:none;display:flex;align-items:center;padding-left:12px;font-weight:800;font-size:19px;color:var(--c);letter-spacing:.02em}}
.boxes{{display:flex;gap:12px;flex:1}}
.box{{flex:1;border-radius:14px;padding:14px 16px;background:color-mix(in srgb,var(--c) 10%,transparent);
 border:1px solid color-mix(in srgb,var(--c) 30%,transparent)}}
.box b{{display:block;font-size:17px;color:{title};font-weight:650;margin-bottom:6px}}
code{{font-family:'JetBrains Mono',monospace;font-size:14px;color:{muted}}}
.base{{border-radius:20px;padding:18px 24px;text-align:center;font-weight:700;font-size:19px;color:{title};
 background:linear-gradient(90deg,rgba(255,106,26,.18),rgba(245,158,11,.14),rgba(34,197,94,.18));border:1px solid {border}}}
.base small{{display:block;font-weight:500;font-size:15px;color:{muted};margin-top:4px}}
.side{{display:flex;flex-direction:column;gap:14px}}
.facade{{border-radius:20px;padding:22px;background:linear-gradient(135deg,rgba(255,106,26,.22),rgba(34,197,94,.18));
 border:1px solid {border};color:{title}}}
.facade b{{font-size:21px}} .facade p{{font-size:15px;color:{text};margin-top:8px;line-height:1.5}}
.tool{{display:flex;gap:14px;align-items:center;border-radius:18px;padding:18px;background:{panel};border:1px solid {border};flex:1}}
.tool span{{font-size:30px}} .tool b{{display:block;font-size:17px;color:{title};margin-bottom:4px}}
"""
    body = ('<div class="wrap"><div class="eyebrow">Architecture</div>'
            '<h2>Focused crates. <span>One versioned workspace.</span></h2>'
            '<div class="main"><div class="stack">'
            f'<div class="appbar"><div class="lname">Your app</div>{apps}</div>'
            + "".join(rows)
            + '<div class="base">Axum · Tokio · Tower · SQLx<small>Standard Rust underneath — use their routers and pools whenever you need</small></div>'
            '</div><div class="side"><div class="facade"><b>🦀 <code style="font-size:19px;color:inherit">rullst</code></b>'
            '<p>One facade crate. Enable only the features your application needs.</p></div>'
            f'{tools}</div></div></div>')
    return page(theme, 1600, 880, css, body)


DEMOS = [("showcase", "rullst-showcase…azurecontainerapps.io"), ("lms", "rullst-lms…azurecontainerapps.io"),
         ("portfolio", "rullst-portfolio…azurecontainerapps.io"), ("saas", "saas.rullst.win")]


def frame(name: str, url: str, shot: pathlib.Path) -> str:
    css = """
body{{padding:34px}}
.win{{border-radius:18px;overflow:hidden;border:1px solid rgba(255,255,255,.14);
 box-shadow:0 30px 80px rgba(0,0,0,.55),0 0 0 1px rgba(255,106,26,.15)}}
.bar{{height:46px;background:linear-gradient(#1e293b,#111827);display:flex;align-items:center;padding:0 18px;gap:9px}}
.dot{{width:13px;height:13px;border-radius:50%}}
.url{{margin-left:22px;flex:1;max-width:560px;height:28px;border-radius:9px;background:rgba(255,255,255,.07);
 color:#cbd5e1;font-size:14px;display:flex;align-items:center;padding:0 14px;gap:8px}}
img{{display:block;width:1212px;height:682px;object-fit:cover;object-position:top}}
"""
    body = ('<div class="win"><div class="bar"><div class="dot" style="background:#ff5f57"></div>'
            '<div class="dot" style="background:#febc2e"></div><div class="dot" style="background:#28c840"></div>'
            f'<div class="url">🔒 {url}</div></div><img src="file://{shot}"></div>')
    return page("dark", 1280, 796, css, body)


if __name__ == "__main__":
    out, shots = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    for theme in THEME:
        (out / f"features-{theme}.html").write_text(features(theme))
        (out / f"architecture-{theme}.html").write_text(architecture(theme))
    for name, url in DEMOS:
        (out / f"demo-{name}.html").write_text(frame(name, url, shots / (f"{name}-crop.png" if name == "saas" else f"{name}.png")))
