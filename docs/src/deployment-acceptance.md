# Two-replica deployment acceptance

This v13 candidate exercises two independent Rullst application processes behind
a digest-pinned Caddy proxy and an owned Redis server. The local contract passes, alongside 242 Core and 163 Security library tests,
strict lints and five detected body-lifetime mutations. Hosted workspace and
installed-archive checks subsequently passed in
[PR #224](https://github.com/Rullst/Rullst/pull/224); the final release campaign
remains separate. It is a deployment test
and reference boundary, not a new load-balancer crate or an automatic cloud/VPS
deployment feature. Foundry's generated production configuration remains separate.

Run on Linux with Docker and the repository Rust toolchain:

```sh
python3 .github/check-deployment-proxy.py
```

The runner builds an explicitly ignored test fixture, creates temporary application
directories and fresh fixture keys, and starts uniquely named containers. Every
listener is loopback-only. No real account, application data, cloud credentials,
SSH changes or Docker socket mounted inside a container are required. The proxy
uses host networking solely to reach the owned loopback listeners; this is not
container network isolation. Its root filesystem and configuration are read-only,
it runs as an unprivileged UID, and its only retained capability is
`NET_BIND_SERVICE`, required by the official image binary's file capability.
The runner removes its own processes/containers on exit. Redis state is disposable.

## Executable boundary

| Case | Observed contract |
| --- | --- |
| Startup | Both dependency gates start unavailable. Core denies application admission; the proxy does not return a successful application response from an unready replica. |
| Availability | Once ready, both independent processes serve. Marking one dependency unavailable removes that replica after health-check propagation; making it ready restores participation. |
| Shared request budget | Both processes explicitly require the distributed Redis limiter and consume the same six-request fixture budget. A seventh request fails, including direct requests with forged forwarding headers. Redis loss returns `503`, with a bounded application timeout and no local fallback. |
| Forwarded metadata | Core derives its default identity from the transport socket. The proxy has no trusted inbound forwarding hops, sanitizes `X-Forwarded-For`, and removes `Forwarded`. Forged headers cannot replace the identity. |
| HTTP controls | The real `Server` production baseline supplies secure headers and CSRF enforcement. A route accepts a 1,024-byte body with a valid double-submit token and rejects 1,025 bytes. Missing CSRF is rejected. |
| In-flight response | A real streaming response returns its first chunk, remains counted during drain and finishes only when explicitly released. The draining replica stops receiving new application work while another replica continues serving. |
| WebSocket | The Security origin policy rejects missing/foreign origins. A valid connection exchanges frames through Caddy. Application-owned drain sends a restart close frame; ordinary HTTP body accounting is not used as WebSocket lifetime accounting. |
| Process termination | After its stream completes, the drained process shuts down within the fixture's deadline. Killing the remaining process causes the proxy to report no healthy upstream. |

The fixture's routing and control endpoints are test code. Its management channel
is the parent-owned stdin pipe, not an HTTP administration route. There are no
user accounts or domain authorization claims. A socket-level quota behind a proxy
conservatively combines clients from that peer; real per-user quotas must use an
authenticated application identity, or an explicitly reviewed trusted-hop policy.
Raw forwarding headers never supply a tenant, role or user identity.

## HTTP drain correction

`ApplicationLifecycle` now retains an admitted request through its handler and
the ordinary response body's data/trailers, releasing it exactly once on
completion, error or drop. Previously, the guard was released when the handler
returned response headers, which could report zero in-flight requests while a
stream remained active. The regression was observed failing before the fix.

The body wrapper preserves frames, size hints and end-of-stream state without
buffering. Local tests cover a pending body, multiple data/trailer frames, errors,
cancellation and an empty body. This is the lifetime observed at the middleware
boundary; buffering by another layer can consume that body before bytes reach
the client. It is not proof of TCP acknowledgement or successful client receipt.
Applications still terminate detached work and upgraded connections separately.

`wait_for_drain` has an explicit bounded wait and typed timeout. `Server`'s Axum
graceful-shutdown future itself does not impose a universal kill deadline; the
application/supervisor owns that deadline and the choice to abort unfinished work.

## Production differences

The fixture uses cleartext HTTP on loopback, fixed test budgets and aggressive
health intervals. It does not validate public TLS/ACME, a CDN, physical host or
region failure, Redis failover, persistence rollback, autoscaling or production
traffic capacity. Health changes take time to propagate; intermediate errors can
occur. No zero-downtime or exactly-once claim follows from a passing run.

The profile disables automatic proxy retries. Application retries of mutations
still require domain idempotency and durable transactional state. Multi-host
authentication must choose suitable shared storage; placing a process-local or
shared-local SQLite store behind a proxy does not make it cross-host durable.
The [shared passkey candidate](shared-passkey-ceremonies.md) addresses one such
state boundary and retains its own acceptance requirements.

Read-only cloud/VPS diagnostics, a generated multi-replica Foundry profile and
additional deployment topologies remain follow-up work. No firewall, SSH, IAM or
host security configuration is changed by this increment.

The reviewed proxy behavior follows Caddy's official
[reverse-proxy documentation](https://caddyserver.com/docs/caddyfile/directives/reverse_proxy),
including health checks, forwarding headers and upgraded streams. The fixture
configuration and runner pin the actual tested image independently of future
upstream releases.
