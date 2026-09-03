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

## Step 2: Mount lifecycle-aware health routes

```rust,no_run
use rullst::{ApplicationLifecycle, Router, Server};
use rullst::health::{health_router_with_lifecycle, init_health_boot_time};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_health_boot_time();
    let lifecycle =
        ApplicationLifecycle::with_required_components(["database", "queue"])?;

    // Set these only after the application's own bounded checks succeed.
    lifecycle.set_component_ready("database", true)?;
    lifecycle.set_component_ready("queue", true)?;

    let app = Router::new()
        .merge_axum(health_router_with_lifecycle(lifecycle.clone()));
    Server::new(app)
        .with_lifecycle(lifecycle)
        .run(3000)
        .await?;
    Ok(())
}
```

`/health` remains process-only so an external database outage does not create a
restart loop. `/ready` returns `503` during startup, while any registered
component is unavailable, during drain, and after stop. The JSON carries only
aggregate counts; it never emits the component labels or their error messages.

The component registry is immutable, accepts at most 32 validated labels, and
does not execute probes by itself. The application must perform bounded,
timeout-protected checks and update each bit. `Server` marks startup complete
after binding, closes new application admission before graceful shutdown, and
waits through Axum for accepted requests. `run_with_shutdown` accepts a
supervisor future when OS signals are not the desired trigger.

This is one process contract. Kubernetes removes an unready Pod from service
according to its own timing; Rullst does not coordinate replica consensus,
load-balancer propagation, dependency failover, `preStop`, or the Pod's
`terminationGracePeriodSeconds`. Measure those together in staging. The legacy
`health_router()` remains the simpler process-only pair when dependency-aware
admission is not requested.

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
