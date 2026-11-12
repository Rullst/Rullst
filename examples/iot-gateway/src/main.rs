//! `rullst-iot-gateway-example <command>`:
//!
//! - `server` (default): the receiver, the dashboard and Nexus.
//! - `register <device-id> <name...>`: adds a device and prints its token
//!   once.
//! - `revoke <device-id>`: refuses the device's token from the next request.
//! - `devices`: lists the registry.
//! - `gateway <serial-port> <device-id> [server-url]`: posts one board's
//!   readings; the token comes from `IOT_DEVICE_TOKEN`.
//! - `db:migrate` and the other Artisan commands.
//!
//! The database is `DATABASE_URL`, else `sqlite://iot-gateway.db?mode=rwc`.

use rullst::db::Orm;
use rullst::realtime::RealtimeSse;
use rullst_iot_gateway_example::gateway::{self, GatewayConfig};
use rullst_iot_gateway_example::registry::SqlDeviceDirectory;
use rullst_iot_gateway_example::server::{self, TELEMETRY_PATH};

const DEFAULT_DATABASE: &str = "sqlite://iot-gateway.db?mode=rwc";

fn database_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE.to_owned())
}

#[rullst::runtime::main]
#[cfg_attr(mutants, mutants::skip)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    rullst::artisan!(server::migrations());
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str).unwrap_or("server") {
        "server" => serve().await,
        "register" => {
            let device_id = args.get(1).ok_or("usage: register <device-id> <name...>")?;
            let name = args
                .get(2..)
                .map(|words| words.join(" "))
                .unwrap_or_default();
            let name = if name.is_empty() {
                device_id.clone()
            } else {
                name
            };
            migrated_registry().await?;
            let (device, token) = SqlDeviceDirectory::new().register(device_id, name).await?;
            println!("registered {} ({})", device.device_id, device.name);
            println!("token (shown once; give it to the gateway as IOT_DEVICE_TOKEN):");
            println!("{}", token.expose());
            Ok(())
        }
        "revoke" => {
            let device_id = args.get(1).ok_or("usage: revoke <device-id>")?;
            migrated_registry().await?;
            if SqlDeviceDirectory::new().revoke(device_id).await? {
                println!("revoked {device_id}: its token is refused from now on");
            } else {
                println!("{device_id} was already revoked");
            }
            Ok(())
        }
        "devices" => {
            migrated_registry().await?;
            for device in SqlDeviceDirectory::new().list().await? {
                let state = if device.is_revoked() {
                    "revoked"
                } else {
                    "active"
                };
                println!(
                    "{}\t{}\t{state}\tlast seen {:?}\tfirmware {:?}",
                    device.device_id, device.name, device.last_seen_at, device.firmware_version
                );
            }
            Ok(())
        }
        "gateway" => {
            let usage = "usage: gateway <serial-port> <device-id> [server-url]";
            let serial_port = args.get(1).ok_or(usage)?.clone();
            let device_id = args.get(2).ok_or(usage)?.clone();
            let base = args.get(3).map_or("http://127.0.0.1:3000", String::as_str);
            let token = std::env::var("IOT_DEVICE_TOKEN")
                .map_err(|_| "set IOT_DEVICE_TOKEN to the token `register` printed")?;
            let baud = std::env::var("IOT_SERIAL_BAUD")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(115_200);
            gateway::run(GatewayConfig {
                serial_port,
                baud,
                device_id,
                endpoint: format!("{}{TELEMETRY_PATH}", base.trim_end_matches('/')),
                token,
            })
            .await
        }
        other => Err(format!("unknown command {other}; see the README").into()),
    }
}

/// Opens the database and applies the registry migration.
async fn migrated_registry() -> Result<(), Box<dyn std::error::Error>> {
    Orm::init(&database_url()).await?;
    rullst::db::schema::migration::run_artisan_with_args(
        &[
            "rullst-iot-gateway-example".to_owned(),
            "migrate".to_owned(),
        ],
        server::migrations(),
        Vec::new(),
    )
    .await?;
    Ok(())
}

async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    let app = server::app(server::nexus()?)?;
    let sse: RealtimeSse = app.sse.clone();
    let shutdown = async move {
        let _ = rullst::async_runtime::tokio::signal::ctrl_c().await;
        sse.shutdown();
    };
    rullst::Server::new(app.router)
        .with_db(database_url())
        .with_migrations(server::migrations())
        .auto_migrate(true)
        .with_machine_endpoints(app.policy)
        .run_with_shutdown(3000, shutdown)
        .await?;
    Ok(())
}
