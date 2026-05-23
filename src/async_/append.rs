use std::env;
use std::path::{Path, PathBuf};

use tokio::io::AsyncWriteExt;

use super::{cleanup_backup_file, create_async_backup_file, AsyncOpFuture, AsyncRollbackableOperation};

/// Asynchronously appends data to a file.
pub struct AsyncAppendFile {
    path: PathBuf,
    temp_dir: PathBuf,
    backup_path: PathBuf,
    data: Vec<u8>,
}

impl AsyncAppendFile {
    /// Constructs a new `AsyncAppendFile` operation, using the OS temp directory for backups.
    pub fn new<S: AsRef<Path>>(path: S, data: Vec<u8>) -> Self {
        Self::with_temp_dir(path, env::temp_dir(), data)
    }

    /// Constructs a new `AsyncAppendFile` operation with a custom backup directory.
    pub fn with_temp_dir<S: AsRef<Path>, T: AsRef<Path>>(path: S, temp_dir: T, data: Vec<u8>) -> Self {
        Self {
            path: path.as_ref().into(),
            temp_dir: temp_dir.as_ref().into(),
            backup_path: PathBuf::new(),
            data,
        }
    }
}

impl AsyncRollbackableOperation for AsyncAppendFile {
    fn execute(&mut self) -> AsyncOpFuture<'_> {
        Box::pin(async move {
            self.backup_path = create_async_backup_file(&self.path, &self.temp_dir).await?;
            let mut file = tokio::fs::OpenOptions::new()
                .append(true)
                .open(&self.path)
                .await?;
            file.write_all(&self.data).await
        })
    }

    fn rollback(&mut self) -> AsyncOpFuture<'_> {
        Box::pin(async move {
            if self.backup_path.as_os_str().is_empty() {
                return Ok(());
            }
            tokio::fs::copy(&self.backup_path, &self.path).await.map(|_| ())
        })
    }
}

impl Drop for AsyncAppendFile {
    fn drop(&mut self) {
        cleanup_backup_file(&self.backup_path);
    }
}
