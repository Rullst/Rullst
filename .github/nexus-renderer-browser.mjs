// Synthetic stored values and HTML from the real Rust renderer, supplied on stdin.
// No provider accounts, application data, npm dependencies or external assets.
import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { once } from "node:events";
import { mkdtemp, rm } from "node:fs/promises";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";

let input = "";
for await (const chunk of process.stdin) {
  input += chunk;
  assert(Buffer.byteLength(input) <= 2 * 1024 * 1024, "bounded renderer fixtures");
}
const cases = JSON.parse(input);
assert(Array.isArray(cases) && cases.length > 0 && cases.length <= 128);
for (const fixture of cases) {
  assert.equal(typeof fixture.html, "string");
  assert.equal(typeof fixture.text, "string");
  assert.equal(typeof fixture.enabled, "boolean");
}
const expected = JSON.stringify(cases.map(({ text, enabled }) => ({ text, enabled })))
  .replaceAll("<", "\\u003c");
const script = `
const expected = ${expected};
const result = document.getElementById("nexus-result");
try {
  expected.forEach((fixture, index) => {
    const table = document.getElementById("fixture-" + index);
    const rows = table.querySelectorAll("tbody > tr");
    if (rows.length !== 1) throw new Error("row structure changed: " + index);
    const cells = rows[0].querySelectorAll(":scope > td");
    if (cells.length !== 6) throw new Error("cell structure changed: " + index);
    if (cells[1].textContent !== "1" || cells[4].textContent !== "42")
      throw new Error("numeric cells changed: " + index);
    if (cells[2].childElementCount !== 0 || cells[2].textContent !== fixture.text)
      throw new Error("stored text became DOM or changed value: " + index);
    if (cells[3].textContent !== (fixture.enabled ? "✅ Yes" : "❌ No"))
      throw new Error("boolean indicator changed: " + index);
  });
  result.dataset.result = "pass";
} catch (error) {
  result.textContent = String(error);
  result.dataset.result = "fail";
}
`;
const hash = createHash("sha256").update(script).digest("base64");
const html = '<!doctype html><html lang="en"><head><meta charset="utf-8">'
  + '<title>Nexus renderer fixture</title></head><body>'
  + cases.map(({ html }, index) => `<table id="fixture-${index}"><tbody>${html}</tbody></table>`).join("")
  + '<output id="nexus-result" data-result="pending"></output>'
  + `<script>${script}</script></body></html>`;
const profile = await mkdtemp(join(tmpdir(), "rullst-nexus-renderer-"));
const server = createServer((_request, response) => {
  response.writeHead(200, {
    "Content-Type": "text/html; charset=utf-8",
    "Content-Security-Policy": `default-src 'none'; script-src 'sha256-${hash}'; base-uri 'none'; form-action 'none'`,
    "Cache-Control": "no-store",
  });
  response.end(html);
});
try {
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const { stdout } = await promisify(execFile)(process.env.CHROME_BIN || "google-chrome", [
    "--headless=new", "--no-first-run", "--no-default-browser-check",
    "--disable-background-networking", "--disable-component-update", "--disable-dev-shm-usage",
    `--user-data-dir=${profile}`, "--dump-dom", `http://127.0.0.1:${server.address().port}`,
  ], { timeout: 60_000, maxBuffer: 2 * 1024 * 1024 });
  assert(stdout.includes('id="nexus-result" data-result="pass"'), "browser must verify every stored value as literal text");
  console.log(`Nexus stored-value DOM contracts passed (${cases.length} fixtures).`);
} finally {
  server.closeAllConnections();
  await new Promise(resolve => server.close(resolve));
  await rm(profile, { recursive: true, force: true });
}
