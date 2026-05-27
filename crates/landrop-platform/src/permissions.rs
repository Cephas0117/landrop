use anyhow::Result;

pub struct PlatformInfo {
    pub os: String,
    pub hostname: String,
}

pub fn platform_info() -> Result<PlatformInfo> {
    let os = std::env::consts::OS.to_string();
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    Ok(PlatformInfo { os, hostname })
}
