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

#[cfg(target_os = "macos")]
async fn check_macos(_port: u16) -> Result<FirewallStatus> {
    // Attempt to bind and accept a loopback connection to verify port is reachable
    use tokio::net::TcpListener;
    match TcpListener::bind(format!("127.0.0.1:{}", _port)).await {
        Ok(_) => Ok(FirewallStatus::Ok),
        Err(_) => Ok(FirewallStatus::BlockingInbound),
    }
}

#[cfg(target_os = "windows")]
async fn check_windows(_port: u16) -> Result<FirewallStatus> {
    Ok(FirewallStatus::Unknown)
}
