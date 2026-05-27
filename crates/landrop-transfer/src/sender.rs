use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;
use futures::{SinkExt, StreamExt};
use landrop_fs::ManifestBuilder;
use landrop_protocol::{
    Chunk, Done, FileDecision, TransferStats, WireMessage,
};
use landrop_state::transfer::TransferProgress;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, BufReader};
use tokio::sync::mpsc;
use tokio_util::codec::Framed;
use uuid::Uuid;

use crate::ewma::EwmaTracker;
use landrop_protocol::codec::FrameCodec;

const CHUNK_SIZE: usize = 256 * 1024; // 256KB
const ACK_EVERY: u32 = 4;

pub struct Sender;

impl Sender {
    pub async fn run(
        stream: tokio_rustls::client::TlsStream<tokio::net::TcpStream>,
        paths: Vec<PathBuf>,
        _transfer_id: Uuid,
        progress_tx: mpsc::Sender<TransferProgress>,
        mut cancel_rx: mpsc::Receiver<()>,
    ) -> Result<()> {
        let manifest = ManifestBuilder::build(&paths)?;
        let session_id = manifest.session_id;
        let total_bytes = manifest.total_bytes;
        let files_total = manifest.files.iter().filter(|f| !f.is_dir).count() as u32;

        let mut framed = Framed::new(stream, FrameCodec::default());

        framed.send(WireMessage::Manifest(manifest.clone())).await?;

        let ack = match framed.next().await {
            Some(Ok(WireMessage::ManifestAck(a))) => a,
            Some(Ok(other)) => anyhow::bail!("expected ManifestAck, got {:?}", other),
            Some(Err(e)) => return Err(e.into()),
            None => anyhow::bail!("connection closed waiting for ManifestAck"),
        };

        let mut bytes_sent = 0u64;
        let mut files_done = 0u32;
        let mut ewma = EwmaTracker::new();
        let started = Instant::now();

        for (entry, file_ack) in manifest.files.iter().zip(ack.files.iter()) {
            if entry.is_dir {
                continue;
            }
            if let FileDecision::SkipAlreadyPresent = file_ack.decision {
                files_done += 1;
                continue;
            }

            let resume_offset = if let FileDecision::RestartFile { resume_offset } = file_ack.decision {
                resume_offset
            } else {
                0
            };

            let src_path = find_source_path(&paths, &entry.relative_path)
                .ok_or_else(|| anyhow::anyhow!("source not found: {}", entry.relative_path))?;

            let file = File::open(&src_path).await?;
            let mut reader = BufReader::new(file);

            if resume_offset > 0 {
                use tokio::io::AsyncSeekExt;
                reader.seek(std::io::SeekFrom::Start(resume_offset)).await?;
                bytes_sent += resume_offset;
            }

            let mut seq = 0u32;
            let mut buf = vec![0u8; CHUNK_SIZE];
            let mut chunks_since_ack = 0u32;

            loop {
                // Check cancel
                if cancel_rx.try_recv().is_ok() {
                    anyhow::bail!("transfer canceled");
                }

                let n = reader.read(&mut buf).await?;
                if n == 0 { break; }

                let offset = resume_offset + (seq as u64 * CHUNK_SIZE as u64);
                framed.send(WireMessage::Chunk(Chunk {
                    session_id,
                    file_id: entry.file_id,
                    seq,
                    offset,
                    data: buf[..n].to_vec(),
                    eof: false,
                })).await?;
                seq += 1;
                bytes_sent += n as u64;
                chunks_since_ack += 1;

                if chunks_since_ack >= ACK_EVERY {
                    match framed.next().await {
                        Some(Ok(WireMessage::ChunkAck(_))) => {}
                        _ => anyhow::bail!("expected ChunkAck"),
                    }
                    chunks_since_ack = 0;
                }

                let speed = ewma.update(bytes_sent);
                let remaining = total_bytes.saturating_sub(bytes_sent);
                let eta = if speed > 0.0 { remaining as f64 / speed } else { 0.0 };
                let _ = progress_tx.try_send(TransferProgress {
                    bytes_sent,
                    total_bytes,
                    speed_bps: speed,
                    eta_secs: eta,
                    files_done,
                    files_total,
                });
            }

            // EOF chunk
            framed.send(WireMessage::Chunk(Chunk {
                session_id,
                file_id: entry.file_id,
                seq,
                offset: entry.size,
                data: vec![],
                eof: true,
            })).await?;

            if chunks_since_ack > 0 {
                match framed.next().await {
                    Some(Ok(WireMessage::ChunkAck(_))) => {}
                    _ => {}
                }
            }

            files_done += 1;
        }

        let elapsed = started.elapsed().as_millis() as u64;
        framed.send(WireMessage::Done(Done {
            session_id,
            stats: TransferStats {
                total_bytes,
                transferred_bytes: bytes_sent,
                elapsed_ms: elapsed,
                files_total,
                files_completed: files_done,
                files_failed: 0,
            },
        })).await?;

        match framed.next().await {
            Some(Ok(WireMessage::DoneAck(_))) => {}
            _ => {}
        }

        Ok(())
    }
}

fn find_source_path(paths: &[PathBuf], relative_path: &str) -> Option<PathBuf> {
    for p in paths {
        if p.is_file() {
            if p.file_name().map(|n| n.to_string_lossy() == relative_path).unwrap_or(false) {
                return Some(p.clone());
            }
        } else if p.is_dir() {
            let parent = p.parent().unwrap_or(p);
            let candidate = parent.join(relative_path);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}
