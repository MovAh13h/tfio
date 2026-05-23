use std::env;
use std::path::{Path, PathBuf};

use uuid::Uuid;

use super::{
    async_copy_dir, cleanup_backup_dir, cleanup_backup_file, AsyncOpFuture,
    AsyncRollbackableOperation,
};

/// Asynchronously copies a file to the destination.
///
/// If the destination already exists it is backed up and restored on rollback.
pub struct AsyncCopyFile {
    source: PathBuf,
    dest: PathBuf,
    temp_dir: PathBuf,
    backup_path: PathBuf,
    dest_existed: bool,
}

impl AsyncCopyFile {
    /// Constructs a new `AsyncCopyFile` operation, using the OS temp directory for backups.
    pub fn new<S: AsRef<Path>, T: AsRef<Path>>(source: S, dest: T) -> Self {
        Self::with_temp_dir(source, dest, env::temp_dir())
    }

    /// Constructs a new `AsyncCopyFile` operation with a custom backup directory.
    pub fn with_temp_dir<S: AsRef<Path>, T: AsRef<Path>, U: AsRef<Path>>(
        source: S,
        dest: T,
        temp_dir: U,
    ) -> Self {
        Self {
            source: source.as_ref().into(),
            dest: dest.as_ref().into(),
            temp_dir: temp_dir.as_ref().into(),
            backup_path: PathBuf::new(),
            dest_existed: false,
        }
    }
}

impl AsyncRollbackableOperation for AsyncCopyFile {
    fn execute(&mut self) -> AsyncOpFuture<'_> {
        Box::pin(async move {
            if tokio::fs::metadata(&self.dest).await.is_ok() {
                self.dest_existed = true;
                tokio::fs::create_dir_all(&self.temp_dir).await?;
                self.backup_path = self.temp_dir.join(Uuid::new_v4().to_string());
                tokio::fs::copy(&self.dest, &self.backup_path).await?;
            }
            tokio::fs::copy(&self.source, &self.dest).await.map(|_| ())
        })
    }

    fn rollback(&mut self) -> AsyncOpFuture<'_> {
        Box::pin(async move {
            if self.dest_existed {
                tokio::fs::copy(&self.backup_path, &self.dest).await.map(|_| ())
            } else {
                match tokio::fs::remove_file(&self.dest).await {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                    other => other,
                }
            }
        })
    }
}

impl Drop for AsyncCopyFile {
    fn drop(&mut self) {
        cleanup_backup_file(&self.backup_path);
    }
}

/// Asynchronously copies a directory to the destination.
///
/// If the destination already exists it is backed up and restored on rollback.
pub struct AsyncCopyDirectory {
    source: PathBuf,
    dest: PathBuf,
    temp_dir: PathBuf,
    backup_path: PathBuf,
    dest_existed: bool,
}

impl AsyncCopyDirectory {
    /// Constructs a new `AsyncCopyDirectory` operation, using the OS temp directory for backups.
    pub fn new<S: AsRef<Path>, T: AsRef<Path>>(source: S, dest: T) -> Self {
        Self::with_temp_dir(source, dest, env::temp_dir())
    }

    /// Constructs a new `AsyncCopyDirectory` operation with a custom backup directory.
    pub fn with_temp_dir<S: AsRef<Path>, T: AsRef<Path>, U: AsRef<Path>>(
        source: S,
        dest: T,
        temp_dir: U,
    ) -> Self {
        Self {
            source: source.as_ref().into(),
            dest: dest.as_ref().into(),
            temp_dir: temp_dir.as_ref().into(),
            backup_path: PathBuf::new(),
            dest_existed: false,
        }
    }
}

impl AsyncRollbackableOperation for AsyncCopyDirectory {
    fn execute(&mut self) -> AsyncOpFuture<'_> {
        Box::pin(async move {
            if tokio::fs::metadata(&self.dest).await.is_ok() {
                self.dest_existed = true;
                tokio::fs::create_dir_all(&self.temp_dir).await?;
                self.backup_path = self.temp_dir.join(Uuid::new_v4().to_string());
                async_copy_dir(&self.dest, &self.backup_path).await?;
            }
            async_copy_dir(&self.source, &self.dest).await
        })
    }

    fn rollback(&mut self) -> AsyncOpFuture<'_> {
        Box::pin(async move {
            if self.dest_existed {
                tokio::fs::remove_dir_all(&self.dest).await?;
                async_copy_dir(&self.backup_path, &self.dest).await
            } else {
                match tokio::fs::remove_dir_all(&self.dest).await {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                    other => other,
                }
            }
        })
    }
}

impl Drop for AsyncCopyDirectory {
    fn drop(&mut self) {
        cleanup_backup_dir(&self.backup_path);
    }
}
