# Tutorial 14: RASP — Runtime Application Self-Protection ⚡

`rullst-security::rasp` inspects URI query parameters and payload strings to intercept SQL Injection, XSS, Path Traversal, and SSRF attacks before controller execution.

---

## 🛠️ Step 1: Mount `RaspSecurityLayer` in `main.rs`

```rust
use axum::Router;
use rullst_security::rasp::RaspSecurityLayer;
use rullst::Server;

#[tokio::main]
async fn main() {
    let app = Router::new()
        // ... routes
        .layer(RaspSecurityLayer::default());

    Server::new()
        .merge(app)
        .run()
        .await;
}
```

---

## 🧪 Step 2: Test Malicious Attack Payload Interception

Send an attack payload in query string:

```bash
curl "http://localhost:3000/api/users?query=SELECT%20*%20FROM%20users;--' OR 1=1"
```

RASP instantly responds with HTTP `403 Forbidden` and logs the blocked attack vector in the **Visual Threat Radar (SOC)** at `http://localhost:5555/studio/security`.

---

## 💡 Key Takeaways
- Zero-latency pre-controller inspection.
- Protects APIs even if raw queries or third-party crates have vulnerabilities.
