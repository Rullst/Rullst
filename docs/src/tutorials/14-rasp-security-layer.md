# Tutorial 14: RASP — Runtime Application Self-Protection ⚡

`rullst-security::rasp` inspects URI query parameters and payload strings to intercept SQL Injection, XSS, Path Traversal, and SSRF attacks before controller execution.

---

## 🛠️ Step 1: Mount `RaspSecurityLayer` in `main.rs`

```rust
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

Send an attack payload in query string:

```bash
curl "http://localhost:3000/api/users?query=SELECT%20*%20FROM%20users;--' OR 1=1"
```

For a recognized bounded signature, the layer returns `403 Forbidden` and adds
a process-local event to `SecurityStore`. A Studio instance running in the same
process can display that event at
`http://127.0.0.1:5555/studio/security`.

---

## 💡 Key Takeaways
- Inspection has runtime cost and uses bounded pattern heuristics, with possible
  false positives and false negatives.
- RASP is defense in depth; parameterized SQL, validation, authorization, body
  limits, and dependency review remain required.
