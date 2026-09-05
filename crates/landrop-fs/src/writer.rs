use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use landrop_protocol::{FileAck, FileDecision, FileEntry, Manifest, ManifestAck};
use tokio::fs::{self, File, OpenOptions};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

pub struct FileWriter {
    receive_dir: PathBuf,
    handles: HashMap<Uuid, FileHandle>,
}

struct FileHandle {
    file: File,
    written: u64,
    expected_size: u64,
}

impl FileWriter {
    pub fn new(receive_dir: PathBuf) -> Self {
        Self {
            receive_dir,
            handles: HashMap::new(),
        }
    }

    pub async fn prepare_manifest(&mut self, manifest: &Manifest) -> Result<ManifestAck> {
        let mut file_acks = Vec::new();

        for entry in &manifest.files {
            let dest = self.receive_dir.join(&entry.relative_path);
            Self::validate_dest(&self.receive_dir, &dest)?;

            let ack = if entry.is_dir {
                fs::create_dir_all(&dest).await?;
                FileAck {
                    file_id: entry.file_id,
                    decision: FileDecision::SkipAlreadyPresent,
                }
            } else {
                self.prepare_file(entry, &dest).await?
            };
            file_acks.push(ack);
        }

        Ok(ManifestAck {
            session_id: manifest.session_id,
            files: file_acks,
        })
    }

    async fn prepare_file(&mut self, entry: &FileEntry, dest: &Path) -> Result<FileAck> {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Size alone does not prove that an existing file is the same source.
        // Never skip, append to, or truncate an unrelated file on that basis.
        let mut candidate = dest.to_path_buf();
        let mut suffix = 0u64;
        let file = loop {
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&candidate)
                .await
            {
                Ok(file) => break file,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    suffix += 1;
                    let mut name = dest.file_stem().unwrap_or_default().to_os_string();
                    name.push(format!(" ({suffix})"));
                    if let Some(ext) = dest.extension() {
                        name.push(".");
                        name.push(ext);
                    }
                    candidate = dest.with_file_name(name);
                }
                Err(e) => return Err(e.into()),
            }
        };
        self.handles.insert(
            entry.file_id,
            FileHandle {
                file,
                written: 0,
                expected_size: entry.size,
            },
        );
        Ok(FileAck {
            file_id: entry.file_id,
            decision: FileDecision::Send,
        })
    }

    pub async fn write_chunk(&mut self, file_id: Uuid, data: &[u8]) -> Result<u64> {
        let handle = self
            .handles
            .get_mut(&file_id)
            .ok_or_else(|| anyhow::anyhow!("no open handle for file {}", file_id))?;
        anyhow::ensure!(
            data.len() as u64 <= handle.expected_size - handle.written,
            "chunk exceeds declared file size"
        );
        handle.file.write_all(data).await?;
        handle.written += data.len() as u64;
        Ok(handle.written)
    }

    pub async fn write_chunk_at(&mut self, file_id: Uuid, offset: u64, data: &[u8]) -> Result<u64> {
        let handle = self
            .handles
            .get(&file_id)
            .ok_or_else(|| anyhow::anyhow!("no open handle for file {file_id}"))?;
        anyhow::ensure!(offset == handle.written, "chunk offset mismatch");
        self.write_chunk(file_id, data).await
    }

    pub async fn finalize_file(&mut self, file_id: Uuid) -> Result<()> {
        let handle = self
            .handles
            .get(&file_id)
            .ok_or_else(|| anyhow::anyhow!("no open handle for file {file_id}"))?;
        anyhow::ensure!(
            handle.written == handle.expected_size,
            "incomplete file at EOF"
        );
        if let Some(mut handle) = self.handles.remove(&file_id) {
            handle.file.flush().await?;
            handle.file.sync_all().await?;
        }
        Ok(())
    }

    pub async fn finalize_all(&mut self) -> Result<()> {
        let ids: Vec<Uuid> = self.handles.keys().copied().collect();
        for id in ids {
            self.finalize_file(id).await?;
        }
        Ok(())
    }

    fn validate_dest(receive_dir: &Path, dest: &Path) -> Result<()> {
        let dest_str = dest.to_string_lossy();
        if dest_str.contains("..") {
            anyhow::bail!("path traversal in destination: {}", dest_str);
        }
        let recv_str = receive_dir.to_string_lossy();
        let dest_abs = if dest.is_absolute() {
            dest.to_path_buf()
        } else {
            receive_dir.join(dest)
        };
        if !dest_abs.to_string_lossy().starts_with(recv_str.as_ref()) {
            anyhow::bail!("destination escapes receive dir: {}", dest_abs.display());
        }
        Ok(())
    }
}
