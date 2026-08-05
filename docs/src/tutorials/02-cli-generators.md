# Tutorial 02: CLI Automation & Generators ⚡

Rullst provides opinionated code generators (`make:*`) that scaffold controllers, models, migrations, and resources while automatically updating route files and AI context.

---

## 🛠️ Essential Scaffolding Commands

### 1. Generate a Controller
```bash
cargo rullst make:controller ProductsController
```
Creates `src/controllers/products_controller.rs` with standard CRUD handlers (`index`, `show`, `create`, `update`, `delete`).

### 2. Generate a Model & Migration
```bash
cargo rullst make:model Product --migration
```
Creates the Model struct in `src/models/product.rs` and a timestamped migration in `migrations/`.

### 3. Generate a Complete Full-Stack Resource
```bash
cargo rullst make:resource Product
```
Scaffolds Model, Migration, Controller, and HTML views (`views/product/index.html` & `views/product/form.html`) in a single command.

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
- `cargo rullst inspect` statically analyzes proc-macros and routing AST without runtime overhead.
