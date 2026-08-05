# Tutorial 07: Forms & DTO Validation 📝

Learn how to accept, parse, and validate incoming form submissions and JSON DTOs in Rullst handlers using `validator`.

---

## 🛠️ Step 1: Define a Validated DTO Struct

```rust
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserForm {
    #[validate(length(min = 3, message = "Name must be at least 3 characters"))]
    pub name: String,

    #[validate(email(message = "Invalid email address"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
}
```

---

## 💻 Step 2: Handle Form Submissions in a Controller

```rust
use axum::Form;
use validator::Validate;
use rullst_core::{AppError, html, Response};
use crate::models::User;

pub async fn store(Form(form): Form<CreateUserForm>) -> Result<Response, AppError> {
    // Validate the incoming DTO
    if let Err(errors) = form.validate() {
        return Err(AppError::BadRequest(format!("Validation failed: {:?}", errors)));
    }

    // Persist user to database
    let user = User::create(serde_json::json!({
        "name": form.name,
        "email": form.email,
        "password": form.password
    })).await?;

    Ok(html! {
        <div class="p-4 bg-emerald-900/50 text-emerald-300 rounded border border-emerald-500">
            <p>"User created successfully! ID: " { user.id.to_string() }</p>
        </div>
    })
}
```

---

## 💡 Key Takeaways
- Use `axum::Form` for traditional HTML forms and `axum::Json` for JSON payloads.
- Always validate incoming DTOs before executing database queries or business operations.
