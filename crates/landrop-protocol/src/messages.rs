use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub protocol_version: u16,
    pub max_chunk_size: u32,
    pub supports_tls13: bool,
    pub supports_resume: bool,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            protocol_version: 1,
            max_chunk_size: 256 * 1024,
            supports_tls13: true,
            supports_resume: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hello {
    pub device_id: Uuid,
    pub session_id: Uuid,
    pub device_name: String,
    pub os: String,
    pub cert_fingerprint: String,
    pub capabilities: Capabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapsAck {
    pub device_id: Uuid,
    pub session_id: Uuid,
    pub device_name: String,
    pub cert_fingerprint: String,
    pub accepted_version: u16,
    pub requires_pairing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairRequest {
    pub device_id: Uuid,
    pub session_id: Uuid,
    pub pin: String,
    pub peer_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairAccept {
    pub device_id: Uuid,
    pub session_id: Uuid,
    pub accepted: bool,
    pub peer_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub file_id: Uuid,
    pub relative_path: String,
    pub size: u64,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub session_id: Uuid,
    pub transfer_name: String,
    pub files: Vec<FileEntry>,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileDecision {
    Send,
    SkipAlreadyPresent,
    RestartFile { resume_offset: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAck {
    pub file_id: Uuid,
    pub decision: FileDecision,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestAck {
    pub session_id: Uuid,
    pub files: Vec<FileAck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub session_id: Uuid,
    pub file_id: Uuid,
    pub seq: u32,
    pub offset: u64,
    pub data: Vec<u8>,
    pub eof: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkAck {
    pub session_id: Uuid,
    pub file_id: Uuid,
    pub seq: u32,
    pub committed_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferStats {
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub elapsed_ms: u64,
    pub files_total: u32,
    pub files_completed: u32,
    pub files_failed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Done {
    pub session_id: Uuid,
    pub stats: TransferStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoneAck {
    pub session_id: Uuid,
    pub success: bool,
    pub stats: TransferStats,
    pub failed_files: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WireMessage {
    Hello(Hello),
    CapsAck(CapsAck),
    PairRequest(PairRequest),
    PairAccept(PairAccept),
    Manifest(Manifest),
    ManifestAck(ManifestAck),
    Chunk(Chunk),
    ChunkAck(ChunkAck),
    Done(Done),
    DoneAck(DoneAck),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_message_roundtrip() {
        let sid = Uuid::new_v4();
        let msg = WireMessage::Hello(Hello {
            device_id: Uuid::new_v4(),
            session_id: sid,
            device_name: "TestDevice".into(),
            os: "macos".into(),
            cert_fingerprint: "abc123".into(),
            capabilities: Capabilities::default(),
        });
        let encoded = rmp_serde::to_vec(&msg).unwrap();
        let decoded: WireMessage = rmp_serde::from_slice(&encoded).unwrap();
        assert!(matches!(decoded, WireMessage::Hello(_)));
    }

    #[test]
    fn chunk_roundtrip() {
        let msg = WireMessage::Chunk(Chunk {
            session_id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            seq: 42,
            offset: 1024,
            data: vec![0u8; 256],
            eof: false,
        });
        let encoded = rmp_serde::to_vec(&msg).unwrap();
        let decoded: WireMessage = rmp_serde::from_slice(&encoded).unwrap();
        assert!(matches!(decoded, WireMessage::Chunk(_)));
    }
}
