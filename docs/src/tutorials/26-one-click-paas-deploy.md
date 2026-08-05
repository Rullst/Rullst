# Tutorial 26: 1-Click PaaS Cloud Deployment 🚀

Deploy your Rullst application to Fly.io, Railway, Render, or a production VPS with 1 command.

---

## 🛠️ Step 1: Run the Deploy Wizard

```bash
cargo rullst deploy
```

Or target a specific platform:

```bash
# Deploy to Fly.io
cargo rullst deploy --platform=fly

# Deploy to Railway
cargo rullst deploy --platform=railway

# Deploy to Render
cargo rullst deploy --platform=render

# Scaffold Production VPS (Docker Compose + Caddy SSL)
cargo rullst deploy --platform=vps
```

---

## 🔒 VPS Production Deployment

For self-hosted VPS setups, Rullst generates `docker-compose.prod.yml` and `Caddyfile`:

```bash
DOMAIN=api.mycompany.com docker compose -f docker-compose.prod.yml up -d --build
```

Caddy automatically provisions free Let's Encrypt SSL certificates!

---

## 💡 Key Takeaways
- Scaffolds platform configuration files if missing.
- Integrates health probes (`/health`, `/ready`) automatically into deployment manifests.
