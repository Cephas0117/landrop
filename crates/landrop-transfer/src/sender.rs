use std::path::PathBuf;
use std::time::Instant;

use anyhow::{ensure, Context, Result};
use landrop_fs::ManifestBuilder;
use landrop_protocol::{
    codec::FrameCodec, Chunk, Done, FileDecision, Manifest, TransferStats, WireMessage,
};
use landrop_state::transfer::TransferProgress;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, BufReader};
use tokio::sync::mpsc;
use tokio_util::codec::Framed;
use uuid::Uuid;

use crate::{ewma::EwmaTracker, read_message, send_message};

const CHUNK_SIZE: usize = 256 * 1024;
const ACK_EVERY: u32 = 4;
type ClientStream = tokio_rustls::client::TlsStream<tokio::net::TcpStream>;
type ClientFramed = Framed<ClientStream, FrameCodec>;

pub struct Sender;

impl Sender {
    pub async fn run(
        stream: ClientStream,
        paths: Vec<PathBuf>,
        _transfer_id: Uuid,
        progress_tx: mpsc::Sender<TransferProgress>,
        cancel_rx: mpsc::Receiver<()>,
    ) -> Result<()> {
        let manifest = ManifestBuilder::build(&paths)?;
        Self::run_prepared(stream, paths, manifest, progress_tx, cancel_rx).await
    }

    pub(crate) async fn run_prepared(
        stream: ClientStream,
        paths: Vec<PathBuf>,
        manifest: Manifest,
        progress_tx: mpsc::Sender<TransferProgress>,
        mut cancel_rx: mpsc::Receiver<()>,
    ) -> Result<()> {
        tokio::select! {
            biased;
            _ = cancel_rx.recv() => anyhow::bail!("transfer canceled"),
            result = Self::transfer(stream, paths, manifest, progress_tx) => result,
        }
    }

    async fn transfer(
        stream: ClientStream,
        paths: Vec<PathBuf>,
        manifest: Manifest,
        progress_tx: mpsc::Sender<TransferProgress>,
    ) -> Result<()> {
        let session_id = manifest.session_id;
        let total_bytes = manifest.total_bytes;
        let files_total = manifest.files.iter().filter(|f| !f.is_dir).count() as u32;
        let mut framed = Framed::new(stream, FrameCodec);
        send_message(&mut framed, WireMessage::Manifest(manifest.clone())).await?;

        let ack = match read_message(&mut framed, "ManifestAck").await? {
            WireMessage::ManifestAck(a) => a,
            other => anyhow::bail!("expected ManifestAck, got {other:?}"),
        };
        ensure!(ack.session_id == session_id, "ManifestAck session mismatch");
        ensure!(
            ack.files.len() == manifest.files.len(),
            "ManifestAck file count mismatch"
        );
        for (entry, file_ack) in manifest.files.iter().zip(&ack.files) {
            ensure!(
                entry.file_id == file_ack.file_id,
                "ManifestAck file order mismatch"
            );
        }

        let mut bytes_sent = 0u64;
        let mut files_done = 0u32;
        let mut ewma = EwmaTracker::new();
        let started = Instant::now();

        for (entry, file_ack) in manifest.files.iter().zip(&ack.files) {
            if entry.is_dir {
                continue;
            }
            if file_ack.decision == FileDecision::SkipAlreadyPresent {
                files_done += 1;
                bytes_sent += entry.size;
                continue;
            }
            let resume_offset = match file_ack.decision {
                FileDecision::RestartFile { resume_offset } => resume_offset,
                _ => 0,
            };
            ensure!(resume_offset <= entry.size, "invalid resume offset");
            let src_path = find_source_path(&paths, &entry.relative_path)
                .with_context(|| format!("source not found: {}", entry.relative_path))?;
            let mut reader = BufReader::new(File::open(src_path).await?);
            reader.seek(std::io::SeekFrom::Start(resume_offset)).await?;
            bytes_sent += resume_offset;
            let mut offset = resume_offset;
            let mut seq = 0u32;
            let mut buf = vec![0u8; CHUNK_SIZE];

            while offset < entry.size {
                let n = (entry.size - offset).min(CHUNK_SIZE as u64) as usize;
                reader
                    .read_exact(&mut buf[..n])
                    .await
                    .context("source file changed during transfer")?;
                send_message(
                    &mut framed,
                    WireMessage::Chunk(Chunk {
                        session_id,
                        file_id: entry.file_id,
                        seq,
                        offset,
                        data: buf[..n].to_vec(),
                        eof: false,
                    }),
                )
                .await?;
                offset += n as u64;
                bytes_sent += n as u64;
                if (seq + 1) % ACK_EVERY == 0 {
                    expect_chunk_ack(&mut framed, session_id, entry.file_id, seq).await?;
                }
                seq += 1;
                let speed = ewma.update(bytes_sent);
                let _ = progress_tx.try_send(TransferProgress {
                    bytes_sent,
                    total_bytes,
                    speed_bps: speed,
                    eta_secs: if speed > 0.0 {
                        total_bytes.saturating_sub(bytes_sent) as f64 / speed
                    } else {
                        0.0
                    },
                    files_done,
                    files_total,
                });
            }

            send_message(
                &mut framed,
                WireMessage::Chunk(Chunk {
                    session_id,
                    file_id: entry.file_id,
                    seq,
                    offset: entry.size,
                    data: vec![],
                    eof: true,
                }),
            )
            .await?;
            // Every EOF has its own acknowledgement, including empty files and
            // files ending exactly on a four-chunk boundary.
            expect_chunk_ack(&mut framed, session_id, entry.file_id, seq).await?;
            files_done += 1;
        }

        send_message(
            &mut framed,
            WireMessage::Done(Done {
                session_id,
                stats: TransferStats {
                    total_bytes,
                    transferred_bytes: bytes_sent,
                    elapsed_ms: started.elapsed().as_millis() as u64,
                    files_total,
                    files_completed: files_done,
                    files_failed: 0,
                },
            }),
        )
        .await?;
        match read_message(&mut framed, "DoneAck").await? {
            WireMessage::DoneAck(ack) => {
                ensure!(ack.session_id == session_id, "DoneAck session mismatch");
                ensure!(
                    ack.success && ack.failed_files.is_empty() && ack.stats.files_failed == 0,
                    "receiver failed to save all files"
                );
            }
            other => anyhow::bail!("expected DoneAck, got {other:?}"),
        }
        let _ = progress_tx
            .send(TransferProgress {
                bytes_sent: total_bytes,
                total_bytes,
                speed_bps: 0.0,
                eta_secs: 0.0,
                files_done,
                files_total,
            })
            .await;
        Ok(())
    }
}

async fn expect_chunk_ack(
    framed: &mut ClientFramed,
    session_id: Uuid,
    file_id: Uuid,
    seq: u32,
) -> Result<()> {
    match read_message(framed, "ChunkAck").await? {
        WireMessage::ChunkAck(ack) => ensure!(
            ack.session_id == session_id && ack.file_id == file_id && ack.seq == seq,
            "ChunkAck does not match the sent chunk"
        ),
        other => anyhow::bail!("expected ChunkAck, got {other:?}"),
    }
    Ok(())
}

fn find_source_path(paths: &[PathBuf], relative_path: &str) -> Option<PathBuf> {
    for p in paths {
        if p.is_file() {
            if p.file_name()
                .map(|n| n.to_string_lossy() == relative_path)
                .unwrap_or(false)
            {
                return Some(p.clone());
            }
        } else if p.is_dir() {
            let candidate = p.parent().unwrap_or(p).join(relative_path);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}
