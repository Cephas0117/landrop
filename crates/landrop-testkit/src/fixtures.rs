use std::path::PathBuf;

use tempfile::TempDir;

pub struct TempFiles {
    pub dir: TempDir,
}

impl TempFiles {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self { dir: tempfile::tempdir()? })
    }

    pub fn create_file(&self, name: &str, size_bytes: usize) -> anyhow::Result<PathBuf> {
        let path = self.dir.path().join(name);
        let data: Vec<u8> = (0..size_bytes).map(|i| (i % 256) as u8).collect();
        std::fs::write(&path, data)?;
        Ok(path)
    }
}
