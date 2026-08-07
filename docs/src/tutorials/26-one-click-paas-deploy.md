# Tutorial 26: Cloud Deployment & Rullst Foundry SSH Pipeline 🚀

Rullst provides two distinct, powerful deployment strategies tailored to different infrastructure needs:

1. **`cargo rullst deploy`**: 1-Click PaaS Cloud Wizard (Fly.io, Railway, Render, Local Docker Compose).
2. **`cargo rullst foundry:deploy`**: Enterprise Bare-Metal & Cloud VPS SSH Pipeline (Hetzner, DigitalOcean, AWS EC2, Linode, Vultr, Self-Hosted Servers).

---

## ⚡ Comparison: `deploy` vs `foundry:deploy`

| Feature | `cargo rullst deploy` (PaaS) | `cargo rullst foundry:deploy` (Foundry SSH) |
| :--- | :--- | :--- |
| **Primary Target** | Managed Cloud (Fly.io, Railway, Render) | Cloud VPS / Bare-Metal (Hetzner, DigitalOcean, AWS, Linode) |
| **Mechanism** | Platform CLI (`flyctl`, `railway up`) & Manifests | Automated SSH + SCP + Systemd / Docker |
| **Setup Required** | Platform Account & CLI installed | SSH Access (`Foundry.toml`) |
| **Config File** | `fly.toml`, `railway.json`, `render.yaml` | `Foundry.toml` (auto-gitignored) |
| **Migrations** | Executed in container entrypoint | Auto-executed via SSH remote `db:migrate` |
| **SSL Certificates** | Managed by PaaS provider | Auto-provisioned by Caddy / Let's Encrypt |

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

Rullst Foundry is an automated deployment engine for **any Linux server with SSH access** (Hetzner, DigitalOcean Droplets, AWS EC2, Linode, Vultr, or local hardware).

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
deploy_path = "/var/www/my_rullst_app"

[env]
APP_ENV = "production"
PORT = "3000"
DATABASE_URL = "postgres://user:password@localhost:5432/production_db"
JWT_SECRET = "super_secure_production_jwt_secret_key"
```

### Step 2: Deploy to Remote Server with 1 Command

```bash
cargo rullst foundry:deploy
```

### What Happens During `foundry:deploy`:
1. **Local Native Build**: Compiles optimized release binary (`cargo build --release`).
2. **Directory & Service Provisioning**: Ensures remote directory structure and Systemd / Docker service files exist on the server via SSH.
3. **Secure Transfer**: Uploads binary and static assets via `scp` with SHA-256 integrity verification.
4. **Remote Migration**: Executes pending database migrations (`cargo rullst db:migrate`) on the remote server.
5. **Zero-Downtime Reload**: Restarts the Systemd service and verifies HTTP health probes (`GET /health`).

---

## 💡 Summary & Best Practices
- Use **`cargo rullst deploy`** when hosting on serverless container platforms (Fly.io, Railway, Render).
- Use **`cargo rullst foundry:deploy`** when deploying to cost-effective VPS providers (Hetzner, DigitalOcean, AWS EC2) with full control over server resources and zero vendor lock-in.
