# Migration guide: v11-era dependencies to v12

There is no repository tag for a `rullst` umbrella v11 release. The v5/v6 source
line did, however, consume ecosystem packages such as `rullst-connect = 11`.
This guide therefore targets applications whose manifests or lockfiles contain
v11 Rullst ecosystem crates, and any untagged development snapshot described as
“v11”. Record the exact starting commit and dependency graph before proceeding.

Follow the common [safe upgrade procedure](migration-v12.md).

## 1. Inventory before editing

```bash
cargo tree -e features
cargo metadata --format-version 1
```

Save the output and list every direct or transitive `rullst-*` package. In v12,
all 16 published packages must use the same release version. Do not update only
`rullst-connect` or only the umbrella crate.

Example explicit selection:

```toml
[dependencies]
rullst = {
    version = "12.0.0-rc.1",
    default-features = false,
    features = ["orm", "queue-sqlite", "auth", "oauth", "security"]
}
```

If the project uses renamed dependencies or local paths, update those entries
manually; the upgrade command intentionally changes only standard versioned
Rullst dependency keys.

## 2. Connect and authentication

V12 `rullst-connect` keeps provider-neutral OAuth/OIDC types available without a
web adapter. Select the adapter explicitly:

- `axum` for Axum extractors and the local mock IdP router;
- `actix` for Actix extractors;
- `axum-session` for Axum plus `tower-sessions` integration;
- `retry` only when the Connect retry client is intended;
- `mock` only for deterministic offline provider modules.

Empty credentials choose explicit offline behavior. Custom-provider
configuration errors and live callback/state validation remain fallible. Re-test
PKCE, state, redirect URL allowlists, session binding, logout, refresh, and
unknown-key JWKS refresh behavior.

`rullst-auth` enables Connect integration through its `oauth` feature, while the
umbrella `rullst` facade exposes Connect through its own `oauth` feature. Review
which layer the application imports instead of enabling both reflexively.

## 3. Adopt the v12 runtime contracts

- Use explicit `routes!` registration; compatibility route attributes do not
  perform runtime registration.
- Propagate startup, pool, migration, provider, and server errors.
- Review the changed default features (`orm` and `queue-sqlite`).
- Select at most one `strict-*` database backend.
- Build Nexus with an explicit access policy and keep Studio on debug loopback.
- Mark tenant-owned Nexus models with an explicit text tenant column and supply
  a membership-derived `TenantContext`; install the audit table before enabling
  `with_required_audit()`.
- Re-run IDOR, tenant-isolation, CSRF/CORS, webhook, and trusted-proxy negative
  tests.

## 4. Review AI and tool assumptions

V12 has a machine-readable [provider capability matrix](ai-provider-capabilities.md).
Built-in live transports have configurable request deadlines. The separate
`StreamingAiClient` enforces bounded output and explicit cancellation, and an
exact OpenAI-compatible configuration may opt into strict incremental SSE.
Provider-native tool calling, automatic provider retries, ordinary
non-streaming cancellation and streaming for other provider protocols are not
uniform transport capabilities. `ToolRegistry` is guarded local dispatch
infrastructure, not an authorized autonomous-agent boundary.

## Completion gate

Compile both the application's exact feature set and the intended release
profile. Run the full local trifecta, provider integration tests in non-production
accounts, database restore/migration/rollback, and network-exposure checks. Keep
the previous artifact and database restore procedure available until the v12
deployment has passed its observation window.
