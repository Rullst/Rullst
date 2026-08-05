# Tutorial 25: Kubernetes-Native Scaffolding & Health Probes ☸️

Scaffold production Kubernetes manifests and configure Liveness (`/health`) and Readiness (`/ready`) probes.

---

## 🛠️ Step 1: Generate Kubernetes Manifests

```bash
cargo rullst make:k8s
```

Generates cloud-native files in `k8s/`:
- `deployment.yaml`
- `service.yaml`
- `configmap.yaml`
- `hpa.yaml` (Horizontal Pod Autoscaler)
- `ingress.yaml`
- `all-in-one.yaml`

---

## 💻 Step 2: Health Probe Endpoints

Mount the health router in `src/main.rs`:

```rust
use rullst_core::health::health_router;

#[tokio::main]
async fn main() {
    let app = axum::Router::new()
        .merge(health_router()); // GET /health & GET /ready

    rullst::Server::new().merge(app).run().await;
}
```

Deploy to Kubernetes:

```bash
kubectl apply -f k8s/all-in-one.yaml
```

---

## 💡 Key Takeaways
- `/health` monitors Liveness (application process health).
- `/ready` monitors Readiness (database pool & dependency readiness).
