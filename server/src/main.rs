use std::collections::HashMap;

use anyhow::Result;
use axum::{Router, routing::get};
use chrono::Utc;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use tokio::net::TcpListener;

const SERVICE_TYPE: &str = "_hello._tcp.local.";

async fn hello() -> String {
    Utc::now().to_rfc3339()
}

#[tokio::main]
async fn main() -> Result<()> {
    let app = Router::new().route("/hello", get(hello));

    let listener = TcpListener::bind("0:0").await?;
    let port = listener.local_addr()?.port();

    let daemon = ServiceDaemon::new()?;
    let hostname = hostname::get()?.to_string_lossy().replace('.', "-");
    let host_name = format!("{hostname}.local.");
    let instance_name = format!("hello-server-{hostname}");

    let properties: HashMap<String, String> = HashMap::new();
    let service_info = ServiceInfo::new(
        SERVICE_TYPE,
        &instance_name,
        &host_name,
        "",
        port,
        properties,
    )?
    .enable_addr_auto();

    let fullname = service_info.get_fullname().to_string();
    daemon.register(service_info)?;

    println!("Server listening on port {port}");
    println!("Registered mDNS service: {fullname}");
    println!("Press Ctrl+C to exit.");

    let shutdown_daemon = daemon.clone();
    let shutdown_fullname = fullname.clone();
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = tokio::signal::ctrl_c().await;
            println!("\nUnregistering mDNS service and shutting down...");
            if let Ok(rx) = shutdown_daemon.unregister(&shutdown_fullname) {
                let _ = rx.recv();
            }
            let _ = shutdown_daemon.shutdown();
        })
        .await?;

    Ok(())
}
