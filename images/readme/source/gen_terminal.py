"""Generate the animated README terminal: the real `cargo rullst new` flow."""
import html
import pathlib
import sys

W, H = 1200, 568
T = 17.0  # loop length in seconds
X0, Y0, LH = 34, 92, 31
CHAR = 10.2  # estimated advance at 17px; covers adapt if a font is wider
MONO = "'JetBrains Mono', 'SF Mono', 'Cascadia Code', Menlo, Consolas, 'DejaVu Sans Mono', monospace"
BG = "#0b1020"

C = {
    "g": "#22c55e", "w": "#e5e7eb", "d": "#64748b", "c": "#22d3ee",
    "b": "#f8fafc", "o": "#fb923c", "y": "#facc15",
}

# (start second, typed?, segments[(text, color, bold)])
LINES = [
    (0.6, True, [("$ ", "g", True), ("cargo rullst new", "w", False)]),
    (2.1, False, [("✔ ", "g", True), ("🚀 What's the New App Name? ", "b", True),
                  ("(lowercase, no spaces, must start with a letter) · ", "d", False), ("my_saas", "c", True)]),
    (3.3, False, [("✔ ", "g", True), ("👉 Select a Starter Blueprint · ", "b", True),
                  ("SaaS App Starter ", "c", True), ("(Authentication + Stripe payments billing template)", "d", False)]),
    (4.4, False, [("✔ ", "g", True), ("🗄️ Will your project need a Database? · ", "b", True), ("yes", "c", True)]),
    (5.3, False, [("✔ ", "g", True), ("💾 Select the primary DB · ", "b", True), ("SQLite", "c", True)]),
    (6.4, False, [("  ✅ Database tables created successfully.", "g", False)]),
    (7.2, False, [("How to run:", "b", True)]),
    (7.5, False, [("  cd my_saas", "c", False)]),
    (7.8, False, [("    cargo rullst dev   ", "w", False), ("(standard output)", "d", False)]),
    (8.9, True, [("$ ", "g", True), ("cd my_saas && cargo rullst dev", "w", False)]),
    (10.9, False, [("   Compiling ", "g", True), ("my_saas v0.1.0", "w", False)]),
    (12.0, False, [("Rullst framework serving on ", "w", False), ("http://127.0.0.1:3000", "c", False)]),
    (12.6, False, [("🚀 Visit: ", "b", True), ("http://localhost:3000", "g", True), (" to see the result!", "b", True)]),
]
TYPE_SPEED = 0.055
FADE_START, FADE_END = 16.2, 16.8


def pct(t: float) -> str:
    return f"{t / T * 100:.3f}%"


def build(freeze: float | None = None) -> str:
    css, body = [], []
    for i, (start, typed, segs) in enumerate(LINES):
        y = Y0 + i * LH + (LH if i >= 9 else 0)  # blank line before the second command
        spans = "".join(
            f'<tspan fill="{C[c]}"{" font-weight=\"700\"" if bold else ""}>{html.escape(text)}</tspan>'
            for text, c, bold in segs
        )
        css.append(f"@keyframes ln{i}{{0%,{pct(max(start - 0.01, 0))}{{opacity:0}}{pct(start)},100%{{opacity:1}}}}")
        css.append(f".ln{i}{{opacity:0;animation:ln{i} {T}s linear infinite}}")
        line = f'<g class="ln{i}"><text x="{X0}" y="{y}" xml:space="preserve">{spans}</text>'
        if typed:
            prompt = len(segs[0][0])
            typed_chars = sum(len(t) for t, _, _ in segs[1:])
            x_cover = X0 + prompt * CHAR
            end = start + 0.25 + typed_chars * TYPE_SPEED
            dist = typed_chars * CHAR
            css.append(
                f"@keyframes cv{i}{{0%,{pct(start + 0.25)}{{transform:translateX(0);animation-timing-function:steps({typed_chars},end)}}"
                f"{pct(end)}{{transform:translateX({dist:.1f}px)}}{pct(end + 0.35)}{{transform:translateX({dist:.1f}px)}}"
                f"{pct(end + 0.36)},100%{{transform:translateX(1200px)}}}}"
            )
            css.append(f".cv{i}{{animation:cv{i} {T}s linear infinite}}")
            line += (
                f'<g class="cv{i}"><rect x="{x_cover:.1f}" y="{y - 22}" width="1150" height="30" fill="{BG}"/>'
                f'<rect class="blink" x="{x_cover:.1f}" y="{y - 19}" width="10" height="23" fill="{C["w"]}"/></g>'
            )
        line += "</g>"
        body.append(line)
    last_y = Y0 + (len(LINES) - 1) * LH + LH
    body.append(
        f'<g class="ln{len(LINES) - 1}"><rect class="blink" x="{X0}" y="{last_y + LH - 19}" width="10" height="23" fill="{C["w"]}"/></g>'
    )
    frozen = f"*{{animation-play-state:paused!important;animation-delay:-{freeze}s!important}}" if freeze is not None else ""
    return f'''<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}" role="img" aria-labelledby="t d">
<title id="t">cargo rullst new — from zero to a running SaaS</title>
<desc id="d">Terminal recording: cargo rullst new asks for the app name, blueprint and database, creates the SaaS starter with SQLite, then cargo rullst dev compiles and serves it at http://localhost:3000.</desc>
<style>
text{{font-family:{MONO};font-size:17px}}
@keyframes blink{{0%,49%{{opacity:1}}50%,100%{{opacity:0}}}}
@keyframes fade{{0%,{pct(FADE_START)}{{opacity:1}}{pct(FADE_END)},100%{{opacity:0}}}}
.blink{{animation:blink 1.05s steps(1) infinite}}
.content{{animation:fade {T}s linear infinite}}
{"".join(css)}
@media (prefers-reduced-motion:reduce){{*{{animation:none!important}}[class^=ln]{{opacity:1!important}}[class^=cv]{{transform:translateX(1200px)!important}}}}
{frozen}
</style>
<defs>
<linearGradient id="bar" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#1e293b"/><stop offset="1" stop-color="#111827"/></linearGradient>
<linearGradient id="edge" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#ff6a1a" stop-opacity=".55"/><stop offset=".5" stop-color="#ffffff" stop-opacity=".08"/><stop offset="1" stop-color="#22c55e" stop-opacity=".55"/></linearGradient>
<clipPath id="win"><rect x="1" y="1" width="{W - 2}" height="{H - 2}" rx="14"/></clipPath>
</defs>
<g clip-path="url(#win)">
<rect width="{W}" height="{H}" fill="{BG}"/>
<rect width="{W}" height="46" fill="url(#bar)"/>
<circle cx="26" cy="23" r="7" fill="#ff5f57"/><circle cx="50" cy="23" r="7" fill="#febc2e"/><circle cx="74" cy="23" r="7" fill="#28c840"/>
<text x="{W / 2}" y="28" text-anchor="middle" fill="#94a3b8" style="font-size:14px">~/projects — cargo rullst</text>
<g class="content">{"".join(body)}</g>
</g>
<rect x="1" y="1" width="{W - 2}" height="{H - 2}" rx="14" fill="none" stroke="url(#edge)" stroke-width="2"/>
</svg>
'''


if __name__ == "__main__":
    out = pathlib.Path(sys.argv[1])
    (out / "terminal.svg").write_text(build())
    for t in (1.2, 9.6, 14.0):
        (out / f"preview-terminal-{t}.svg").write_text(build(freeze=t))
