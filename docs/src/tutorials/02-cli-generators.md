# Tutorial 02: CLI Automation & Generators ⚡

**Your goal:** generate a small piece of the application, find the resulting
files and understand what still needs your code. You need the CLI installed in
[Tutorial 01](01-hello-world.md#6-continue-with-the-cli) (`cargo rullst --version`
prints `rullst 13.0.0-alpha.4`).

Rullst provides opinionated code generators (`make:*`) for controllers, models,
migrations, and resources. The current generators register generated Rust
modules when possible; they do not add application routes. `make:controller`
and `make:model` also refresh the generated project map (`.llms.txt`,
`.rullst/context-map.json`) and `diagram.md`; hand-written versions of those
files are kept. Review every generated file and mount the intended routes
yourself.

---

## Create the project

Create a Blank starter with SQLite and enter it:

<!-- first-hour: create -->
```bash
cargo rullst new shop --default --blueprint blank --database sqlite \
  --skip-initial-migration
cd shop
```

`--default` skips the wizard's questions. The project has `Cargo.toml`,
`Rullst.toml`, `.env`, `src/main.rs` (the routes, in `router()`) and a first
`users` migration. Run every command below from this directory; the following
tutorials use the same project unless they say otherwise. To see the welcome
page, run `cargo rullst dev` and open `http://127.0.0.1:3000`; the first build
can take several minutes.

---

## 🛠️ Essential Scaffolding Commands

Run the three commands in order: each one adds different files.

### 1. Generate a Controller
<!-- first-hour: controller -->
```bash
cargo rullst make:controller ProductsController
```
Creates `src/controllers/products_controller.rs` with placeholder `index`,
`show`, `store`, `update`, and `delete` handlers and registers the module
(`pub mod controllers;` in `src/main.rs`). Mount its page by adding a line to
the `routes![...]` list in `router()`:

<!-- first-hour: mount insert=src/main.rs after='get("/" => home),' -->
```rust,ignore
get("/products" => controllers::products_controller::index),
```

After `cargo rullst dev` rebuilds, `http://127.0.0.1:3000/products` shows the
generated placeholder page.

### 2. Generate a Model & Migration
<!-- first-hour: model -->
```bash
cargo rullst make:model Product --migration
```
Creates the model in `src/models/product.rs` and a timestamped Rust migration
(`src/migrations/m<timestamp>_create_products.rs`) that `src/migrations/mod.rs`
registers. The model starts with an `id` and a `snapshot` field, the
migration with `id` and timestamps; [Tutorial 03](03-active-record-crud.md)
adds the columns. `cargo rullst dev` applies pending migrations when it starts;
`cargo rullst db:migrate` applies them without starting the server
([Tutorial 05](05-migrations-and-seeds.md)).

### 3. Generate a Full-Stack Resource Starting Point
<!-- first-hour: resource -->
```bash
cargo rullst make:resource Category
```
Scaffolds a model (`src/models/category.rs`), its migration, a controller
(`src/controllers/category_controller.rs`) and HTML view placeholders
(`views/category/index.html` and `views/category/form.html`) in one command.
Use a name that has no model yet: for an existing model or table the command
skips the create-table migration. The generated controller is not a complete
authorized CRUD implementation.

---

## 🔍 Static CLI Code Inspection

Inspect recognizable source declarations without launching a server:

<!-- first-hour: inspect -->
```bash
# Inspect recognizable routes! declarations
cargo rullst inspect routes

# Print ORM models and field types
cargo rullst inspect models
```

Add `--json` for a versioned machine-readable document (see the
[CLI reference](../cli_reference.md)).

---

## 💡 Key Takeaways

- Scaffolding generators maintain architectural consistency across team members.
- `cargo rullst inspect` performs bounded source-text inspection. Route output
  recognizes explicit single-line `routes!` entries (`get`, `post`, `put` and
  `delete`); model output lists public declarations from `src/models`. It is
  not complete macro expansion or a Rust semantic analysis.
- Generators never add routes: mount each handler in `router()` yourself.
