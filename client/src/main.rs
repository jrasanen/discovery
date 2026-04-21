use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use anyhow::{Result, anyhow};
use mdns_sd::{ServiceDaemon, ServiceEvent};
use tokio::time::timeout;

const SERVICE_TYPE: &str = "_hello._tcp.local.";
const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(10);

#[tokio::main]
async fn main() -> Result<()> {
    let daemon = ServiceDaemon::new()?;
    let receiver = daemon.browse(SERVICE_TYPE)?;

    println!(
        "Browsing for {SERVICE_TYPE} (timeout {:?})...",
        DISCOVERY_TIMEOUT
    );

    let resolved = timeout(DISCOVERY_TIMEOUT, async {
        loop {
            match receiver.recv_async().await {
                Ok(ServiceEvent::ServiceResolved(info)) => {
                    let addrs = info.get_addresses();
                    let addr = addrs
                        .iter()
                        .find(|a| a.is_ipv4())
                        .or_else(|| addrs.iter().next())
                        .map(|a| a.to_ip_addr());
                    if let Some(addr) = addr {
                        return Ok::<_, anyhow::Error>((
                            addr,
                            info.get_port(),
                            info.get_fullname().to_string(),
                        ));
                    }
                }
                Ok(_) => {}
                Err(e) => return Err(anyhow!("browse channel error: {e}")),
            }
        }
    })
    .await
    .map_err(|_| anyhow!("timed out waiting for a {SERVICE_TYPE} service"))??;

    let (addr, port, fullname): (IpAddr, u16, String) = resolved;
    let socket = SocketAddr::new(addr, port);
    let url = format!("http://{socket}/hello");

    println!("Resolved {fullname} at {socket}");
    println!("GET {url}");

    let body = reqwest::get(&url).await?.text().await?;
    println!("<- {body}");

    let _ = daemon.shutdown();
    Ok(())
}
