# Tutorial 27: Typed Dependency Injection (`rullst::di`) 💉

Rullst DI uses Rust types as keys and avoids runtime reflection metadata. It is
not literally zero-cost: registration uses a map, resolution performs a
type-indexed lookup/downcast, and each injection clones an `Arc`.

---

## Step 1: Register services without embedding secrets in source

```rust,no_run
use std::sync::Arc;
use rullst::di::Container;
use rullst::security_runtime::VaultSecret;

pub struct PaymentGateway {
    api_key: VaultSecret<String>,
}

impl PaymentGateway {
    pub fn is_configured(&self) -> bool {
        !self.api_key.expose_secret().is_empty()
    }
}

pub struct UserService {
    pub gateway: Arc<PaymentGateway>,
}

pub fn configure_di() -> Result<Arc<Container>, std::env::VarError> {
    let key = std::env::var("PAYMENT_API_KEY")?;
    let mut container = Container::new();
    let gateway = Arc::new(PaymentGateway {
        api_key: VaultSecret::new(key),
    });

    container.register_arc(Arc::clone(&gateway));
    container.register(UserService { gateway });
    Ok(Arc::new(container))
}
```

`security_runtime` requires the umbrella `security` feature. A secret manager
should inject the real value in production; `VaultSecret` reduces accidental
formatting and zeroizes its owned allocation on drop, but it is not key custody
or secure memory.

---

## Step 2: Attach the container and extract a service

```rust,ignore
use axum::{Extension, Json, routing::post};
use rullst::{Router, di::Inject};

pub async fn process_payment(
    Inject(user_service): Inject<UserService>,
) -> Json<&'static str> {
    // Use a bounded provider adapter; never log the key or full request payload.
    let _configured = user_service.gateway.is_configured();
    Json("Payment request accepted")
}

let container = configure_di()?;
let app = Router::new()
    .route("/payments", post(process_payment))
    .layer(Extension(container));
```

The continuation is application code because its error type and provider
adapter belong to the project. If the requested type or container extension is
missing, `Inject<T>` fails closed with a `500` rejection.

---

## Key takeaways

- Services are registered as type-safe `Arc<T>` singletons.
- DI controls construction and lookup; it does not provide authorization,
  transaction boundaries, or secret management.
- Prefer ordinary constructors when a small service graph does not benefit from
  a container.
