use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use dashmap::DashMap;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::broadcast::BroadcastDiscovery;
#[allow(unused_imports)]
use crate::mdns::MdnsDiscovery;

const PEER_TTL_MS: u64 = 8_000;

#[derive(Debug, Clone)]
pub struct DiscoveredPeer {
    pub device_id: Uuid,
    pub device_name: String,
    pub addr: SocketAddr,
    pub last_seen_ms: u64,
}

#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    PeerAdded(DiscoveredPeer),
    PeerUpdated(DiscoveredPeer),
    PeerExpired(Uuid),
}

pub struct DiscoveryManager {
    device_id: Uuid,
    device_name: String,
    peers: Arc<DashMap<Uuid, DiscoveredPeer>>,
    event_tx: mpsc::UnboundedSender<DiscoveryEvent>,
}

impl DiscoveryManager {
    pub fn new(
        device_id: Uuid,
        device_name: String,
    ) -> (Self, mpsc::UnboundedReceiver<DiscoveryEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (
            Self {
                device_id,
                device_name,
                peers: Arc::new(DashMap::new()),
                event_tx: tx,
            },
            rx,
        )
    }

    pub async fn start(&self, tcp_port: u16) -> Result<()> {
        let (_, mut broadcast_rx) =
            BroadcastDiscovery::new(self.device_id, self.device_name.clone(), tcp_port).await?;

        let peers_bcast = self.peers.clone();
        let tx_bcast = self.event_tx.clone();

        tokio::spawn(async move {
            while let Some(p) = broadcast_rx.recv().await {
                let addr = SocketAddr::new(p.addr, p.tcp_port);
                let peer = DiscoveredPeer {
                    device_id: p.device_id,
                    device_name: p.device_name,
                    addr,
                    last_seen_ms: now_ms(),
                };
                let evt = if peers_bcast.contains_key(&p.device_id) {
                    DiscoveryEvent::PeerUpdated(peer.clone())
                } else {
                    DiscoveryEvent::PeerAdded(peer.clone())
                };
                peers_bcast.insert(p.device_id, peer);
                let _ = tx_bcast.send(evt);
            }
        });

        // TTL eviction task
        let peers_evict = self.peers.clone();
        let tx_evict = self.event_tx.clone();

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_millis(1_000));
            loop {
                ticker.tick().await;
                let expired: Vec<Uuid> = peers_evict
                    .iter()
                    .filter(|e| now_ms() - e.last_seen_ms > PEER_TTL_MS)
                    .map(|e| *e.key())
                    .collect();
                for id in expired {
                    peers_evict.remove(&id);
                    let _ = tx_evict.send(DiscoveryEvent::PeerExpired(id));
                }
            }
        });

        Ok(())
    }

    pub fn get_peers(&self) -> Vec<DiscoveredPeer> {
        self.peers.iter().map(|e| e.value().clone()).collect()
    }

    pub async fn probe_manual(&self, addr: SocketAddr) -> Result<DiscoveredPeer> {
        // TCP connect probe - just verify the address is reachable
        let stream = tokio::time::timeout(
            Duration::from_secs(3),
            tokio::net::TcpStream::connect(addr),
        )
        .await??;
        drop(stream);

        // Return a placeholder; real HELLO exchange happens in connection setup
        let peer = DiscoveredPeer {
            device_id: Uuid::new_v4(),
            device_name: addr.to_string(),
            addr,
            last_seen_ms: now_ms(),
        };
        Ok(peer)
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
