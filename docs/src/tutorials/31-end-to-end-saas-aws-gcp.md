# Tutorial 31: SaaS deployment preparation for AWS or GCP

This guide prepares a generated SaaS application for a cloud deployment. It is
not an end-to-end production certification: identity, network policy, database
operation, secrets, billing and recovery remain deployment responsibilities.

## 1. Materialize and verify the SaaS starter

While v12 is unreleased, use a reviewed `main` checkout pinned by `Cargo.lock`.
After a prerelease ships,
install the matching versioned CLI and generate deterministically:

```bash
cargo rullst new my_cloud_saas --default --blueprint saas --docker
cd my_cloud_saas
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

Review every generated route, access policy, migration and environment
placeholder before supplying live credentials.

## 2. Supply secrets outside source control

Use AWS Secrets Manager, Google Secret Manager or an equivalent deployment
boundary for values such as `DATABASE_URL`, provider keys, webhook secrets and
the Rullst field-encryption key. Do not bake `.env` into an image.

Set `RULLST_ENV=production` and make startup fail when a required production
adapter or credential is missing. Empty and `mock_*` provider credentials are
for deterministic offline development, not live operation.

## 3. Build and scan the exact image

```bash
docker build --pull --tag my-cloud-saas:<git-sha> .
docker inspect my-cloud-saas:<git-sha>
```

Pin the deployed image by digest. Run the application's tests and a container
scanner against the exact candidate. The CLI's `deploy` command scaffolds or
invokes Fly.io, Railway, Render and VPS paths; AWS App Runner/ECS and Google
Cloud Run configuration remains explicit cloud work.

## 4. Configure the cloud boundary

For AWS or GCP, define and review:

- private database connectivity, TLS and least-privilege credentials;
- trusted proxy handling and the application's external origin policy;
- ingress authentication, rate limits, body limits and request timeouts;
- readiness/liveness behavior and a bounded shutdown grace period;
- immutable image rollout and a tested rollback procedure;
- logs, metrics and alerts that avoid secrets and unnecessary PII.

Managed services have provider-specific scaling floors, quotas, cold starts and
costs. Verify current provider documentation and load-test your selected region
and topology rather than assuming scale-to-zero or a request rate.

## 5. Exercise stateful recovery

Before production traffic, rehearse migrations, backup, restore, webhook replay,
field-encryption key rotation, a failed rollout and database unavailability.
Health endpoints only report the checks implemented by the application; they do
not create zero-downtime deployment by themselves.
