use std::path::{Path, PathBuf};

use anyhow::Result;
use landrop_protocol::{FileEntry, Manifest};
use uuid::Uuid;
use walkdir::WalkDir;

pub struct ManifestBuilder;

impl ManifestBuilder {
    pub fn build(paths: &[PathBuf]) -> Result<Manifest> {
        let session_id = Uuid::new_v4();
        let mut files = Vec::new();
        let mut total_bytes = 0u64;

        let transfer_name = paths
            .first()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "transfer".to_string());

        for path in paths {
            if path.is_dir() {
                Self::walk_dir(path, path, &mut files, &mut total_bytes)?;
            } else if path.is_file() {
                let rel = path.file_name().unwrap().to_string_lossy().to_string();
                let entry = Self::file_entry(path, &rel)?;
                total_bytes += entry.size;
                files.push(entry);
            }
        }

        Ok(Manifest { session_id, transfer_name, files, total_bytes })
    }

    fn walk_dir(
        root: &Path,
        dir: &Path,
        files: &mut Vec<FileEntry>,
        total_bytes: &mut u64,
    ) -> Result<()> {
        let root_parent = root.parent().unwrap_or(root);

        for entry in WalkDir::new(dir).follow_links(false).sort_by_file_name() {
            let entry = entry?;
            let rel = entry.path().strip_prefix(root_parent)?;
            let rel_str = Self::normalize_path(rel)?;

            if entry.file_type().is_dir() {
                files.push(FileEntry {
                    file_id: Uuid::new_v4(),
                    relative_path: rel_str,
                    size: 0,
                    is_dir: true,
                });
            } else {
                let size = entry.metadata()?.len();
                *total_bytes += size;
                files.push(FileEntry {
                    file_id: Uuid::new_v4(),
                    relative_path: rel_str,
                    size,
                    is_dir: false,
                });
            }
        }
        Ok(())
    }

    fn file_entry(path: &Path, rel_path: &str) -> Result<FileEntry> {
        let meta = std::fs::metadata(path)?;
        Ok(FileEntry {
            file_id: Uuid::new_v4(),
            relative_path: rel_path.to_string(),
            size: meta.len(),
            is_dir: false,
        })
    }

    fn normalize_path(path: &Path) -> Result<String> {
        let s = path.to_string_lossy();
        if s.contains("..") {
            anyhow::bail!("path traversal detected: {}", s);
        }
        Ok(s.replace('\\', "/"))
    }
}
