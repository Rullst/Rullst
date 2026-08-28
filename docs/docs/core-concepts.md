# Core Concepts

Rullst is designed around standard MVC concepts, but supercharged with Rust's macros and typing.

## Controllers

Controllers handle your HTTP requests. You can generate one via CLI:

```bash
cargo rullst make:controller users
```

This creates a RESTful controller inside `src/controllers/users_controller.rs`.

## Active Record ORM

Rullst includes `rullst-orm`, an Active Record pattern ORM.

```rust
#[derive(Orm)]
#[orm(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
}

// In your controller:
let user = User::find(1).await?;
```

## HTMX Live State

Rullst renders pages instantly. By using the `html!` macro, any changes in your code are parsed and sent to the browser via WebSockets during development, updating the DOM seamlessly without a full reload!
