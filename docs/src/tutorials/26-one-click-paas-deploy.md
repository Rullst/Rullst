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

---

## 🏭 Strategy 2: Rullst Foundry SSH Pipeline (`cargo rullst foundry:*`)

Rullst Foundry is a bounded deployment helper for compatible systemd-based
Linux servers. Its current provisioning commands require root or passwordless
non-interactive `sudo`; it is not portable to every SSH host and does not
support IPv6 SCP targets.

### Step 1: Initialize `Foundry.toml`

```bash
cargo rullst foundry:init
```

This generates `Foundry.toml` at your project root and automatically adds it to `.gitignore` to protect sensitive server credentials:

```toml
# Foundry.toml — Rullst Deployment Manifest
[app]
name = "my_rullst_app"
domain = "api.mycompany.com"

[server]
host = "203.0.113.50"
user = "root"
ssh_port = 22
ssh_key = "~/.ssh/id_ed25519"

[env]
RULLST_ENV = "production"
PORT = "3000"
DATABASE_URL = "sqlite:///opt/rullst/my_rullst_app/data/db.sqlite"
APP_KEY = "REPLACE_WITH_A_STRONG_RANDOM_KEY"
```

### Step 2: Run the reviewed deployment command

```bash
cargo rullst foundry:deploy
```

### What the current `foundry:deploy` does

1. Builds the selected profile and optional target locally.
2. Connects over SSH, checks the preinstalled `curl`, `systemctl`, and `caddy`
   executables, creates `/opt/rullst/<app>/{bin,config,data}`, and fails if that
   step fails. Foundry does not install operating-system packages and never pipes
   an unpinned network script into a shell.
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
   sites.
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
