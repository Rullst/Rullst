use loco_rs::app::Hooks;
use loco_rs::controller::{AppRoutes, Routes};
use loco_rs::prelude::*;
use axum::{routing::get, Json};
use serde::Serialize;
use sea_orm_migration::prelude::*;
use loco_rs::boot::{StartMode, BootResult};
use loco_rs::environment::Environment;

#[derive(Serialize)]
struct Message {
    message: &'static str,
}

async fn plaintext() -> &'static str {
    "Hello, World!"
}

async fn json_endpoint() -> Json<Message> {
    Json(Message { message: "Hello, World!" })
}

struct App;

struct Migrator;

#[async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![]
    }
}

#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        "bench-loco"
    }

    fn routes(_ctx: &AppContext) -> AppRoutes {
        let routes = Routes::new()
            .add("/", get(plaintext))
            .add("/json", get(json_endpoint));
        AppRoutes::with_default_routes()
            .add_route(routes)
    }

    async fn boot(
        mode: StartMode,
        environment: &Environment,
    ) -> Result<BootResult> {
        loco_rs::boot::create_app::<Self, Migrator>(mode, environment).await
    }

    async fn connect_workers(_ctx: &AppContext, _queue: &loco_rs::prelude::Queue) -> Result<()> {
        Ok(())
    }

    fn register_tasks(_tasks: &mut loco_rs::task::Tasks) {}

    async fn truncate(_db: &DatabaseConnection) -> Result<()> {
        Ok(())
    }

    async fn seed(_db: &DatabaseConnection, _path: &std::path::Path) -> Result<()> {
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    loco_rs::cli::main::<App, Migrator>().await;
}
