# Tutorial 27: Typed Dependency Injection (`rullst::di`) 💉

Organize services and repositories with type-indexed registration and no
runtime reflection. Lookup, `Arc` cloning, and application constructors still
have their normal runtime cost.

---

## 🛠️ Step 1: Register Services in `Container`

```rust
use rullst_core::di::Container;
use std::sync::Arc;

pub struct PaymentGateway {
    pub api_key: String,
}

pub struct UserService {
    pub gateway: Arc<PaymentGateway>,
}

pub fn configure_di() -> Arc<Container> {
    let mut container = Container::new();
    
    let gateway = Arc::new(PaymentGateway {
        api_key: "sk_live_12345".to_string(),
    });
    
    container.register_arc(gateway.clone());
    container.register(UserService { gateway });
    
    Arc::new(container)
}
```

---

## 💻 Step 2: Extract Services in Controllers

Use the `Inject<T>` extractor in route handlers:

```rust
use rullst_core::di::Inject;
use axum::Json;

pub async fn process_payment(
    user_svc: Inject<UserService>,
) -> Result<Json<&'static str>, rullst_core::AppError> {
    println!("Using gateway key: {}", user_svc.gateway.api_key);
    Ok(Json("Payment Processed"))
}
```

---

## 💡 Key Takeaways
- Static dispatch without a DI-container vtable; constructors and application logic still have their normal runtime cost.
- Services are stored as type-safe `Arc<T>` singletons across worker threads.
