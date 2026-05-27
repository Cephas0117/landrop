use std::path::PathBuf;
use std::sync::Arc;

use dashmap::DashMap;
use parking_lot::RwLock;
use uuid::Uuid;

use crate::peer::Peer;
use crate::transfer::Transfer;

pub struct AppState {
    pub device_id: Uuid,
    pub device_name: String,
    pub peers: Arc<DashMap<Uuid, Peer>>,
    pub transfers: Arc<DashMap<Uuid, Transfer>>,
    pub history: Arc<RwLock<Vec<Transfer>>>,
    pub receive_dir: Arc<RwLock<PathBuf>>,
}

impl AppState {
    pub fn new(device_id: Uuid, device_name: String, receive_dir: PathBuf) -> Self {
        Self {
            device_id,
            device_name,
            peers: Arc::new(DashMap::new()),
            transfers: Arc::new(DashMap::new()),
            history: Arc::new(RwLock::new(Vec::new())),
            receive_dir: Arc::new(RwLock::new(receive_dir)),
        }
    }

    pub fn upsert_peer(&self, peer: Peer) {
        self.peers.insert(peer.id, peer);
    }

    pub fn remove_peer(&self, id: Uuid) {
        self.peers.remove(&id);
    }

    pub fn get_peers(&self) -> Vec<Peer> {
        self.peers.iter().map(|e| e.value().clone()).collect()
    }

    pub fn add_transfer(&self, t: Transfer) -> Uuid {
        let id = t.id;
        self.transfers.insert(id, t);
        id
    }

    pub fn update_transfer<F>(&self, id: Uuid, f: F) where F: FnOnce(&mut Transfer) {
        if let Some(mut t) = self.transfers.get_mut(&id) {
            f(&mut t);
        }
    }

    pub fn complete_transfer(&self, id: Uuid) {
        if let Some((_, t)) = self.transfers.remove(&id) {
            self.history.write().push(t);
        }
    }

    pub fn get_transfers(&self) -> Vec<Transfer> {
        self.transfers.iter().map(|e| e.value().clone()).collect()
    }

    pub fn get_history(&self) -> Vec<Transfer> {
        self.history.read().clone()
    }

    pub fn receive_dir(&self) -> PathBuf {
        self.receive_dir.read().clone()
    }

    pub fn set_receive_dir(&self, path: PathBuf) {
        *self.receive_dir.write() = path;
    }
}
