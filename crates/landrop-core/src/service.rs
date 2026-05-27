use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use landrop_discovery::{DiscoveryEvent, DiscoveryManager};
use landrop_security::{DeviceIdentity, PairingManager, TrustStore};
use landrop_state::app::AppState;
use landrop_transfer::{TransferEngine, TransferEvent};
use parking_lot::{Mutex, RwLock};
use tokio::sync::mpsc;

pub struct ServiceContainer {
    pub app_state: Arc<AppState>,
    pub identity: Arc<DeviceIdentity>,
    pub trust_store: Arc<RwLock<TrustStore>>,
    pub discovery: Arc<DiscoveryManager>,
    pub transfer: Arc<TransferEngine>,
    pub pairing: Arc<PairingManager>,
    // Taken once by the event bridge on startup
    discovery_rx: Mutex<Option<mpsc::UnboundedReceiver<DiscoveryEvent>>>,
    transfer_rx: Mutex<Option<mpsc::UnboundedReceiver<TransferEvent>>>,
}

impl ServiceContainer {
    pub async fn init(device_name: String, receive_dir: PathBuf) -> Result<Arc<Self>> {
        let identity = Arc::new(DeviceIdentity::generate()?);
        let trust_store = Arc::new(RwLock::new(TrustStore::load().unwrap_or_default()));
        let pairing = Arc::new(PairingManager::new());

        let app_state = Arc::new(AppState::new(
            identity.device_id,
            device_name.clone(),
            receive_dir.clone(),
        ));

        let (discovery, discovery_rx) =
            DiscoveryManager::new(identity.device_id, device_name);

        let (transfer, transfer_rx) = TransferEngine::new(
            identity.clone(),
            trust_store.clone(),
            receive_dir,
        );

        Ok(Arc::new(Self {
            app_state,
            identity,
            trust_store,
            pairing,
            discovery: Arc::new(discovery),
            transfer: Arc::new(transfer),
            discovery_rx: Mutex::new(Some(discovery_rx)),
            transfer_rx: Mutex::new(Some(transfer_rx)),
        }))
    }

    pub fn take_discovery_rx(&self) -> Option<mpsc::UnboundedReceiver<DiscoveryEvent>> {
        self.discovery_rx.lock().take()
    }

    pub fn take_transfer_rx(&self) -> Option<mpsc::UnboundedReceiver<TransferEvent>> {
        self.transfer_rx.lock().take()
    }
}
