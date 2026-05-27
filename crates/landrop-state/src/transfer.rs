use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransferStatus {
    Queued,
    Connecting,
    Negotiating,
    Transferring,
    Completed,
    Failed(String),
    Canceled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferProgress {
    pub bytes_sent: u64,
    pub total_bytes: u64,
    pub speed_bps: f64,
    pub eta_secs: f64,
    pub files_done: u32,
    pub files_total: u32,
}

impl TransferProgress {
    pub fn fraction(&self) -> f64 {
        if self.total_bytes == 0 {
            0.0
        } else {
            self.bytes_sent as f64 / self.total_bytes as f64
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transfer {
    pub id: Uuid,
    pub session_id: Option<Uuid>,
    pub peer_id: Uuid,
    pub peer_name: String,
    pub paths: Vec<String>,
    pub direction: TransferDirection,
    pub status: TransferStatus,
    pub progress: TransferProgress,
    pub started_at_ms: u64,
    pub finished_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransferDirection {
    Send,
    Receive,
}

impl Transfer {
    pub fn new_send(peer_id: Uuid, peer_name: String, paths: Vec<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id: None,
            peer_id,
            peer_name,
            paths,
            direction: TransferDirection::Send,
            status: TransferStatus::Queued,
            progress: TransferProgress {
                bytes_sent: 0,
                total_bytes: 0,
                speed_bps: 0.0,
                eta_secs: 0.0,
                files_done: 0,
                files_total: 0,
            },
            started_at_ms: now_ms(),
            finished_at_ms: None,
        }
    }

    pub fn new_receive(peer_id: Uuid, peer_name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id: None,
            peer_id,
            peer_name,
            paths: vec![],
            direction: TransferDirection::Receive,
            status: TransferStatus::Connecting,
            progress: TransferProgress {
                bytes_sent: 0,
                total_bytes: 0,
                speed_bps: 0.0,
                eta_secs: 0.0,
                files_done: 0,
                files_total: 0,
            },
            started_at_ms: now_ms(),
            finished_at_ms: None,
        }
    }

    pub fn finish(&mut self, success: bool, err: Option<String>) {
        self.status = if success {
            TransferStatus::Completed
        } else {
            TransferStatus::Failed(err.unwrap_or_default())
        };
        self.finished_at_ms = Some(now_ms());
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
