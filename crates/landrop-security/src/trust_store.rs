use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrustStore {
    peers: HashMap<Uuid, String>, // device_id -> fingerprint
    #[serde(skip)]
    path: PathBuf,
}

impl TrustStore {
    pub fn load() -> Result<Self> {
        let path = store_path()?;
        if !path.exists() {
            return Ok(Self { path, ..Default::default() });
        }
        let raw = std::fs::read(&path)
            .with_context(|| format!("failed to read trust store: {}", path.display()))?;
        let mut store: TrustStore = serde_json::from_slice(&raw)
            .with_context(|| format!("failed to parse trust store: {}", path.display()))?;
        store.path = path;
        Ok(store)
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_vec_pretty(self)?;
        std::fs::write(&self.path, data)
            .with_context(|| format!("failed to write trust store: {}", self.path.display()))?;
        Ok(())
    }

    pub fn is_trusted(&self, id: Uuid, fingerprint: &str) -> bool {
        self.peers.get(&id).map(|fp| fp == fingerprint).unwrap_or(false)
    }

    pub fn add_peer(&mut self, id: Uuid, fingerprint: String) {
        self.peers.insert(id, fingerprint);
    }

    pub fn remove_peer(&mut self, id: Uuid) {
        self.peers.remove(&id);
    }

    pub fn contains(&self, id: Uuid) -> bool {
        self.peers.contains_key(&id)
    }
}

fn store_path() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("com", "landrop", "LANDrop")
        .context("cannot determine data directory")?;
    Ok(dirs.data_dir().join("trust_store.json"))
}
