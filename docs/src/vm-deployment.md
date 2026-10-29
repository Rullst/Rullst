# Deploy to a Linux VM

This page is the shared part of the four provider guides
([Hetzner Cloud](deploy-hetzner.md), [DigitalOcean](deploy-digitalocean.md),
[Oracle Cloud](deploy-oracle-cloud.md) and [Google Cloud](deploy-google-cloud.md)).
It explains what happens on a virtual machine (VM) once you have one, in the
order you will do it. Each provider guide covers only what is different: how to
create the machine, open the firewall and point DNS.

If you have never run a server before, read this page once from top to bottom
first. Nothing here is specific to one company.

## What you will end up with

```text
Internet --> 80/443 --> Caddy (HTTPS, certificates) --> your Rullst app (port 3000)
                                                              |
                                                       database volume
```

- One VM running Linux (Debian or Ubuntu are the easiest to follow).
- A container engine: Docker or Podman.
- Two containers started by one file: your application and
  [Caddy](https://caddyserver.com/docs/), a web server that asks for and renews
  HTTPS certificates by itself.
- Only ports 22 (SSH), 80 and 443 reachable from the internet. The application
  port (3000) is never opened to the internet; only Caddy talks to it.

## 1. Generate the deployment files

In your project, on your own computer:

```bash
cargo rullst deploy
```

For a plain Linux VM this writes three files and does not contact any provider:

| File | Purpose |
| :--- | :--- |
| `Dockerfile` | Builds the release binary in a build stage (with a separate dependency-cache layer, so later builds are faster) and copies it into a small runtime image that runs as a non-root user. |
| `compose.yaml` | Describes the two containers (your app and Caddy), the private network between them and the volumes. The app reads its settings from `.env.production` and keeps its data in the `app_data` volume. It works with `docker compose` and with `podman compose`. |
| `Caddyfile` | Tells Caddy to serve the domain in the `DOMAIN` variable and to forward requests to the application. |

It also adds the Caddy container's fixed address to `[security]
trusted_proxies` in `Rullst.toml` (see section 5), and prints the next steps.
Existing files are kept: it writes each file only when it is missing.

Open all three and read them. They are a starting point, and you are the one
who deploys them. Do not commit files that contain secrets (see section 5).

The existing generated `Dockerfile` has a note you should keep in mind: run
`db:migrate` once as its own job before you start or replace the application
(see [migrations](tutorials/26-one-click-paas-deploy.md#startup-migrations)).

## 2. Create the VM and the DNS record

Follow your provider guide. In every case you need:

- **Size.** A small application (one process, SQLite or a small PostgreSQL)
  runs on 1 to 2 virtual CPUs and 1 to 2 GB of memory. The first image build
  can need more memory than running the app, so either add a swap file on the
  VM (see the note below), build the image on your own computer, or build it in
  CI and only pull it on the VM. Check each provider's current pricing and
  free-tier limits; they change.
- **Firewall.** Allow inbound TCP 22, 80 and 443 only (and UDP 443 if you
  want HTTP/3). Everything else stays closed.
- **SSH key.** Create the VM with your public SSH key and log in with
  `ssh <user>@<ip-address>`. Do not enable password logins.
- **DNS.** At your domain registrar or DNS host create an `A` record (and an
  `AAAA` record if the VM has an IPv6 address) for your domain pointing to the
  VM's public IP address. Caddy can only get a certificate after this record
  works, so check it with `dig +short your-domain.example` first.

> **Swap file (small VMs only).** If a build is killed with "out of memory",
> create a swap file once: `sudo fallocate -l 2G /swapfile && sudo chmod 600
> /swapfile && sudo mkswap /swapfile && sudo swapon /swapfile`. Add it to
> `/etc/fstab` to keep it after a reboot.

## 3. Install Docker or Podman on the VM

Use the official instructions for your distribution, so you get supported
packages:

- Docker Engine and the Compose plugin:
  <https://docs.docker.com/engine/install/>
- Podman: <https://podman.io/docs/installation> (on Debian and Ubuntu,
  `sudo apt install podman podman-compose` is enough; check that your release
  ships a recent version).

Check that it works:

```bash
docker compose version      # or: podman compose version
```

## 4. Get the application onto the VM

There are two simple ways. Pick one.

**A. Copy the source and build on the VM.** Easiest, but needs enough memory
(see the swap note above):

```bash
# on your computer, from the project directory
rsync -av --exclude target --exclude .git --exclude '.env*' ./ <user>@<ip>:~/app/
```

**B. Build the image elsewhere and move it.** Faster to start on a tiny VM:

```bash
# on your computer
docker build -t my-app:2026-10-29 .
docker save my-app:2026-10-29 | gzip | ssh <user>@<ip> 'gunzip | docker load'
```

If your computer and the VM have different CPU architectures (for example an
Apple Silicon laptop and an x86-64 VM, or an x86-64 laptop and an ARM VM), add
`--platform linux/amd64` or `--platform linux/arm64` to `docker build` so the
image matches the VM. A private registry (Docker Hub, GitHub Container
Registry, or your provider's) also works and replaces `docker save`/`load`.

## 5. Production settings that matter

Put these in a file on the VM named `.env.production`, next to
`compose.yaml` (never commit it, and keep it readable only by you:
`chmod 600 .env.production`). The generated `compose.yaml` passes it to the
`app` service, and Compose refuses to start while the file is missing:

| Setting | Why |
| :--- | :--- |
| `RULLST_ENV=production` | Turns on the production behavior: Secure cookies, strict headers, and a refusal to start with unsafe configuration. The generated `Dockerfile` already sets it. |
| `APP_KEY=<long random value>` | The application secret that protects sessions. Create one with `openssl rand -hex 32`. Never reuse a development value. |
| `DATABASE_URL=...` | For SQLite (the starters' default) the `Dockerfile` that `cargo rullst deploy` writes already points it at `/app/data/rullst.db`, inside the `app_data` volume; leave it out. A `Dockerfile` from an earlier `deploy` or `dockerize` may lack that line (`deploy` warns): add `ENV DATABASE_URL=sqlite:///app/data/rullst.db?mode=rwc` to its last stage, or set it here. For PostgreSQL or MySQL, the connection URL of your database. |
| Other secrets | Mail, payment and AI keys also go here, as environment variables, never in the source or the image. |

You can check this file before you start anything, without sending it
anywhere: `cargo rullst deploy:doctor --env-file .env.production`
(see [Local Deployment Configuration Diagnostic](deployment-diagnostic.md)).

**Trusted proxy.** Every request reaches your application through Caddy, so
the application sees Caddy's address as the visitor's address. Rate limits and
logs would treat all visitors as one. `cargo rullst deploy` already told
Rullst to trust Caddy in `Rullst.toml` (if the list existed, it left it
unchanged and printed a warning; then add the address yourself):

```toml
[security]
trusted_proxies = ["172.31.250.10"]
```

Use the fixed address that `compose.yaml` gives the Caddy container (read it in
the file, and keep the two in step). List only your own proxy: any machine in
that list can choose the client address. Background:
[Client addresses behind the proxy](tutorials/26-one-click-paas-deploy.md#client-addresses-behind-the-proxy).

**Health checks.** Starters created by `cargo rullst new` answer `GET /health`
(the process is alive) and `GET /ready` (the application is ready for
traffic). Use them for monitoring:

```bash
curl -fsS https://your-domain.example/health
```

**Optional monitor token.** If you enable the opt-in monitoring endpoint, set
`RULLST_MONITOR_TOKENS` in the same file and read
[Production monitoring](production-monitoring.md) first. It is off by default;
do not publish its token.

## 6. Start it

Caddy serves the domain in the `DOMAIN` variable. Put it in a file named
`.env` next to `compose.yaml` (Compose reads it for the variables in the file;
it holds no secrets), or write `DOMAIN=your-domain.example` before each
command:

```bash
echo 'DOMAIN=your-domain.example' > .env
```

**Rootless Podman** cannot listen on ports below 1024 by default. Allow it
once: `echo 'net.ipv4.ip_unprivileged_port_start=80' | sudo tee
/etc/sysctl.d/99-rullst-ports.conf && sudo sysctl --system`. Docker and
rootful Podman need nothing.

On the VM, in the directory that holds the generated files:

```bash
docker compose up -d --build      # or: podman compose up -d --build
docker compose ps
docker compose logs -f
```

If you use the Rullst database, run the migrations once, as their own step,
before sending traffic (see the note on migrations above):

```bash
docker compose run --rm app /app/<binary-name> db:migrate
```

(`<binary-name>` is your package name from `Cargo.toml`. The runtime image has
no shell, so call the binary directly.)

When the DNS record is correct and ports 80 and 443 are open, Caddy gets a
certificate on the first request. Open `https://your-domain.example` in a
browser. If it does not work, read `docker compose logs caddy`: the usual
causes are a wrong DNS record, a closed port, or a typo in the domain.

## 7. Logs

The application writes its logs to standard output, and the container engine
stores them:

```bash
docker compose logs --tail 200 app
docker compose logs -f caddy
```

On the host, `journalctl` shows the engine's own messages
(`sudo journalctl -u docker`). The default log driver can fill the disk on a
small VM, so limit it: in Docker, set `"log-driver": "local"` (or a
`max-size` and `max-file` option) in `/etc/docker/daemon.json` and restart
Docker. Podman has `--log-opt max-size=...` for the same purpose.

## 8. Back up the database volume

A backup that has never been restored is only a hope. Do both steps once.

**SQLite.** Use SQLite's own backup command so the copy is consistent while
the app is running (the runtime image has no shell, so run it from a helper
container that mounts the same volume, or from the host path of the volume):

```bash
docker run --rm -v <volume-name>:/data -v "$PWD":/backup alpine:3 \
  sh -c 'apk add --no-cache sqlite >/dev/null && sqlite3 /data/rullst.db ".backup /backup/rullst-$(date +%F).db"'
```

Find the volume name with `docker volume ls`. The generated database file name
and location are in the `DATABASE_URL` you configured.

**PostgreSQL or MySQL.** Use `pg_dump` or `mysqldump` (or your managed
database's snapshots) on a schedule.

Then copy the backup off the VM (to object storage or another machine) with a
scheduled job such as `cron`, and every few months restore one into a
temporary copy of the app to prove it works. Provider "snapshots" of the whole
VM are a useful extra, but they are not a substitute for a database-level
backup. Check each provider's current pricing for snapshots and backups.

## 9. Update and roll back with image tags

Never deploy the image called `latest`: you could not say afterwards what is
running. Give every build a tag you can read, such as the date or the Git
commit:

```bash
docker build -t my-app:$(git rev-parse --short HEAD) .
```

Make the `app` service in `compose.yaml` use that tag (its `image:` line), then:

```bash
docker compose run --rm app /app/<binary-name> db:migrate   # only if there are new migrations
docker compose up -d app
curl -fsS https://your-domain.example/ready
```

To **roll back**, change the `image:` line back to the previous tag and run
`docker compose up -d app` again. Keep the last few images on the VM
(`docker image ls`) and do not run `docker image prune -a` before you are sure
the new version is fine. A rollback does not undo a database migration: write
migrations so the previous version still works with the new schema, and take a
backup (section 8) before any change that removes or rewrites data.

Caddy keeps serving while the app container is replaced, but a single VM has a
short gap during the restart. That is normal for this setup; if you need zero
downtime, you need more than one machine, which is outside this guide
([Kubernetes](advanced-deployment.md#kubernetes) is one option).

## 10. Keep the VM healthy

- Apply operating-system security updates regularly
  (`sudo apt update && sudo apt upgrade`); consider unattended upgrades.
- Keep SSH key-only and, if you wish, install `fail2ban`.
- Watch disk space (`df -h`) and memory (`free -h`).
- Keep the VM's firewall at 22, 80 and 443. Do not publish database ports.

## Where next

- Providers: [Hetzner Cloud](deploy-hetzner.md),
  [DigitalOcean](deploy-digitalocean.md),
  [Oracle Cloud](deploy-oracle-cloud.md), [Google Cloud](deploy-google-cloud.md).
- Other ways to run the same image: [Advanced deployment](advanced-deployment.md).
- Before real users: the [production monitoring](production-monitoring.md) and
  [security architecture](security-architecture.md) pages.
