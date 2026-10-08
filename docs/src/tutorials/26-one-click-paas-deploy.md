# Tutorial 26: Guided Cloud Deployment & Foundry SSH Pipeline 🚀

Rullst provides two deployment scaffolds that require provider credentials,
application-specific review, and rollback planning:

1. **`cargo rullst deploy`**: guided PaaS manifest/CLI helper (Fly.io,
   Railway, Render, or local Docker Compose).
2. **`cargo rullst foundry:deploy`**: reviewed SSH pipeline for a compatible
   systemd-based Linux VPS.

---

## ⚡ Comparison: `deploy` vs `foundry:deploy`

| Feature | `cargo rullst deploy` (PaaS) | `cargo rullst foundry:deploy` (Foundry SSH) |
| :--- | :--- | :--- |
| **Primary Target** | Managed Cloud (Fly.io, Railway, Render) | Cloud VPS / Bare-Metal (Hetzner, DigitalOcean, AWS, Linode) |
| **Mechanism** | Platform CLI (`flyctl`, `railway up`) and manifests | SSH + SCP + systemd + an existing Caddy installation |
| **Setup Required** | Platform account, credentials, and CLI | Reviewed root or passwordless-sudo SSH access, systemd, Caddy, DNS and firewall policy |
| **Config File** | `fly.toml`, `railway.json`, `render.yaml` | `Foundry.toml` (auto-gitignored) |
| **Migrations** | Application/platform configuration | Not executed by the current Foundry command |
| **TLS Certificates** | Provider configuration | Requested by Caddy when DNS/network prerequisites are satisfied |

---

## 🛠️ Strategy 1: PaaS Cloud Deploy Wizard (`cargo rullst deploy`)

Launch the interactive PaaS deployment wizard:

```bash
cargo rullst deploy
```

Or target a specific platform directly:

```bash
# Deploy to Fly.io (Global Edge Containers)
cargo rullst deploy --platform=fly

# Deploy to Railway (Zero-config PaaS)
cargo rullst deploy --platform=railway

# Deploy to Render (Managed Cloud Services)
cargo rullst deploy --platform=render

# Scaffold Local VPS Production Stack (Docker Compose + Caddy SSL)
cargo rullst deploy --platform=vps
```

### Client addresses behind the proxy

Every platform puts a reverse proxy in front of the application, so the socket
peer of each request is the proxy. Rate limits, such as the SaaS and LMS
starters' ten login/registration submissions per client per minute, would then
treat every visitor as one client. `--platform=vps` pins its Caddy container to
`172.31.250.10` (on the `172.31.250.0/24` network in
`docker-compose.prod.yml`) and lists that address in `Rullst.toml`:

```toml
[security]
trusted_proxies = ["172.31.250.10"]
```

Change both files together if that subnet overlaps a network on the host. For
Fly.io, Railway and Render, list the networks their proxies connect from, as
documented by the provider; the command prints a reminder while none is set.
List only proxy networks: any host inside them can choose the client address.

### Container lifecycle boundary

Projects created with `cargo rullst new --docker` receive a non-root runtime
image with production host/port defaults and local assets, but never an
application secret. For an explicit SQLite project the generated database URL
uses `/app/data`; configure the target platform so that directory remains owned
by UID/GID 10001 when attaching persistent storage.

Apply migrations in one bounded pre-deployment job and start application
replicas only after it succeeds. Automatically running migrations inside every
replica creates an avoidable concurrent-startup race and is therefore not the
default. The generated `fly.toml`, `railway.json` and `render.yaml` probe
`/health` (Fly also `/ready`); starters created by `cargo rullst new` mount
`rullst::health::health_router()` for both. An application created otherwise
must mount those routes before configuring platform probes; a redirect to login
is not a health signal.

---

## 🏭 Strategy 2: Rullst Foundry SSH Pipeline (`cargo rullst foundry:*`)

Rullst Foundry is a bounded deployment helper for compatible systemd-based
Linux servers. Its current provisioning commands require root or passwordless
non-interactive `sudo`; it is not portable to every SSH host and does not
support IPv6 SCP targets.

> **Experimental.** `foundry:init` and `foundry:deploy` are experimental CLI
> generators: their manifest, generated scripts and flags may change between
> 13.x releases. See the [maturity tiers](../maturity.md#cli-generators).

### Step 1: Initialize `Foundry.toml`

```bash
cargo rullst foundry:init
```

This generates `Foundry.toml` at your project root and automatically adds it to `.gitignore` to protect sensitive server credentials.
An excerpt, edited with example values (the generated file also has `[deploy]`,
`[build]`, `[database]` and `[caddy]` sections):

```toml
[app]
name = "my_rullst_app"
domain = "api.mycompany.com"
port = 3000

[server]
host = "203.0.113.50"
user = "root"
ssh_key = "~/.ssh/id_ed25519"
ssh_port = 22

[env]
RULLST_ENV = "production"
APP_KEY = "REPLACE_WITH_A_STRONG_RANDOM_KEY"
DATABASE_URL = "sqlite:///opt/rullst/my_rullst_app/data/db.sqlite"
```

Caddy proxies to the `[app] port` (3000 when omitted) and the health check
probes it. Foundry passes that port to the service as `PORT`, so an `[env] PORT`
must name the same port (when `[app] port` is omitted, `[env] PORT` selects it).
Unless `[env]` sets `HOST` (or `RULLST_HOST`), Foundry also sets
`HOST="127.0.0.1"`, so the application's plain-HTTP port is reachable only
through Caddy on the server; set `HOST` only to expose it deliberately.

### Step 2: Run the reviewed deployment command

```bash
cargo rullst foundry:deploy
```

### What the current `foundry:deploy` does

1. Builds the selected profile and optional target locally and takes the
   executable Cargo reports for the package, wherever its target directory is.
2. Connects over SSH, checks the preinstalled `curl`, `systemctl`, `caddy` and
   `useradd` executables, creates `/opt/rullst/<app>/{bin,config,data}` and a
   dedicated system account (`rullst-<app>`, lowercase, with a digest suffix
   when the name needs shortening), and fails if that step fails. Only `data/`
   is owned by that account; earlier root-owned data is re-owned without
   following symlinks. Foundry does not install operating-system packages and
   never pipes an unpinned network script into a shell.
3. Uploads the application binary with `scp` to a staging path. It does not
   currently upload static directories or perform a separate remote checksum
   comparison.
4. Writes staged environment, Caddy, systemd and binary files, validates the
   candidate Caddy configuration, renames each staged file, then restarts the
   services. A validation/reload/restart failure aborts the command. The prior
   binary, environment, systemd unit, and global Caddyfile are retained as
   `.previous`, but rollback is manual and the application restart is not
   zero-downtime. This version manages one global `/etc/caddy/Caddyfile`; review
   that replacement before using the server for multiple independently managed
   sites. The unit runs the application as the dedicated account with
   `NoNewPrivileges`, `ProtectSystem=strict`, `ProtectHome`, `PrivateTmp`,
   `PrivateDevices` and no capabilities (only `CAP_NET_BIND_SERVICE` for an
   application port below 1024). The application can write only under
   `/opt/rullst/<app>/data`, so keep SQLite files, uploads and other state there;
   binaries and the root-only `config/.env` stay read-only to it.
5. Requires `GET /health` to succeed within ten bounded attempts before printing
   that the remote process answered locally. It does not prove public DNS, TLS,
   firewall, proxy, or external reachability.

SSH uses `StrictHostKeyChecking=accept-new`: verify the host fingerprint through
an independent channel before the first connection. The command does not
compare a separate remote checksum, run database migrations, back up/restore
data, coordinate multiple instances, or automatically roll back a failed
release.

---

## 💡 Summary & Best Practices
- Use **`cargo rullst deploy`** when hosting on serverless container platforms (Fly.io, Railway, Render).
- Use **`cargo rullst foundry:deploy`** only after reviewing the generated SSH,
  systemd, Caddy, secret, migration, backup, and rollback plan for the target VPS.
