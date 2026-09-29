"""Generate the animated README hero banner (dark and light) as self-contained SVG."""
import base64
import pathlib
import sys

HERE = pathlib.Path(__file__).parent
LOGO = base64.b64encode((HERE / "logo-320.webp").read_bytes()).decode()

W, H = 1400, 480
SANS = "Inter, 'Segoe UI', 'SF Pro Display', -apple-system, 'Helvetica Neue', Helvetica, Arial, sans-serif"

THEMES = {
    "dark": {
        "bg0": "#060a14", "bg1": "#0c1428", "grid": "rgba(148,163,184,0.08)",
        "title": "#f8fafc", "muted": "#94a3b8", "eyebrow": "#fb923c",
        "pill_bg": "rgba(255,255,255,0.05)", "pill_stroke": "rgba(255,255,255,0.14)",
        "pill_text": "#e2e8f0", "glow": (0.42, 0.30, 0.22), "spark": "#ffffff",
        "shine": "rgba(255,255,255,0.55)",
    },
    "light": {
        "bg0": "#ffffff", "bg1": "#f1f5f9", "grid": "rgba(15,23,42,0.06)",
        "title": "#0f172a", "muted": "#475569", "eyebrow": "#ea580c",
        "pill_bg": "rgba(255,255,255,0.85)", "pill_stroke": "rgba(15,23,42,0.12)",
        "pill_text": "#1e293b", "glow": (0.22, 0.16, 0.12), "spark": "#f97316",
        "shine": "rgba(255,255,255,0.75)",
    },
}

PILLS = [
    ("Auth &amp; Security", "#f97316"),
    ("ORM &amp; Migrations", "#22c55e"),
    ("Payments", "#eab308"),
    ("AI &amp; RAG", "#a855f7"),
    ("Admin &amp; Studio", "#3b82f6"),
]

SPARKS = [(560, 70, 1.8, 0.0), (1330, 110, 2.2, -1.3), (1250, 440, 1.6, -2.1),
          (720, 450, 2.0, -0.7), (95, 60, 1.5, -2.8), (410, 430, 2.4, -1.9),
          (1030, 40, 1.7, -3.3), (1370, 300, 1.9, -0.4), (40, 380, 2.1, -2.4)]


def pill_width(label: str) -> float:
    text = label.replace("&amp;", "&")
    return len(text) * 9.7 + 50


def build(theme: str, freeze: float | None = None) -> str:
    t = THEMES[theme]
    go, gg, gb = t["glow"]
    pills, x = [], 470.0
    for label, color in PILLS:
        w = pill_width(label)
        pills.append(
            f'<g transform="translate({x:.1f},382)">'
            f'<rect width="{w:.1f}" height="38" rx="19" fill="{t["pill_bg"]}" stroke="{t["pill_stroke"]}"/>'
            f'<circle cx="20" cy="19" r="5" fill="{color}"/>'
            f'<text x="33" y="25" font-size="15" font-weight="600" fill="{t["pill_text"]}">{label}</text></g>'
        )
        x += w + 10
    sparks = "".join(
        f'<circle class="spark" cx="{cx}" cy="{cy}" r="{r}" fill="{t["spark"]}" style="animation-delay:{d}s"/>'
        for cx, cy, r, d in SPARKS
    )
    frozen = ""
    if freeze is not None:
        frozen = f"*{{animation-play-state:paused!important;animation-delay:-{freeze}s!important}}"
    return f'''<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}" role="img" aria-labelledby="t d">
<title id="t">Rullst — the full-stack Rust framework</title>
<desc id="d">Build secure apps in Rust without the suffering. Batteries included, secure by default, designed for humans and AI.</desc>
<style>
text{{font-family:{SANS}}}
@keyframes float{{0%,100%{{transform:translateY(0)}}50%{{transform:translateY(-12px)}}}}
@keyframes spin{{to{{transform:rotate(360deg)}}}}
@keyframes spinr{{to{{transform:rotate(-360deg)}}}}
@keyframes pulse{{0%,100%{{opacity:.75}}50%{{opacity:1}}}}
@keyframes drift{{0%,100%{{transform:translate(0,0)}}50%{{transform:translate(28px,-16px)}}}}
@keyframes shine{{0%{{transform:translateX(-420px)}}55%,100%{{transform:translateX(900px)}}}}
@keyframes twinkle{{0%,100%{{opacity:0}}50%{{opacity:.9}}}}
.float{{animation:float 6s ease-in-out infinite}}
.spin{{transform-origin:250px 240px;animation:spin 38s linear infinite}}
.spinr{{transform-origin:250px 240px;animation:spinr 60s linear infinite}}
.glow{{animation:pulse 7s ease-in-out infinite,drift 14s ease-in-out infinite}}
.shine{{animation:shine 5.5s ease-in-out infinite}}
.spark{{opacity:0;animation:twinkle 3.6s ease-in-out infinite}}
@media (prefers-reduced-motion:reduce){{*{{animation:none!important}}.spark{{opacity:.6}}}}
{frozen}
</style>
<defs>
<linearGradient id="bg" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="{t["bg0"]}"/><stop offset="1" stop-color="{t["bg1"]}"/></linearGradient>
<radialGradient id="go"><stop offset="0" stop-color="#ff6a1a" stop-opacity="{go}"/><stop offset="1" stop-color="#ff6a1a" stop-opacity="0"/></radialGradient>
<radialGradient id="gg"><stop offset="0" stop-color="#22c55e" stop-opacity="{gg}"/><stop offset="1" stop-color="#22c55e" stop-opacity="0"/></radialGradient>
<radialGradient id="gb"><stop offset="0" stop-color="#3b82f6" stop-opacity="{gb}"/><stop offset="1" stop-color="#3b82f6" stop-opacity="0"/></radialGradient>
<linearGradient id="brand" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#ff6a1a"/><stop offset=".5" stop-color="#f59e0b"/><stop offset="1" stop-color="#22c55e"/></linearGradient>
<linearGradient id="ring" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#ff6a1a"/><stop offset="1" stop-color="#22c55e"/></linearGradient>
<linearGradient id="shineg" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#fff" stop-opacity="0"/><stop offset=".5" stop-color="{t["shine"]}"/><stop offset="1" stop-color="#fff" stop-opacity="0"/></linearGradient>
<radialGradient id="fade"><stop offset=".35" stop-color="#fff"/><stop offset="1" stop-color="#000"/></radialGradient>
<mask id="gridmask"><rect width="{W}" height="{H}" fill="url(#fade)"/></mask>
<pattern id="grid" width="44" height="44" patternUnits="userSpaceOnUse"><path d="M44 0H0V44" fill="none" stroke="{t["grid"]}" stroke-width="1"/></pattern>
<clipPath id="h2"><text x="470" y="292" font-size="56" font-weight="800">without the suffering.</text></clipPath>
<clipPath id="frame"><rect width="{W}" height="{H}" rx="24"/></clipPath>
</defs>
<g clip-path="url(#frame)">
<rect width="{W}" height="{H}" fill="url(#bg)"/>
<rect width="{W}" height="{H}" fill="url(#grid)" mask="url(#gridmask)"/>
<circle class="glow" cx="250" cy="240" r="300" fill="url(#go)"/>
<circle class="glow" cx="1210" cy="430" r="330" fill="url(#gg)" style="animation-delay:-3s,-5s"/>
<circle class="glow" cx="920" cy="40" r="250" fill="url(#gb)" style="animation-delay:-5s,-9s"/>
{sparks}
<g class="float">
<circle class="spin" cx="250" cy="240" r="168" fill="none" stroke="url(#ring)" stroke-width="2" stroke-dasharray="6 14" opacity=".55"/>
<circle class="spinr" cx="250" cy="240" r="192" fill="none" stroke="url(#ring)" stroke-width="1.2" stroke-dasharray="2 10" opacity=".35"/>
<image href="data:image/webp;base64,{LOGO}" x="100" y="110" width="300" height="261"/>
</g>
<text x="472" y="150" font-size="17" font-weight="700" letter-spacing="5" fill="{t["eyebrow"]}">THE FULL-STACK RUST FRAMEWORK</text>
<text x="470" y="226" font-size="56" font-weight="800" fill="{t["title"]}">Build secure apps in Rust</text>
<text x="470" y="292" font-size="56" font-weight="800" fill="url(#brand)">without the suffering.</text>
<g clip-path="url(#h2)"><rect class="shine" x="470" y="230" width="160" height="80" fill="url(#shineg)"/></g>
<text x="472" y="344" font-size="21" fill="{t["muted"]}">Batteries included · Secure by default · Designed for humans and AI</text>
{"".join(pills)}
<rect y="{H - 3}" width="{W}" height="3" fill="url(#brand)" opacity=".8"/>
</g>
</svg>
'''


if __name__ == "__main__":
    out = pathlib.Path(sys.argv[1])
    for theme in THEMES:
        (out / f"hero-{theme}.svg").write_text(build(theme))
        for t in (1.5, 4.0):
            (out / f"preview-hero-{theme}-{t}.svg").write_text(build(theme, freeze=t))
