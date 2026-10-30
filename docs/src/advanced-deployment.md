# Advanced deployment

This page is for people who already run servers and want to deploy a Rullst
application with a tool the CLI does not know about: Nix, Buildah, Podman
Quadlet or Kubernetes.

> **Read this first.** Everything below is illustrative. `cargo rullst` does
> **not** generate, run or test any of these files, and the Rullst project does
> not validate these snippets against every tool version. They show where
> Rullst's own requirements (the binary name, the port, the environment, the
> health routes, the trusted proxy) fit. Check each tool's official
> documentation, adapt the names, and test in a staging environment.
> If you are new to servers, start with
> [Deploy to a Linux VM](vm-deployment.md) instead.

## What a Rullst application needs from any platform

| Need | Detail |
| :--- | :--- |
| A binary | `cargo build --release --locked` produces one executable named after your package. It needs a glibc Linux (or a musl build you have tested) and CA certificates. |
| Environment | `RULLST_ENV=production`, `APP_KEY` (a long random secret), `DATABASE_URL`, and any provider keys, as environment variables. |
| Address and port | In production the server listens on `0.0.0.0`; set `PORT` (the generated image uses 3000) and, behind a proxy on the same host, `HOST=127.0.0.1`. |
| Probes | `GET /health` (process alive) and `GET /ready` (ready for traffic). Starters mount both. A redirect to login is not a health signal. |
| Trusted proxy | Behind any reverse proxy or ingress, set `[security] trusted_proxies` in `Rullst.toml` (or `Server::trusted_proxies`) to the proxy's address range, and only that. See [Client addresses behind the proxy](tutorials/26-one-click-paas-deploy.md#client-addresses-behind-the-proxy). |
| Migrations | Run `<binary> db:migrate` once per release, before new replicas serve traffic, or opt in to [startup migrations](tutorials/26-one-click-paas-deploy.md#startup-migrations). |
| Writable state | A SQLite file, uploads and similar state need a persistent writable directory. The generated image runs as UID/GID 10001. |

## Nix

A [Nix flake](https://nixos.org/manual/nix/stable/command-ref/new-cli/nix3-flake.html)
can build the release binary reproducibly. Use `rustPlatform.buildRustPackage`
(see the [nixpkgs Rust documentation](https://nixos.org/manual/nixpkgs/stable/#rust)):

```nix
{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-<release>"; # pin the release you test
  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      packages.${system}.default = pkgs.rustPlatform.buildRustPackage {
        pname = "my-app";            # your [package].name
        version = "0.1.0";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;
        nativeBuildInputs = [ pkgs.pkg-config ];
        buildInputs = [ pkgs.openssl ];  # only if a dependency needs it
      };
    };
}
```

```bash
nix build          # result/bin/my-app
```

Commit `flake.lock` so the inputs stay pinned. On NixOS you can then run the
binary from a `systemd.services.my-app` unit with `Environment=` or
`EnvironmentFile=` for the settings above; keep secrets out of the Nix store
(use a file outside it, such as one managed by sops-nix or agenix). The static
`static/` directory and `Rullst.toml` must be installed next to, or referenced
by, the service.

## Buildah (rootless image build)

[Buildah](https://buildah.io/) builds OCI images without a daemon and without
root. It can build the `Dockerfile` that `cargo rullst deploy` (or
`cargo rullst dockerize`) writes:

```bash
buildah bud --layers -t my-app:$(git rev-parse --short HEAD) .
buildah push my-app:<tag> docker://registry.example.com/me/my-app:<tag>
```

`--layers` keeps the dependency-cache stage between builds. Rootless builds need
`/etc/subuid` and `/etc/subgid` entries for your user (see the
[Buildah tutorials](https://github.com/containers/buildah/tree/main/docs/tutorials)).
For a different CPU architecture add `--platform linux/arm64` (this needs QEMU
emulation or a native Arm builder). Use the resulting image with Podman,
Docker or Kubernetes. Image and Kubernetes names must be lowercase and use `-`,
not `_`.

## Podman Quadlet (systemd unit for the container)

[Quadlet](https://docs.podman.io/en/latest/markdown/podman-systemd.unit.5.html)
lets systemd run a Podman container from a small `.container` file.
For a rootless service put it in `~/.config/containers/systemd/my-app.container`
(system-wide: `/etc/containers/systemd/`):

```ini
[Unit]
Description=My Rullst application
After=network-online.target

[Container]
Image=registry.example.com/me/my-app:2026-10-29
PublishPort=127.0.0.1:3000:3000
EnvironmentFile=%h/my-app/.env.production
Volume=%h/my-app/data:/app/data:Z,U
HealthCmd=none

[Service]
Restart=always

[Install]
WantedBy=default.target
```

```bash
systemctl --user daemon-reload
systemctl --user start my-app.service
loginctl enable-linger "$USER"   # keep it running when you log out
```

The generated runtime image has no shell, so container-level `HealthCmd`
checks do not work; use `HealthCmd=none` and probe `/health` and `/ready` from
outside (your reverse proxy or monitoring). Put a reverse proxy such as
Caddy in front for HTTPS, publish the application port to `127.0.0.1` only, and
set `trusted_proxies` to the address the proxy's connections arrive from inside
the container (with rootless port forwarding this can be a gateway address
rather than `127.0.0.1`), and nothing broader. `:U` makes Podman chown the
volume to the container user (10001). Pin the image to a tag or digest and
change it to update; to roll back, set the previous tag and restart the unit.

## Kubernetes

You can write the manifests yourself, or start from `cargo rullst make:k8s`
(see [Tutorial 25](tutorials/25-kubernetes-deployment.md)); the generated files
also need review. The outline:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata: { name: my-app }
spec:
  replicas: 2
  selector: { matchLabels: { app: my-app } }
  template:
    metadata: { labels: { app: my-app } }
    spec:
      containers:
        - name: my-app
          image: registry.example.com/me/my-app@sha256:<digest>
          ports: [{ containerPort: 3000 }]
          env:
            - { name: RULLST_ENV, value: production }
            - name: APP_KEY
              valueFrom: { secretKeyRef: { name: my-app, key: app-key } }
            - name: DATABASE_URL
              valueFrom: { secretKeyRef: { name: my-app, key: database-url } }
          startupProbe:
            httpGet: { path: /health, port: 3000 }
            failureThreshold: 30
            periodSeconds: 2
          livenessProbe:
            httpGet: { path: /health, port: 3000 }
          readinessProbe:
            httpGet: { path: /ready, port: 3000 }
---
apiVersion: v1
kind: Service
metadata: { name: my-app }
spec:
  selector: { app: my-app }
  ports: [{ port: 80, targetPort: 3000 }]
---
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata: { name: my-app }
spec:
  ingressClassName: nginx
  tls: [{ hosts: [app.example.com], secretName: my-app-tls }]
  rules:
    - host: app.example.com
      http:
        paths:
          - { path: /, pathType: Prefix, backend: { service: { name: my-app, port: { number: 80 } } } }
```

Points to check against the [Kubernetes documentation](https://kubernetes.io/docs/home/):

- **Probes.** Liveness on `/health` (process only, so a database outage does
  not cause restart loops) and readiness on `/ready`. The default routes return
  200 while the process serves; the lifecycle-aware router in
  [Tutorial 25](tutorials/25-kubernetes-deployment.md#step-2-mount-lifecycle-aware-health-routes)
  reports startup, dependencies and drain on `/ready`.
- **Trusted proxy.** The application sees the ingress controller's pod address,
  not the visitor's. Set `trusted_proxies` in `Rullst.toml` to the pod network
  range of your ingress controller (look it up in your cluster) and nothing
  broader.
- **Migrations.** Run `db:migrate` as a Kubernetes `Job` (or an init step with
  a lock) before rolling out a new Deployment, not in every replica unless you
  enabled startup migrations.
- **Secrets, storage and security.** Use Secrets (or an external secrets
  manager) for credentials, a database that is not a file in a replicated Pod,
  a non-root `securityContext`, resource requests/limits, a PodDisruptionBudget
  and a NetworkPolicy. See the checklist at the end of Tutorial 25.
- **Rollback.** `kubectl rollout undo deployment/my-app` returns to the previous
  ReplicaSet; it does not undo database migrations.

## Where next

- [Deploy to a Linux VM](vm-deployment.md) and the provider guides.
- [Two-Replica Deployment Acceptance](deployment-acceptance.md) for what the
  project itself tests for more than one instance.
- [Local Deployment Configuration Diagnostic](deployment-diagnostic.md).
