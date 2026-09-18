// Browser-policy regression over generated SaaS CSP rendered by Core.
// Synthetic form/303 and intercepted destinations: no accounts or provider I/O.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdtemp, rm } from "node:fs/promises";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { join } from "node:path";

let input = "";
for await (const chunk of process.stdin) {
  input += chunk;
  assert(Buffer.byteLength(input) <= 16 * 1024, "bounded header fixture");
}
const headers = JSON.parse(input);
assert(headers["content-security-policy"].includes("form-action 'self' https://checkout.stripe.com"));
delete headers["content-length"];
const approved = "https://checkout.stripe.com/c/pay/cs_test_policy";
const cases = [
  {name: "strict-default", destination: approved, allowed: false, strict: true},
  {name: "stripe", destination: approved, allowed: true},
  {name: "lookalike", destination: "https://checkout.stripe.com.example.invalid/c/pay/test", allowed: false},
  {name: "other-provider", destination: "https://checkout.lemonsqueezy.com/checkout/test", allowed: false},
  {name: "cleartext", destination: "http://checkout.stripe.com/c/pay/test", allowed: false},
  {name: "other-port", destination: "https://checkout.stripe.com:444/c/pay/test", allowed: false},
  {name: "second-redirect", destination: approved + "/redirect-again", allowed: false},
];
let posts = 0;
const server = createServer((request, response) => {
  const url = new URL(request.url, "http://localhost");
  const test = cases.find(item => item.name === url.searchParams.get("case"));
  if (!test) { response.writeHead(404).end(); return; }
  if (request.method === "POST" && url.pathname === "/checkout") {
    posts += 1;
    request.resume();
    response.writeHead(303, {Location: test.destination, "Cache-Control": "no-store"}).end();
    return;
  }
  if (request.method !== "GET" || url.pathname !== "/") { response.writeHead(405).end(); return; }
  const policy = test.strict ? headers["content-security-policy"].replace(" https://checkout.stripe.com", "") : headers["content-security-policy"];
  response.writeHead(200, {...headers, "content-type": "text/html; charset=utf-8", "content-security-policy": policy});
  response.end(`<!doctype html><html><head><title>Checkout policy fixture</title><link rel="icon" href="data:,"></head><body><form method="post" action="/checkout?case=${test.name}"><input name="plan" value="server-owned-test-plan" type="hidden"><button type="submit">Continue</button></form></body></html>`);
});
server.listen(0, "127.0.0.1");
await once(server, "listening");
const origin = `http://127.0.0.1:${server.address().port}`;
const profile = await mkdtemp(join(tmpdir(), "rullst-billing-browser-"));
const chrome = spawn(process.env.CHROME_BIN || "google-chrome", [
  "--headless=new", "--no-first-run", "--no-default-browser-check",
  "--disable-background-networking", "--disable-component-update",
  "--disable-dev-shm-usage", "--remote-debugging-port=0",
  `--user-data-dir=${profile}`, "about:blank",
], {stdio: ["ignore", "ignore", "pipe"]});
let socket;
const deadline = setTimeout(() => { chrome.kill("SIGKILL"); }, 90_000);
try {
  const endpoint = await new Promise((accept, reject) => {
    let stderr = "";
    const timer = setTimeout(() => reject(new Error(`Chrome startup timeout: ${stderr}`)), 45_000);
    const fail = error => { clearTimeout(timer); reject(error); };
    chrome.once("error", fail);
    chrome.once("exit", code => fail(new Error(`Chrome exited: ${code}`)));
    chrome.stderr.on("data", chunk => {
      stderr = (stderr + chunk).slice(-8192);
      const match = stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/);
      if (match) { clearTimeout(timer); accept(match[1]); }
    });
  });
  socket = new WebSocket(endpoint);
  await new Promise((accept, reject) => {
    const timer = setTimeout(() => reject(new Error("CDP handshake timeout")), 15_000);
    socket.addEventListener("open", () => { clearTimeout(timer); accept(); }, { once: true });
    socket.addEventListener("error", () => { clearTimeout(timer); reject(new Error("CDP handshake failed")); }, { once: true });
  });
  let sequence = 0;
  const pending = new Map();
  const events = new Map();
  socket.addEventListener("message", ({ data }) => {
    const message = JSON.parse(data);
    if (message.id) {
      const operation = pending.get(message.id);
      if (!operation) return;
      pending.delete(message.id);
      clearTimeout(operation.timer);
      if (message.error) operation.reject(new Error(JSON.stringify(message.error)));
      else operation.accept(message.result);
    } else events.get(message.method)?.(message.params);
  });
  const call = (method, params = {}, sessionId) => new Promise((accept, reject) => {
    const id = ++sequence;
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 15_000);
    pending.set(id, { accept, reject, timer });
    socket.send(JSON.stringify({ id, method, params, sessionId }));
  });
  const { targetId } = await call("Target.createTarget", { url: "about:blank" });
  const { sessionId } = await call("Target.attachToTarget", { targetId, flatten: true });
  const send = (method, params) => call(method, params, sessionId);
  const evaluate = async expression => {
    const result = await send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
    assert(!result.exceptionDetails, JSON.stringify(result.exceptionDetails));
    return result.result.value;
  };
  await send("Page.enable");
  await send("Runtime.enable");
  await send("Network.enable");
  const failures = [];
  const intercepted = [];
  // Intercept every request before dispatch. Only the loopback fixture reaches a server.
  events.set("Fetch.requestPaused", params => {
    (async () => {
      const url = new URL(params.request.url);
      if (url.origin === origin) {
        await send("Fetch.continueRequest", {requestId: params.requestId});
        return;
      }
      intercepted.push(params.request.url);
      if (url.origin !== "https://checkout.stripe.com") {
        await send("Fetch.failRequest", {requestId: params.requestId, errorReason: "BlockedByClient"});
        failures.push(`unexpected destination ${url.origin}`);
        return;
      }
      assert.equal(params.request.method, "GET", "303 must change the form POST to GET");
      if (url.pathname.endsWith("/redirect-again")) {
        await send("Fetch.fulfillRequest", {requestId: params.requestId, responseCode: 303,
          responseHeaders: [{name: "Location", value: "https://unreviewed.example.invalid/checkout"}]});
      } else {
        await send("Fetch.fulfillRequest", {requestId: params.requestId, responseCode: 200,
          responseHeaders: [{name: "Content-Type", value: "text/html"}],
          body: Buffer.from('<!doctype html><html><head><link rel="icon" href="data:,"></head><body>intercepted-checkout</body></html>').toString("base64")});
      }
    })().catch(error => failures.push(String(error)));
  });
  await send("Fetch.enable", {patterns: [{urlPattern: "*", requestStage: "Request"}]});
  const pause = () => new Promise(resolve => setTimeout(resolve, 50));
  const waitFor = async expression => {
    for (let attempt = 0; attempt < 200; attempt += 1) {
      try { if (await evaluate(expression)) return; } catch { /* navigation replaced the context */ }
      await pause();
    }
    assert.fail(`browser condition timed out: ${expression}`);
  };
  for (const test of cases) {
    const priorRequests = intercepted.length;
    await send("Page.navigate", {url: `${origin}/?case=${test.name}`});
    await waitFor(`location.origin === ${JSON.stringify(origin)} && location.search === '?case=${test.name}' && !!document.querySelector('form')`);
    await evaluate(`window.policyViolations = []; document.addEventListener('securitypolicyviolation', event => window.policyViolations.push(event.effectiveDirective)); document.querySelector('form').requestSubmit(); true`);
    if (test.allowed) {
      await waitFor("location.origin === 'https://checkout.stripe.com' && document.body?.textContent === 'intercepted-checkout'");
      assert.equal(intercepted.length, priorRequests + 1, "one intercepted approved handoff");
    } else {
      await waitFor("window.policyViolations?.includes('form-action')");
      assert.equal(await evaluate("location.origin"), origin, "CSP retains the submitting document");
      assert.equal(intercepted.length, priorRequests + (test.name === "second-redirect" ? 1 : 0), "CSP must block disallowed navigation before external dispatch");
    }
    assert.deepEqual(failures, []);
    console.log(`Checkout CSP browser contract passed: ${test.name}`);
  }
  assert.equal(posts, cases.length, "every case exercised the local POST and its 303");
  await call("Browser.close");
} finally {
  clearTimeout(deadline);
  socket?.close();
  if (chrome.exitCode === null) {
    const stopped = once(chrome, "exit").catch(() => {});
    chrome.kill("SIGKILL");
    await stopped;
  }
  server.closeAllConnections();
  await new Promise(resolve => server.close(resolve));
  await rm(profile, {recursive: true, force: true, maxRetries: 5, retryDelay: 100});
}
