# Tutorial 02: CLI Automation & Generators ⚡

Rullst provides opinionated code generators (`make:*`) for controllers, models,
migrations, and resources. The current generators register generated Rust
modules when possible; they do not silently add application routes or rewrite
AI context. Review every generated file and mount the intended routes yourself.

---

## 🛠️ Essential Scaffolding Commands

### 1. Generate a Controller
```bash
cargo rullst make:controller ProductsController
```
Creates `src/controllers/products_controller.rs` with placeholder `index`,
`show`, `store`, `update`, and `delete` handlers and registers the module.

### 2. Generate a Model & Migration
```bash
cargo rullst make:model Product --migration
```
Creates the Model struct in `src/models/product.rs` and a timestamped migration in `migrations/`.

### 3. Generate a Full-Stack Resource Starting Point
```bash
cargo rullst make:resource Product
```
Scaffolds a model, Rust migration, controller, and HTML view placeholders
(`views/product/index.html` and `views/product/form.html`) in one command. The
generated controller is not a complete authorized CRUD implementation.

---

## 🔍 Static CLI Code Inspection

Inspect active routes and models without launching a server:

```bash
# Print active route table
cargo rullst inspect route

# Print ORM models and field types
cargo rullst inspect model
```

---

## 💡 Key Takeaways
- Scaffolding generators maintain architectural consistency across team members.
- `cargo rullst inspect` performs bounded source-text inspection. Route output
  recognizes explicit single-line `routes!` entries; model output lists public
  declarations from `src/models`. It is not complete macro expansion or a Rust
  semantic analysis.
