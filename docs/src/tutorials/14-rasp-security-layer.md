# Tutorial 14: RASP — Runtime Application Self-Protection ⚡

`rullst-security::rasp` applies bounded heuristic signatures to request targets,
non-secret headers, and supported textual bodies. Its current signatures cover
common SQL injection, path traversal, SSRF, shell/RCE, and JNDI indicators. It
does not claim general exploit detection. The `powershell` keyword is not
matched against the stock PowerShell product token in `User-Agent`
(`PowerShell/7.4.1`, `WindowsPowerShell/5.1…`), so `Invoke-WebRequest` and
`Invoke-RestMethod` clients are served, while execution syntax such as
`powershell -enc` or `powershell.exe` in that header still blocks.

---

## 🛠️ Step 1: Mount `RaspSecurityLayer` in `main.rs`

```rust,no_run
use axum::Router;
use rullst_security::rasp::RaspSecurityLayer;
use rullst::Server;

#[tokio::main]
async fn main() -> Result<(), rullst::ServerError> {
    let app = Router::new()
        // ... routes
        .layer(RaspSecurityLayer::default());

    Server::new(app.into()).run(3000).await
}
```

---

## 🧪 Step 2: Test Malicious Attack Payload Interception

Send a percent-encoded `UNION SELECT` payload in the query string:

```bash
curl -i "http://localhost:3000/api/users?query=1%27%20UNION%20SELECT%20password%20FROM%20users"
```

For a recognized bounded signature, the layer returns `403 Forbidden` and adds
a process-local event to `SecurityStore`. A Studio instance running in the same
process can display that event at
`http://127.0.0.1:5555/studio/security`.

The body inspector accepts identity-encoded UTF-8 text, JSON, form, and XML
media types up to 1 MiB. It fails closed for oversized declared bodies and
encoded textual bodies that it cannot inspect. Put an independent request-body
limit outside this layer as well.

---

## 💡 Key Takeaways
- Inspection has runtime cost and uses bounded pattern heuristics, with possible
  false positives and false negatives.
- RASP is defense in depth; parameterized SQL, validation, authorization, body
  limits, and dependency review remain required.
