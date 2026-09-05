use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{ensure, Result};
use landrop_fs::writer::FileWriter;
use landrop_protocol::{
    codec::FrameCodec, ChunkAck, DoneAck, FileDecision, Manifest, TransferStats, WireMessage,
};
use landrop_state::transfer::TransferProgress;
use tokio::sync::mpsc;
use tokio_util::codec::Framed;
use uuid::Uuid;

use crate::{ewma::EwmaTracker, read_message, send_message};

const ACK_EVERY: u32 = 4;
type ServerFramed = Framed<tokio_rustls::server::TlsStream<tokio::net::TcpStream>, FrameCodec>;

pub struct Receiver;

impl Receiver {
    pub async fn run(
        stream: tokio_rustls::server::TlsStream<tokio::net::TcpStream>,
        receive_dir: PathBuf,
        _transfer_id: Uuid,
        progress_tx: mpsc::Sender<TransferProgress>,
    ) -> Result<Manifest> {
        let mut framed = Framed::new(stream, FrameCodec);
        let manifest = match read_message(&mut framed, "Manifest").await? {
            WireMessage::Manifest(m) => m,
            other => anyhow::bail!("expected Manifest, got {other:?}"),
        };
        Self::transfer(framed, manifest, receive_dir, progress_tx).await
    }

    pub async fn run_from_manifest(
        framed: ServerFramed,
        manifest: Manifest,
        receive_dir: PathBuf,
        progress_tx: mpsc::Sender<TransferProgress>,
    ) -> Result<Manifest> {
        Self::transfer(framed, manifest, receive_dir, progress_tx).await
    }

    async fn transfer(
        mut framed: ServerFramed,
        manifest: Manifest,
        receive_dir: PathBuf,
        progress_tx: mpsc::Sender<TransferProgress>,
    ) -> Result<Manifest> {
        let session_id = manifest.session_id;
        let total_bytes = manifest.total_bytes;
        let files_total = manifest.files.iter().filter(|f| !f.is_dir).count() as u32;
        let mut writer = FileWriter::new(receive_dir);
        let manifest_ack = writer.prepare_manifest(&manifest).await?;
        let mut bytes_received = 0u64;
        let mut completed = HashSet::new();
        for (entry, ack) in manifest.files.iter().zip(&manifest_ack.files) {
            if entry.is_dir {
                continue;
            }
            match ack.decision {
                FileDecision::SkipAlreadyPresent => {
                    completed.insert(entry.file_id);
                    bytes_received += entry.size;
                }
                FileDecision::RestartFile { resume_offset } => bytes_received += resume_offset,
                FileDecision::Send => {}
            }
        }
        send_message(&mut framed, WireMessage::ManifestAck(manifest_ack)).await?;
        let mut ewma = EwmaTracker::new();
        let mut active_file = None;
        let mut next_seq = 0u32;
        let started = Instant::now();

        loop {
            match read_message(&mut framed, "Chunk / Done").await? {
                WireMessage::Chunk(chunk) => {
                    ensure!(chunk.session_id == session_id, "chunk session mismatch");
                    ensure!(
                        !completed.contains(&chunk.file_id),
                        "chunk for completed file"
                    );
                    let current = active_file.get_or_insert(chunk.file_id);
                    ensure!(
                        *current == chunk.file_id && chunk.seq == next_seq,
                        "chunk sequence mismatch"
                    );
                    let written = writer
                        .write_chunk_at(chunk.file_id, chunk.offset, &chunk.data)
                        .await?;
                    bytes_received += chunk.data.len() as u64;
                    if chunk.eof {
                        ensure!(chunk.data.is_empty(), "EOF chunk contains data");
                        writer.finalize_file(chunk.file_id).await?;
                        completed.insert(chunk.file_id);
                        active_file = None;
                        next_seq = 0;
                    } else {
                        next_seq += 1;
                    }
                    if chunk.eof || next_seq % ACK_EVERY == 0 {
                        send_message(
                            &mut framed,
                            WireMessage::ChunkAck(ChunkAck {
                                session_id,
                                file_id: chunk.file_id,
                                seq: chunk.seq,
                                committed_bytes: written,
                            }),
                        )
                        .await?;
                    }
                    let speed = ewma.update(bytes_received);
                    let _ = progress_tx.try_send(TransferProgress {
                        bytes_sent: bytes_received,
                        total_bytes,
                        speed_bps: speed,
                        eta_secs: if speed > 0.0 {
                            total_bytes.saturating_sub(bytes_received) as f64 / speed
                        } else {
                            0.0
                        },
                        files_done: completed.len() as u32,
                        files_total,
                    });
                }
                WireMessage::Done(done) => {
                    ensure!(done.session_id == session_id, "Done session mismatch");
                    ensure!(
                        completed.len() as u32 == files_total && bytes_received == total_bytes,
                        "transfer ended before all files were received"
                    );
                    send_message(
                        &mut framed,
                        WireMessage::DoneAck(DoneAck {
                            session_id,
                            success: true,
                            stats: TransferStats {
                                total_bytes,
                                transferred_bytes: bytes_received,
                                elapsed_ms: started.elapsed().as_millis() as u64,
                                files_total,
                                files_completed: completed.len() as u32,
                                files_failed: 0,
                            },
                            failed_files: vec![],
                        }),
                    )
                    .await?;
                    let _ = progress_tx
                        .send(TransferProgress {
                            bytes_sent: total_bytes,
                            total_bytes,
                            speed_bps: 0.0,
                            eta_secs: 0.0,
                            files_done: files_total,
                            files_total,
                        })
                        .await;
                    return Ok(manifest);
                }
                other => anyhow::bail!("unexpected during transfer: {other:?}"),
            }
        }
    }
}
