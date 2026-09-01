# Tutorial 25: Kubernetes Manifests & Health Probes ☸️

`cargo-rullst` can generate a Kubernetes starter set. The files contain project
name/port defaults and must be reviewed for the target cluster, registry,
secrets, storage, network policy, ingress, workload identity, and security
policy before deployment.

---

## Step 1: Generate the starter manifests

```bash
cargo rullst make:k8s
```

The command writes `deployment.yaml`, `service.yaml`, `configmap.yaml`,
`hpa.yaml`, `ingress.yaml`, and `all-in-one.yaml` under `k8s/`. It may overwrite
files with those names, so run it in a clean worktree and review the diff.

Replace the placeholder `image: <project>:latest` with an immutable registry
reference (preferably a digest). The generated ConfigMap contains non-secret
settings only; use a Kubernetes Secret/external secret manager for credentials.

---

## Step 2: Mount and initialize the health routes

```rust,no_run
use rullst::{Router, Server};
use rullst::health::{health_router, init_health_boot_time};

#[tokio::main]
async fn main() -> Result<(), rullst::ServerError> {
    init_health_boot_time();
    let app = Router::new().merge_axum(health_router());
    Server::new(app).run(3000).await
}
```

The current `/health` and `/ready` handlers report process availability and
uptime only. `/ready` does **not** probe the database or external dependencies.
For an application that must stop receiving traffic when a critical dependency
is unavailable, replace or wrap readiness with a bounded, timeout-protected
application check. Keep liveness independent enough to avoid restart loops
during an external outage.

---

## Step 3: validate before applying

```bash
kubectl apply --dry-run=client -f k8s/all-in-one.yaml
kubectl diff -f k8s/all-in-one.yaml
kubectl apply -f k8s/all-in-one.yaml
```

The generated HPA uses `autoscaling/v2` and requires a working resource metrics
pipeline. Validate against the actual cluster version and admission policies.

---

## Release checklist

- add pod/container security contexts compatible with the reviewed image user;
- define CPU/memory requests and limits from measurements;
- add PodDisruptionBudget, topology spread/anti-affinity, NetworkPolicy, and
  service account/workload identity as required;
- terminate TLS with a configured issuer and real hostname;
- provision durable storage only where application state truly needs it; and
- verify probes, graceful termination, migrations, rollback, and autoscaling in
  a staging cluster.
