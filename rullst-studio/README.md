# Rullst Studio 📊

`rullst-studio` is the built-in, local-first administration and monitoring
dashboard for Rullst. It exposes bounded database, queue and in-process
telemetry views from the sources explicitly supplied by the application.

## ✨ Features

- **Database inspector:** Browse configured tables and schema information. This is not a general production CRUD guarantee.
- **Worker queue monitoring:** Inspect a supplied Rullst queue and request retries through the local dashboard.
- **Tracing and telemetry:** Visualize in-process sources exposed by `rullst-core`; disconnected probes remain `Unavailable`.
- **Local-first security:** The supported launcher binds to loopback, verifies
  the direct peer and local `Host` authority, and requires same-origin `Origin`
  on mutations.

## 🚀 Quickstart

Add `rullst-studio` to your project:

```bash
cargo add rullst-studio
```

### Launching the Studio

The supported v12 mode is a standalone debug server. `run_studio` and
`Studio::into_router(LocalStudioAccess::loopback_only())` reject release builds
and requests whose direct peer is not verified as loopback. Servers composing
the router manually must preserve Axum `ConnectInfo<SocketAddr>`. Non-local
`Host`, cross-origin requests, and unsafe requests without `Origin` fail closed.

The earlier `StudioLayer` embedded-production idea was never implemented.
Keeping an authenticated shared Studio is worthwhile, but it needs its own
explicit identity/RBAC/TLS policy before it can become a supported mode.

**CLI Launch:**

If you don't want to embed it, you can launch it statelessly via the Rullst CLI:

```bash
cargo rullst studio
```

## 🔐 Security Audit

`rullst-studio` currently supports verified-loopback development access. It does
not provide a built-in shared-intranet or production authentication mode. Do not
expose raw subrouters publicly; a future shared mode must fail closed behind
application-owned authentication, administrator authorization, TLS, and network
policy.

Migration actions remain CLI-driven because standalone Studio has no migration
registry. Queue, revenue, security, AI, and telemetry views expose only supplied
process-local sources; unsupported operations or disconnected integrations are
reported explicitly.

## 📚 Documentation

For supported usage and security boundaries, see the
**[Rullst Book](https://rullst.github.io/Rullst/book/index.html)** and its
capability ledger. A production/shared Studio mode is not currently supplied.
