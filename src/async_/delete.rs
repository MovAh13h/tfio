use std::env;
use std::io;
use std::path::{Path, PathBuf};

use super::{
    async_copy_dir, cleanup_backup_dir, cleanup_backup_file, create_async_backup_file,
    create_async_backup_folder, AsyncOpFuture, AsyncRollbackableOperation,
};

/// Asynchronously deletes a file (backed up for rollback).
pub struct AsyncDeleteFile {
    source: PathBuf,
    temp_dir: PathBuf,
    backup_path: PathBuf,
}

impl AsyncDeleteFile {
    /// Constructs a new `AsyncDeleteFile` operation, using the OS temp directory for backups.
    pub fn new<S: AsRef<Path>>(source: S) -> Self {
        Self::with_temp_dir(source, env::temp_dir())
    }

    /// Constructs a new `AsyncDeleteFile` operation with a custom backup directory.
    pub fn with_temp_dir<S: AsRef<Path>, T: AsRef<Path>>(source: S, temp_dir: T) -> Self {
        Self {
            source: source.as_ref().into(),
            temp_dir: temp_dir.as_ref().into(),
            backup_path: PathBuf::new(),
        }
    }
}

impl AsyncRollbackableOperation for AsyncDeleteFile {
    fn execute(&mut self) -> AsyncOpFuture<'_> {
        Box::pin(async move {
            self.backup_path = create_async_backup_file(&self.source, &self.temp_dir).await?;
            tokio::fs::remove_file(&self.source).await
        })
    }

    fn rollback(&mut self) -> AsyncOpFuture<'_> {
        Box::pin(async move {
            if self.backup_path.as_os_str().is_empty() {
                return Ok(());
            }
            tokio::fs::copy(&self.backup_path, &self.source).await.map(|_| ())
        })
    }
}

impl Drop for AsyncDeleteFile {
    fn drop(&mut self) {
        cleanup_backup_file(&self.backup_path);
    }
}

/// Asynchronously deletes a directory (backed up for rollback).
pub struct AsyncDeleteDirectory {
    source: PathBuf,
    temp_dir: PathBuf,
    backup_path: PathBuf,
}

impl AsyncDeleteDirectory {
    /// Constructs a new `AsyncDeleteDirectory` operation, using the OS temp directory for backups.
    pub fn new<S: AsRef<Path>>(source: S) -> Self {
        Self::with_temp_dir(source, env::temp_dir())
    }

    /// Constructs a new `AsyncDeleteDirectory` operation with a custom backup directory.
    pub fn with_temp_dir<S: AsRef<Path>, T: AsRef<Path>>(source: S, temp_dir: T) -> Self {
        Self {
            source: source.as_ref().into(),
            temp_dir: temp_dir.as_ref().into(),
            backup_path: PathBuf::new(),
        }
    }
}

impl AsyncRollbackableOperation for AsyncDeleteDirectory {
    fn execute(&mut self) -> AsyncOpFuture<'_> {
        Box::pin(async move {
            self.backup_path = create_async_backup_folder(&self.source, &self.temp_dir).await?;
            tokio::fs::remove_dir_all(&self.source).await
        })
    }

    fn rollback(&mut self) -> AsyncOpFuture<'_> {
        Box::pin(async move {
            if self.backup_path.as_os_str().is_empty() {
                return Ok(());
            }
            match tokio::fs::rename(&self.backup_path, &self.source).await {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == io::ErrorKind::CrossesDevices => {
                    async_copy_dir(&self.backup_path, &self.source).await?;
                    tokio::fs::remove_dir_all(&self.backup_path).await
                }
                Err(e) => Err(e),
            }
        })
    }
}

impl Drop for AsyncDeleteDirectory {
    fn drop(&mut self) {
        cleanup_backup_dir(&self.backup_path);
    }
}
