# Tutorial 28: gRPC Microservices with Tonic 🌐

Generate a Tonic/Protobuf starting point using `cargo rullst make:grpc`. The
command does not start a gRPC server, add every build dependency, or define
production transport/authentication policy for the application.

---

## 🛠️ Step 1: Generate a gRPC Service

```bash
cargo rullst make:grpc UserService
```

This generates:
- `proto/user_service.proto` (Protobuf definition)
- `src/grpc/user_service.rs` (Tonic service implementation)

Names ending in `Service` remain a single service suffix: `UserService`
produces the `user_service_server::UserService` trait rather than
`UserServiceService`.

---

## 💻 Step 2: Implement the gRPC Handler

In `src/grpc/user_service.rs`, after the generated application's `build.rs` has
compiled `proto/user_service.proto` and made the Tonic dependencies available:

```rust,ignore
use tonic::{Request, Response, Status};

pub mod proto {
    tonic::include_proto!("user_service");
}

use proto::user_service_server::UserService;
use proto::{HelloRequest, HelloResponse};

#[derive(Debug, Default)]
pub struct UserServiceImpl;

#[tonic::async_trait]
impl UserService for UserServiceImpl {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloResponse>, Status> {
        let reply = HelloResponse {
            message: format!("Hello {} from Rullst gRPC!", request.into_inner().name),
        };
        Ok(Response::new(reply))
    }
}
```

---

## 💡 Key Takeaways
- The generator creates a starting point for `tonic` and `prost`; inspect the
  generated service and add/review the required `build.rs`, dependencies,
  reflection/health endpoints, and server bootstrap before deployment.
- Apply TLS or mTLS, authentication, per-method authorization, deadlines,
  message-size/concurrency limits, and proxy policy explicitly.
- Network, serialization, handler, and proxy latency must be measured in the
  target environment. Rullst does not claim a universal latency bound.
