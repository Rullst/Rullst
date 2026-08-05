# Tutorial 28: gRPC Microservices with Tonic 🌐

Build high-performance gRPC microservices and Protobuf schemas using `cargo rullst make:grpc`.

---

## 🛠️ Step 1: Generate a gRPC Service

```bash
cargo rullst make:grpc UserService
```

This generates:
- `proto/user_service.proto` (Protobuf definition)
- `src/grpc/user_service.rs` (Tonic service implementation)

---

## 💻 Step 2: Implement the gRPC Handler

In `src/grpc/user_service.rs`:

```rust
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
- gRPC delivers sub-microsecond inter-service communication for microservice mesh architectures.
- Powered by `tonic` and static `prost` codegen.
