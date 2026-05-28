use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use directories::UserDirs;
use landrop_core::ServiceContainer;
use landrop_discovery::DiscoveryEvent;
use landrop_transfer::{PairingEvent, TransferEvent};
use tokio::sync::mpsc;

pub struct TauriState {
    pub services: Arc<ServiceContainer>,
}

pub struct AppInit {
    pub state: TauriState,
    pub discovery_rx: mpsc::UnboundedReceiver<DiscoveryEvent>,
    pub transfer_rx: mpsc::UnboundedReceiver<TransferEvent>,
    pub pairing_rx: mpsc::UnboundedReceiver<PairingEvent>,
}

pub async fn init_app_state() -> Result<AppInit> {
    let receive_dir = UserDirs::new()
        .and_then(|d| d.download_dir().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "LANDrop".to_string());

    let services = ServiceContainer::init(hostname, receive_dir).await?;

    let discovery_rx = services.take_discovery_rx().expect("discovery_rx taken twice");
    let transfer_rx = services.take_transfer_rx().expect("transfer_rx taken twice");
    let pairing_rx = services.take_pairing_rx().expect("pairing_rx taken twice");

    Ok(AppInit {
        state: TauriState { services },
        discovery_rx,
        transfer_rx,
        pairing_rx,
    })
}
