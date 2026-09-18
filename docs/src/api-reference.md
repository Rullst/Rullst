# Find the API you need

There are two different meanings of **API** in this documentation:

- **Rust API:** the framework types, traits, functions and Cargo features your
  application imports. Use the versioned rustdoc references below for exact
  signatures and follow the book for application-level context.
- **HTTP API:** the endpoints your application exposes to a browser, mobile
  client or another service. Start with the REST learning path below; an
  OpenAPI document describes your endpoints, not the entire Rust framework.

This index targets **12.0.0**, the published stable release. It connects existing
guides; it does not certify every public symbol or implement the planned 12.1
updater or v13 capabilities. Check [capability boundaries](capability-status.md)
before interpreting a provider or blueprint name as complete product support.

## Rust reference: start with a task

| What you need | Application guide | Exact v12 Rust reference |
| --- | --- | --- |
| Choose imports and start an application | [First application](tutorials/01-hello-world.md) | [rullst facade](https://docs.rs/rullst/12.0.0/rullst/) |
| Route requests and compose middleware | [Routing](tutorials/08-routing-and-middlewares.md), [Core](crates/core.md) | [rullst-core](https://docs.rs/rullst-core/12.0.0/rullst_core/) |
| Query data and manage persistence | [ORM](crates/orm.md), [CRUD](tutorials/03-active-record-crud.md), [backend boundaries](polyglot-persistence.md) | [rullst-orm](https://docs.rs/rullst-orm/12.0.0/rullst_orm/) |
| Authenticate users and authorize access | [Auth](crates/auth.md), [ownership and roles](tutorials/13-rbac-authorization.md) | [rullst-auth](https://docs.rs/rullst-auth/12.0.0/rullst_auth/) |
| Compose request defenses | [Security](crates/security.md), [security architecture](security-architecture.md) | [rullst-security](https://docs.rs/rullst-security/12.0.0/rullst_security/) |
| Integrate social sign-in | [Connect](crates/connect.md) | [rullst-connect](https://docs.rs/rullst-connect/12.0.0/rullst_connect/) |
| Integrate payments and signed webhooks | [Capital](crates/capital.md), [billing tutorial](tutorials/19-saas-billing-capital.md) | [rullst-capital](https://docs.rs/rullst-capital/12.0.0/rullst_capital/) |
| Call local or cloud language models | [AI](crates/ai.md) | [rullst-ai](https://docs.rs/rullst-ai/12.0.0/rullst_ai/) |
| Send transactional email | [Mail](crates/mail.md) | [rullst-mail](https://docs.rs/rullst-mail/12.0.0/rullst_mail/) |
| Publish and consume messages | [Messaging](crates/messaging.md) | [rullst-messaging](https://docs.rs/rullst-messaging/12.0.0/rullst_messaging/) |
| Inspect the local app or compose an admin interface | [Studio](crates/studio.md), [Nexus](crates/nexus.md) | [rullst-studio](https://docs.rs/rullst-studio/12.0.0/rullst_studio/), [rullst-nexus](https://docs.rs/rullst-nexus/12.0.0/rullst_nexus/) |
| Generate code or prepare a framework upgrade | [CLI reference](cli_reference.md), [assisted upgrades](tutorials/36-assisted-framework-upgrades.md) | [cargo-rullst](https://docs.rs/cargo-rullst/12.0.0/cargo_rullst/) |

Search within the selected crate's rustdoc page for an exact symbol. Read its
module context and feature requirements as well as its signature. A facade
re-export and a direct dependency can expose different features; enabling a
feature does not configure credentials, an external service or authorization.
For your application's selected feature set, `cargo doc --no-deps --open`
builds its local documentation; remove `--no-deps` to include dependency APIs.
Run Cargo commands only in a trusted checkout: build scripts and procedural
macros can execute code during documentation builds too.

## HTTP/REST: a connected learning path

1. [Return typed JSON](tutorials/rest-api-quickstart.md). Run the endpoint and
   inspect its status, headers and body with curl.
2. [Compose routing and middleware](tutorials/08-routing-and-middlewares.md).
   Understand which layer receives or rejects a request first.
3. [Validate input](tutorials/07-forms-and-validation.md). This guide is
   form-oriented; its validation principles do not turn it into a complete JSON
   validation/error-handling tutorial. Define the JSON error contract explicitly.
4. [Persist records](tutorials/03-active-record-crud.md) and [manage migrations](tutorials/05-migrations-and-seeds.md).
   Design bounded pagination and test failed writes as well as successful reads.
5. Choose [session or token authentication](tutorials/12-jwt-and-session-auth.md)
   and enforce [ownership/role authorization](tutorials/13-rbac-authorization.md).
   Authentication alone must not grant access to another user's records.
6. [Expose a reviewed OpenAPI specification](tutorials/24-scalar-api-docs.md).
   The viewer does not infer a complete specification or security policy from
   arbitrary handlers. Read its CSP, asset and missing-specification boundaries.
7. Review [deployment and security boundaries](security-architecture.md) and
   [the compatibility policy](compatibility-policy.md) before exposing the app.

For your own API, test malformed and oversized input, authentication failure,
cross-owner/tenant denial, missing records, bounded pagination and error responses
that do not expose credentials or database details. Public demos and a compiling
example are not substitutes for these application tests.

## What still needs improvement

The existing chapters are useful but are not yet one complete, consistently
tested REST product walkthrough. A coherent CRUD example with JSON validation,
typed error responses, pagination, authorization negatives and matching OpenAPI
is still needed. Per-symbol reference reviews also need explicit feature,
error, concurrency and migration coverage. The
[API documentation plan](https://github.com/Rullst/Rullst/blob/v13/ROADMAP.md#api-documentation-quality)
tracks that work separately from this navigation index.
