use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PeerOs {
    Windows,
    MacOs,
    Linux,
    Unknown,
}

impl From<&str> for PeerOs {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "windows" => PeerOs::Windows,
            "macos" | "darwin" => PeerOs::MacOs,
            "linux" => PeerOs::Linux,
            _ => PeerOs::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PeerState {
    Discovered,
    Connecting,
    TlsHandshaking,
    HelloExchange,
    PairingRequired,
    WaitingPairDecision,
    Trusted,
    ManifestNegotiation,
    Transferring,
    Completing,
    Completed,
    Failed(String),
    Canceled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    pub id: Uuid,
    pub name: String,
    pub os: PeerOs,
    pub addr: std::net::SocketAddr,
    pub fingerprint: String,
    pub state: PeerState,
    pub last_seen_ms: u64,
}

impl Peer {
    pub fn new(
        id: Uuid,
        name: String,
        os: PeerOs,
        addr: std::net::SocketAddr,
        fingerprint: String,
    ) -> Self {
        Self {
            id,
            name,
            os,
            addr,
            fingerprint,
            state: PeerState::Discovered,
            last_seen_ms: now_ms(),
        }
    }

    pub fn touch(&mut self) {
        self.last_seen_ms = now_ms();
    }

    pub fn is_expired(&self, ttl_ms: u64) -> bool {
        now_ms().saturating_sub(self.last_seen_ms) > ttl_ms
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
