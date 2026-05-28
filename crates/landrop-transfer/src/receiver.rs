use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;
use futures::{SinkExt, StreamExt};
use landrop_fs::writer::FileWriter;
use landrop_protocol::codec::FrameCodec;
use landrop_protocol::{ChunkAck, DoneAck, Manifest, TransferStats, WireMessage};
use landrop_state::transfer::TransferProgress;
use tokio::sync::mpsc;
use tokio_util::codec::Framed;
use uuid::Uuid;

use crate::ewma::EwmaTracker;

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
        let mut framed = Framed::new(stream, FrameCodec::default());

        let manifest = match framed.next().await {
            Some(Ok(WireMessage::Manifest(m))) => m,
            Some(Ok(other)) => anyhow::bail!("expected Manifest, got {:?}", other),
            Some(Err(e)) => return Err(e.into()),
            None => anyhow::bail!("connection closed before Manifest"),
        };

        Self::transfer(framed, manifest, receive_dir, progress_tx).await
    }

    /// Called by the engine when the first wire message has already been read and routed.
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
        framed.send(WireMessage::ManifestAck(manifest_ack)).await?;

        let mut bytes_received = 0u64;
        let mut files_done = 0u32;
        let mut ewma = EwmaTracker::new();
        let mut chunk_count = 0u32;
        let started = Instant::now();

        loop {
            match framed.next().await {
                Some(Ok(WireMessage::Chunk(chunk))) => {
                    if !chunk.eof {
                        let written = writer.write_chunk(chunk.file_id, &chunk.data).await?;
                        bytes_received += chunk.data.len() as u64;
                        chunk_count += 1;

                        if chunk_count % ACK_EVERY == 0 {
                            framed
                                .send(WireMessage::ChunkAck(ChunkAck {
                                    session_id,
                                    file_id: chunk.file_id,
                                    seq: chunk.seq,
                                    committed_bytes: written,
                                }))
                                .await?;
                        }
                    } else {
                        writer.finalize_file(chunk.file_id).await?;
                        files_done += 1;
                        chunk_count = 0;

                        framed
                            .send(WireMessage::ChunkAck(ChunkAck {
                                session_id,
                                file_id: chunk.file_id,
                                seq: chunk.seq,
                                committed_bytes: bytes_received,
                            }))
                            .await?;
                    }

                    let speed = ewma.update(bytes_received);
                    let remaining = total_bytes.saturating_sub(bytes_received);
                    let eta = if speed > 0.0 { remaining as f64 / speed } else { 0.0 };
                    let _ = progress_tx.try_send(TransferProgress {
                        bytes_sent: bytes_received,
                        total_bytes,
                        speed_bps: speed,
                        eta_secs: eta,
                        files_done,
                        files_total,
                    });
                }
                Some(Ok(WireMessage::Done(_done))) => {
                    writer.finalize_all().await?;
                    let elapsed = started.elapsed().as_millis() as u64;
                    framed
                        .send(WireMessage::DoneAck(DoneAck {
                            session_id,
                            success: true,
                            stats: TransferStats {
                                total_bytes,
                                transferred_bytes: bytes_received,
                                elapsed_ms: elapsed,
                                files_total,
                                files_completed: files_done,
                                files_failed: 0,
                            },
                            failed_files: vec![],
                        }))
                        .await?;
                    break;
                }
                Some(Ok(other)) => anyhow::bail!("unexpected during transfer: {:?}", other),
                Some(Err(e)) => return Err(e.into()),
                None => {
                    writer.finalize_all().await?;
                    break;
                }
            }
        }

        Ok(manifest)
    }
}
