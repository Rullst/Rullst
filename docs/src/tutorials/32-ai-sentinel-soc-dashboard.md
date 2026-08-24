# Tutorial 32: SOC Threat Radar & Autonomous AI Security 🛡️

Build a complete enterprise defense system combining Honeypot deception traps, RASP runtime inspection, Vault secret zeroization, and an offline AI Threat Sentinel.

---

## 🛠️ Step 1: Wire Defense Layers in `main.rs`

```rust
use axum::Router;
use rullst_security::{
    HoneypotLayer, HoneypotState, CspSecurityLayer,
    rasp::RaspSecurityLayer,
};
use rullst::Server;

#[tokio::main]
async fn main() {
    let state = HoneypotState::default();

    let app = Router::new()
        // ... routes
        .layer(RaspSecurityLayer::default())
        .layer(CspSecurityLayer::default())
        .layer(HoneypotLayer::new(state));

    Server::new().merge(app).run().await;
}
```

---

## 🤖 Step 2: Enable Local AI Sentinel via Ollama

In `.env`:

```dotenv
AI_PROVIDER=ollama
OLLAMA_HOST=http://localhost:11434
AI_MODEL=llama3:8b
```

When malicious requests hit synthetic deception endpoints (e.g. `/.env`, `/admin.php`), Honeypot traps fingerprint the bot IP, and AI Sentinel automatically issues dynamic Proof-of-Work challenge tokens.

---

## 📊 Step 3: Monitor Live SOC Threat Radar

Launch dev mode:
```bash
cargo rullst dev
```

Open Rullst Studio Visual Threat Radar at `http://localhost:5555/studio/security` to observe live attack vectors, blocked IP counts, and HMAC audit chain integrity in real-time.

---

## 💡 Key Takeaways
- Deception traps ban automated scanning bots before they reach application logic.
- local AI classification without a required cloud LLM when the deployment is configured and isolated accordingly.
