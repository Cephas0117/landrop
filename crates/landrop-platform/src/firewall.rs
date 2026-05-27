use anyhow::Result;

#[derive(Debug, Clone, PartialEq)]
pub enum FirewallStatus {
    Ok,
    BlockingInbound,
    Unknown,
}

pub async fn check_firewall(port: u16) -> Result<FirewallStatus> {
    #[cfg(target_os = "macos")]
    {
        return check_macos(port).await;
    }
    #[cfg(target_os = "windows")]
    {
        return check_windows(port).await;
    }
    #[allow(unreachable_code)]
    Ok(FirewallStatus::Unknown)
}

/// Ensure inbound rules exist for LANDrop ports.
/// Safe to call repeatedly — netsh silently skips duplicate rules.
pub fn ensure_windows_firewall_rules() {
    #[cfg(target_os = "windows")]
    {
        let rules = [
            (
                "LANDrop Discovery (UDP)",
                "protocol=UDP",
                "7777",
            ),
            (
                "LANDrop Transfer (TCP)",
                "protocol=TCP",
                "7878",
            ),
        ];
        for (name, proto, port) in rules {
            let _ = std::process::Command::new("netsh")
                .args([
                    "advfirewall", "firewall", "add", "rule",
                    &format!("name={name}"),
                    "dir=in",
                    "action=allow",
                    proto,
                    &format!("localport={port}"),
                    "enable=yes",
                    "profile=any",
                ])
                .output();
        }
    }
}

#[cfg(target_os = "macos")]
async fn check_macos(_port: u16) -> Result<FirewallStatus> {
    use tokio::net::TcpListener;
    match TcpListener::bind(format!("127.0.0.1:{}", _port)).await {
        Ok(_) => Ok(FirewallStatus::Ok),
        Err(_) => Ok(FirewallStatus::BlockingInbound),
    }
}

#[cfg(target_os = "windows")]
async fn check_windows(port: u16) -> Result<FirewallStatus> {
    // Try to bind on the port; if it fails the port may be blocked or in use.
    use tokio::net::UdpSocket;
    match UdpSocket::bind(format!("0.0.0.0:{}", port)).await {
        Ok(_) => Ok(FirewallStatus::Ok),
        Err(_) => Ok(FirewallStatus::BlockingInbound),
    }
}
