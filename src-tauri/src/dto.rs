use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerDto {
    pub id: String,
    pub name: String,
    pub os: String,
    pub addr: String,
    pub fingerprint: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferDto {
    pub id: String,
    pub peer_id: String,
    pub peer_name: String,
    pub direction: String,
    pub status: String,
    pub bytes_sent: u64,
    pub total_bytes: u64,
    pub speed_bps: f64,
    pub eta_secs: f64,
    pub files_done: u32,
    pub files_total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfoDto {
    pub device_id: String,
    pub device_name: String,
    pub fingerprint: String,
    pub receive_dir: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingRequestDto {
    pub peer_id: String,
    pub peer_name: String,
    pub peer_fingerprint: String,
    pub pin: String,
}
