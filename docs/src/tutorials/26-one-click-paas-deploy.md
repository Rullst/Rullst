# Tutorial 26: Guided Container Deployment 🚀

`cargo rullst deploy` prepares the files you need to run your application on any
Linux server (a virtual machine from any provider, or your own machine) with
Docker or Podman. It writes them into your project and does not contact any
provider; you review them and run them yourself.

| File | Purpose |
| :--- | :--- |
| `Dockerfile` | Multi-stage build with a dependency-cache layer and a small non-root runtime image. |
| `compose.yaml` | Your application and Caddy (automatic HTTPS), a private network and volumes. Works with `docker compose` and `podman compose`. |
| `Caddyfile` | The domain Caddy serves and the proxy to the application. |

Step-by-step guides that use these files:
[Deploy to a Linux VM](../vm-deployment.md), with provider pages for
[Hetzner Cloud](../deploy-hetzner.md), [DigitalOcean](../deploy-digitalocean.md),
[Oracle Cloud](../deploy-oracle-cloud.md) and [Google Cloud](../deploy-google-cloud.md).
For Nix, Buildah, Podman Quadlet or Kubernetes see
[Advanced deployment](../advanced-deployment.md). The CLI no longer has
provider-specific targets (Fly.io, Railway, Render) or the Foundry SSH
pipeline; container platforms can still build the generated `Dockerfile`, but
their configuration is yours to write and review.

---

## Before the first deploy

These staging and production defaults apply to every way of deploying:

- **Migrations are a separate step by default.** `Server::run` only opens the
  database pool, and `cargo rullst dev` applies pending migrations only in
  development. Run the application binary's `db:migrate` command once per
  release, before the new version serves traffic: `/app/<name> db:migrate` in
  the generated image (it has no shell), or `cargo run --release -- db:migrate`
  from a checkout. Since 13.0.0-alpha.2 the server can instead apply them at
  startup: see [startup migrations](#startup-migrations) and the
  [container lifecycle boundary](#container-lifecycle-boundary).
- **Bind address.** With `RULLST_ENV=staging` or `production`, the server
  listens on `0.0.0.0` unless `HOST` (or `RULLST_HOST`) is set in the process
  environment or `.env`. Behind a reverse proxy on the same host, set
  `HOST=127.0.0.1` so the plain-HTTP port is reachable only through the proxy.
  Inside a container keep `0.0.0.0` and publish
  the port only to the proxy's network. The server prints the address it
  listens on at startup (`Rullst framework serving on http://0.0.0.0:3000
  (every network interface)`); since 13.0.0-alpha.2, when that is every
  interface and a trusted reverse proxy is configured (`[security]
  trusted_proxies` or `Server::trusted_proxies`), the same line suggests
  `HOST=127.0.0.1` for a proxy on the same host outside a container network
  and, since 13.0.0-alpha.4, keeping `HOST=0.0.0.0` when the proxy reaches the
  application over a container network (as in the generated `compose.yaml`).
- **Caching.** The security baseline adds `Cache-Control: no-store` to every
  response without a `Cache-Control` of its own; the standard `/static` mount
  sets its own. Downloads, feeds and other responses that browsers or a CDN
  may cache choose a policy with `rullst::cache_control::public(max_age)` or
  `private(max_age)` (13.0.0-alpha.2) on their route, or set the header
  themselves; see [downloads and feeds](10-static-assets-and-compression.md#downloads-and-feeds).

---

## Running the generated files

```bash
cargo rullst deploy
```

Then follow [Deploy to a Linux VM](../vm-deployment.md).

### Client addresses behind the proxy

Every deployment puts a reverse proxy in front of the application, so the
socket peer of each request is the proxy. Rate limits, such as the SaaS
starter's ten login/registration submissions per client per minute, would then
treat every visitor as one client. The generated Caddy container has a fixed
address on the private network in `compose.yaml`, and `cargo rullst deploy`
lists that address in `Rullst.toml` (an existing `trusted_proxies` list is
left unchanged, with a warning):

```toml
[security]
trusted_proxies = ["172.31.250.10"]
```

Keep `compose.yaml` and `Rullst.toml` in step, and change the subnet if it
overlaps a network on the host. For any other proxy or platform, list the
networks it connects from, as documented by that platform. List only proxy
networks: any host inside them can choose the client address.

### Container lifecycle boundary

The `Dockerfile` written by `cargo rullst deploy`, `cargo rullst dockerize` or
`cargo rullst new --docker` builds a non-root runtime image with production
host/port defaults and local assets, but never an application secret. For a
SQLite project (the starters' default) the image sets
`DATABASE_URL=sqlite:///app/data/rullst.db?mode=rwc`, so the database lives in
`/app/data`; configure the target platform so that directory remains owned by
UID/GID 10001 when attaching persistent storage (the generated `compose.yaml`
mounts a volume there).

Apply migrations in one bounded pre-deployment job and start application
replicas only after it succeeds. Running migrations inside every replica is
opt-in ([startup migrations](#startup-migrations)), not the default. Starters
created by `cargo rullst new` mount `rullst::health::health_router()`, which
answers `/health` and `/ready`. An application created otherwise must mount
those routes before configuring probes or monitoring; a redirect to login is
not a health signal.

### Startup migrations

Since 13.0.0-alpha.2 the server can apply pending migrations itself, after it
opens the database pool and before it binds its port. It is off by default.
Register the migrations and turn it on in code, per deployment, or both:

```rust,ignore
Server::new(router())
    .with_migrations(crate::migrations::get_migrations())
    .auto_migrate(true) // or leave it out and set RULLST_AUTO_MIGRATE=true
    .run(3000)
    .await?;
```

`RULLST_AUTO_MIGRATE` (`true` or `false`; `1`/`0`, `yes`/`no` and `on`/`off`
also work), from the process environment and then `.env`, overrides
`auto_migrate`, so a deployment that runs `db:migrate` as a separate job can
set `RULLST_AUTO_MIGRATE=false`. Projects that `cargo rullst new` generates
with an SQLx database (Blank, API and SaaS)
already call `with_migrations` but not `auto_migrate`, so setting
`RULLST_AUTO_MIGRATE=true` is enough; a project generated before
13.0.0-alpha.2 needs the line above in `src/main.rs`. A Turso starter applies
its migrations with its own `db:migrate` command and registers none with the
server, so `RULLST_AUTO_MIGRATE=true` there stops startup with
`startup migrations failed: no migrations are registered; call
Server::with_migrations`: leave it unset and run `db:migrate`.

Replicas that start together apply each migration once: the runner holds a
PostgreSQL advisory lock or a MySQL/MariaDB `GET_LOCK` lock, and for an SQLite
file an exclusive lock on `<database>.rullst-migrate.lock` (left on disk), while
it reads and applies the pending migrations; the others wait and then find
nothing to do. If a migration fails, or migrations are enabled without a
registry or a database, the server does not start: `run` returns
`ServerError::Migration` and the process exits with that error. A migration
must still be safe for the previous release, which keeps serving until the new
replicas are ready.
