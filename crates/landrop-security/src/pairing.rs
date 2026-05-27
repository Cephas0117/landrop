use std::time::Instant;

use dashmap::DashMap;
use rand::Rng;
use uuid::Uuid;

pub struct PairState {
    pub pin: String,
    pub peer_fingerprint: String,
    pub timestamp: Instant,
}

pub struct PairingManager {
    active_requests: DashMap<Uuid, PairState>,
}

impl PairingManager {
    pub fn new() -> Self {
        Self { active_requests: DashMap::new() }
    }

    pub fn generate_pin() -> String {
        let n: u32 = rand::thread_rng().gen_range(0..1_000_000);
        format!("{:06}", n)
    }

    pub fn create_request(&self, peer_id: Uuid, peer_fingerprint: String) -> String {
        let pin = Self::generate_pin();
        self.active_requests.insert(peer_id, PairState {
            pin: pin.clone(),
            peer_fingerprint,
            timestamp: Instant::now(),
        });
        pin
    }

    pub fn verify_pin(&self, peer_id: Uuid, pin: &str) -> bool {
        self.active_requests
            .get(&peer_id)
            .map(|s| s.pin == pin && s.timestamp.elapsed().as_secs() < 120)
            .unwrap_or(false)
    }

    pub fn complete_pairing(&self, peer_id: Uuid) -> Option<PairState> {
        self.active_requests.remove(&peer_id).map(|(_, v)| v)
    }

    pub fn cancel_request(&self, peer_id: Uuid) {
        self.active_requests.remove(&peer_id);
    }
}

impl Default for PairingManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_format() {
        for _ in 0..100 {
            let pin = PairingManager::generate_pin();
            assert_eq!(pin.len(), 6);
            assert!(pin.chars().all(|c| c.is_ascii_digit()));
        }
    }

    #[test]
    fn verify_correct_pin() {
        let mgr = PairingManager::new();
        let peer_id = Uuid::new_v4();
        let pin = mgr.create_request(peer_id, "fp".into());
        assert!(mgr.verify_pin(peer_id, &pin));
    }

    #[test]
    fn verify_wrong_pin() {
        let mgr = PairingManager::new();
        let peer_id = Uuid::new_v4();
        mgr.create_request(peer_id, "fp".into());
        assert!(!mgr.verify_pin(peer_id, "000000"));
    }
}
