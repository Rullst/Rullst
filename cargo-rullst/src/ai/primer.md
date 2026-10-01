Rullst is a full-stack Rust web framework built on Axum and Tokio. It favours
explicit, compile-time checked code over runtime reflection: routes are
declared with the `routes!` macro, HTML is rendered on the server with the
`html!` macro (with HTMX for interactivity), and data lives in Active Record
models derived with `#[derive(Orm)]`. The `cargo rullst` CLI scaffolds most
building blocks. This primer summarises the conventions; when it and the
project disagree, the project's own code and `AGENTS.md` win.

## Project layout

A generated application looks like this (not every folder exists until a
scaffold creates it):

```text
my_app/
├── Cargo.toml          # depends on `rullst` (features select optional crates)
├── Rullst.toml         # framework configuration, e.g. [database] url, [security]
├── .env / .env.example # local settings; .env holds secrets and is never committed
├── AGENTS.md           # project instructions for AI assistants (user-owned)
├── static/             # CSS and other static assets
└── src/
    ├── main.rs         # bootstrap, `artisan!` migrations hook, routes, Server
    ├── controllers/    # async request handlers, one module per resource
    ├── models/         # `#[derive(Orm)]` Active Record models
    ├── migrations/     # timestamped migration modules + generated mod.rs
    ├── middlewares/    # Tower/Axum middleware functions
    ├── pages/          # shared HTML layouts and views
    ├── workers/        # background workers
    └── islands/        # optional Wasm islands (client components)
```

Naming: files and modules in `snake_case` (`blog_post.rs`), types in
`PascalCase` (`BlogPost`), URL paths in lowercase kebab-case
(`/user-profiles`), database tables and columns in `snake_case`, tables
plural (`blog_posts`).

## Routing

Routes are declared in `src/main.rs` with `routes!`. Each entry is
`method("path" => handler)`; the methods are `get`, `post`, `put`, `patch`
and `delete`. Path parameters use Axum 0.8 braces: `/posts/{id}`.

```rust
use rullst::{routes, Server};

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    rullst::artisan!(crate::migrations::get_migrations());

    let router = routes![
        get("/" => controllers::home::index),
        get("/posts" => controllers::posts::index),
        // rullst-access: owner — readers may only open their own drafts
        get("/posts/{id}" => controllers::posts::show),
        post("/posts" => controllers::posts::store),
    ]
    .layer(rullst::server::from_fn(rullst::security::csrf_middleware))
    .merge_axum(rullst::health::health_router())
    .layer(rullst::server::from_fn(rullst::security::headers_middleware));

    Server::new(router).run(3000).await?;
    Ok(())
}
```

- `rullst::artisan!` intercepts `cargo rullst db:*` commands before the
  server starts; keep it first in `main`.
- `Router` also offers `nest(path, router)`, `merge_axum(axum_router)`,
  `layer(layer)` and `fallback(handler)`. Raw Axum types are available under
  `rullst::web::axum`.
- Handlers are ordinary async functions returning `impl IntoResponse`
  (`rullst::response::IntoResponse`), `Html<String>`, `Json<T>` or a
  `Result` of those.
- Extractors come from `rullst::server`: `Path`, `Form`, `Json`, `Extension`.

Every route with a path parameter must have an adjacent marker comment on
the line before it, `// rullst-access: public|owner|role|admin — reason`.
`cargo rullst audit --idor` fails without it. `public` is accepted only for
GET routes; `owner` requires an ownership guard in the handler (see Security).

## Controllers

`cargo rullst make:controller posts` creates `src/controllers/posts.rs` with
`index`, `show`, `store`, `update` and `delete` handlers (add `--api` for JSON
handlers) and registers the module. Wire the handlers into `routes!`
yourself. A typical HTML handler:

```rust
use rullst::{html, response::{Html, IntoResponse}, security::CsrfToken, server::Extension};

pub async fn index(Extension(csrf): Extension<CsrfToken>) -> impl IntoResponse {
    let title = "Posts";
    Html(html! {
        <main class="max-w-2xl mx-auto p-6">
            <h1 class="text-2xl font-bold">{title}</h1>
            <form method="post" action="/posts">
                <input type="hidden" name="_token" value={csrf.as_str()} />
                <input name="title" required="true" />
                <button type="submit">"Create"</button>
            </form>
        </main>
    })
}
```

For JSON APIs use `rullst::server::Json`:

```rust
use rullst::server::{IntoResponse, Json, Path};

pub async fn show(Path(id): Path<i32>) -> impl IntoResponse {
    Json(serde_json::json!({ "id": id }))
}
```

## The html! macro

- Text must be a quoted string literal: `<p>"Hello, " {name} "!"</p>`.
  Unquoted words are not text.
- `{expr}` interpolates a value and HTML-escapes it. Dynamic URL attributes
  (`href`, `src`, `action`, ...) also neutralise `javascript:` URLs.
- Attribute values are quoted strings or `{expr}`: `class="card"`,
  `value={csrf.as_str()}`.
- Boolean attributes must be written with an explicit quoted value:
  `required="true"`, `disabled="true"`, `checked="true"`.
- A dynamic value for an `on*` or `hx-on*` attribute does not compile; keep
  event handlers static and pass data through `data-*` attributes.
- `html!` returns a `String`. To nest fragments (for example a list), build
  each item with `html!`, collect them into a `String`, and embed the result
  with `{rullst::html::RawHtml(items)}`. Only wrap HTML produced by `html!`
  or other trusted code in `RawHtml`; never user input.

```rust
let rows: String = posts
    .iter()
    .map(|post| html! { <li>{post.title.as_str()}</li> })
    .collect();
let page = html! { <ul>{rullst::html::RawHtml(rows)}</ul> };
```

## HTMX and pages

Rullst prefers server-rendered HTML with HTMX over client-side bundles.
`rullst::htmx::HtmxRequest` is an extractor that tells whether a request came
from HTMX, and `rullst::htmx::render_page_with_lang(&htmx, "en", "Title",
content)` returns a full page for normal requests and only the fragment for
HTMX requests. Forms that use `hx-post` still need the `_token` field (or an
`X-CSRF-Token` header).

## Forms and validation

Use `ValidatedForm<T>` (HTML forms) or `ValidatedJson<T>` (JSON) with a DTO
deriving `serde::Deserialize` and `rullst::Validate`:

```rust
use rullst::{Validate, ValidatedForm};
use serde::Deserialize;

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePost {
    #[validate(length(min = 3, max = 120))]
    pub title: String,
}

pub async fn store(ValidatedForm(form): ValidatedForm<CreatePost>) -> impl rullst::response::IntoResponse {
    // form.title is validated here
    rullst::response::Html(format!("Saved {}", form.title.len()))
}
```

Invalid input becomes a bounded 400/413/415/422 response (an HTML fragment
for HTMX requests). Never trust owner or tenant identifiers submitted in a
form; derive them from the authenticated user.

## Models (rullst-orm)

`cargo rullst make:model Post --migration` creates `src/models/post.rs`, adds
`pub mod models;` when missing, and creates a `create_posts` migration.

```rust
use rullst::db::{Orm, FromRow};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "posts")]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub published: bool,
    pub author_id: i32,
}
```

- Every SQLx model needs a named `id: i32` field. `id: 0` means "not saved".
- Generated methods: `Post::all()`, `Post::find(id)` (returns
  `Result<Option<Post>, _>`), `post.save()` (INSERT when `id` is 0, otherwise
  UPDATE), `post.delete()` and `Post::query()`.
- The query builder binds every value as a parameter:
  `Post::query().where_eq("published", true).order_by_desc("id").limit(20).get().await?`.
  Other filters: `where_not_eq`, `where_gt`, `where_lt`, `where_like`,
  `where_in`, `where_null`, `where_not_null`, `where_between`, `or_where`;
  terminals: `get`, `first`, `count`, `paginate(page, per_page)`.
- Column names passed to the builder must be application constants, never
  request input.
- A `deleted_at: Option<String>` field opts the model into soft deletes.
- Avoid reserved words as table or column names (`order`, `group`, `user`
  on PostgreSQL); generated SQL does not quote identifiers.
- Field attributes are strict: unknown `#[orm(...)]` or `#[sqlx(...)]` options
  fail compilation. `#[orm(encrypted)]` encrypts a field at rest.

## Migrations

`cargo rullst make:migration add_slug_to_posts` creates
`src/migrations/m<timestamp>_add_slug_to_posts.rs` and regenerates
`src/migrations/mod.rs`. A create-table migration looks like:

```rust
use rullst_orm::schema::{Schema, Migration};
use rullst_orm::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str {
        "m<timestamp>_create_posts" // the generated file stem
    }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("posts", |table| {
            table.id();
            table.string("title");
            table.boolean("published");
            table.integer("author_id");
            table.timestamps();
        }).await
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("posts").await
    }
}
```

Column helpers: `id`, `string`, `integer`, `big_integer`, `float`, `boolean`,
`enum_col(name, variants)`, `timestamps`, `soft_deletes`; modifiers
`nullable()`, `not_null()` and `default(..)`. Never edit a migration that has
already been applied; add a new one. The user applies migrations with
`cargo rullst db:migrate` and checks them with `cargo rullst db:status`.

## Database configuration

The database URL is resolved from the process `DATABASE_URL`, then
`DATABASE_URL` in `./.env`, then `[database] url` in `Rullst.toml`. There is
no implicit fallback database. Never print, log or commit connection strings
or `.env` values. Application settings are read with
`rullst::config::project_setting`, which also never overrides the process
environment.

## Security rules (mandatory)

- Keep `csrf_middleware` and `headers_middleware` on production routers.
  Every state-changing HTML form includes
  `<input type="hidden" name="_token" value={csrf.as_str()} />`, using the
  `Extension<rullst::security::CsrfToken>` extractor; HTMX may send the
  `X-CSRF-Token` header instead. A `multipart/form-data` form must place
  `_token` before any file input.
- Parameterised routes (`/{id}`) must enforce ownership: load the stored
  record, then call `RbacGuard::authorize_owner_or_role(&user, &stored_owner,
  "role")` (or `authorize_tenant_owner_or_role` for tenant data) with a
  trusted `UserContext` built after authentication. Ownership never comes
  from a route parameter or submitted field. These types live in the
  `rullst-security` crate (`rullst::security_runtime` with the `security`
  feature).
- Use parameterised queries only (the ORM builder, or `sqlx::query(..)` with
  `.bind(..)`); never format user input into SQL.
- Production code must not use `unwrap()`, `expect()` or `panic!()`; return
  typed errors or a suitable HTTP status. Tests may use `unwrap()`.
- Hash passwords with `rullst_auth::hash_password_async` (Argon2). Never
  store or log plaintext secrets.
- Verify webhook signatures with constant-time comparison before trusting
  payment events.
- External providers (payments, mail, OAuth, AI) fall back to deterministic
  offline mocks when credentials are empty or start with `mock_`.

## Authentication

`cargo rullst auth` scaffolds registration, login, the `User` model, its
migration, middleware and views (SQLx projects). `cargo rullst make:mfa` adds
a server-side TOTP second factor, `cargo rullst make:jwt` JWT middleware and
`cargo rullst make:cors` CORS middleware. Authentication is opt-in through
the `auth` feature of the `rullst` dependency.

## Testing

`rullst::testing::TestApp` drives a router without a network listener:

```rust
#[tokio::test]
async fn home_renders() {
    let app = rullst::testing::TestApp::new(
        rullst::routes![get("/" => crate::controllers::home::index)].into_axum(),
    );
    app.get("/").send().await.assert_status(200).assert_see("Welcome");
}
```

Request builders support `.form(&data)`, `.json(&data)` and `.body(..)`;
responses offer `assert_status`, `assert_see`, `assert_dont_see`,
`assert_header`, `json()` and `body_string()`. Run `cargo check` after edits
and `cargo test` before finishing. For protected routes, add a negative test
in which one user requests another user's resource and is denied.

## AI features in applications

With the `ai` feature, `rullst::ai::AiClient::auto()` selects a provider from
`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GEMINI_API_KEY`, `DEEPSEEK_API_KEY` or
`OLLAMA_HOST`, and falls back to an offline mock. Every request passes
mandatory prompt-injection and PII guardrails. `cargo rullst make:chat-session`
scaffolds conversation models.

## CLI commands

The assistant may propose (and the user approves) these commands:

| Command | Purpose |
| --- | --- |
| `make:controller <name> [--api]` | Controller with CRUD handlers in `src/controllers/` |
| `make:model <Name> [-m/--migration]` | Model in `src/models/`, optional create-table migration |
| `make:resource <Name> [--api]` | Model, migration, controller and views together |
| `make:migration <name>` | Empty migration in `src/migrations/` |
| `make:migration:auto` | Migration from model/schema differences (SQLite) |
| `make:middleware <name>` | Middleware in `src/middlewares/` |
| `make:worker <name>` | Background worker in `src/workers/` |
| `make:island <name>` | Wasm island in `src/islands/` |
| `make:live <Name>` | Server-driven live component in `src/live/` |
| `make:mail <Name> [--welcome/--reset/--otp/--invoice]` | Typed email template in `src/mail/` |
| `make:mail-invoice`, `make:mail-dunning` | Receipt and payment-recovery mailables |
| `make:billing [--model User]` | SaaS billing (migrations, webhooks, checkout) |
| `make:chat-session` | Chat session/message models for AI memory |
| `make:jwt`, `make:cors`, `make:mfa` | Security middleware and second factor |
| `make:scalar` | Interactive API docs at `/docs` |
| `make:grpc <Service>`, `make:iot <Device>`, `make:k8s`, `make:omni` | Integrations and packaging |
| `make:privacy`, `make:age-gate` | Optional privacy and age-assurance previews |
| `generate:openapi`, `generate:ts` | OpenAPI document and TypeScript client from routes |
| `generate:api --schema <file> --output <dir>` | Rust/TypeScript contracts from OpenAPI |
| `generate:diagram` | Mermaid ER diagram of the models |
| `generate:ai-context [--check]` | Refresh `.llms.txt` and `.rullst/context-map.json` |
| `db:status` | Migration status (read-only) |
| `doctor` | Toolchain and project diagnostics |
| `audit [--idor] [--sbom] [--compliance]` | Security checks |
| `inspect [routes|models|schema|<file>]` | Inspect macro output and structure |

Commands the user runs personally (the assistant only suggests them): `new`,
`dev` (development server with reload), `dash`, `db:migrate`, `db:rollback`,
`db:seed`, `auth`, `studio`, `build`, `deploy`, `upgrade` and `update`.

## Working style

- Read the project inventory before suggesting file paths; follow the
  existing module layout and naming.
- Prefer small, reviewable changes: one scaffold or edit per action.
- After model changes, remind the user to run `cargo rullst db:migrate`.
- After adding modules, make sure `mod.rs` files and `src/main.rs` declare
  them; scaffolds do this automatically.
- When unsure whether an API exists, say so and suggest checking the Rullst
  book (the framework documentation) instead of inventing it.
