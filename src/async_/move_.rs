use std::io;
use std::path::{Path, PathBuf};

use super::{async_copy_dir, AsyncOpFuture, AsyncRollbackableOperation};

/// Asynchronously moves a file. Type alias for [`AsyncMoveOperation`] for API consistency.
pub type AsyncMoveFile = AsyncMoveOperation;

/// Asynchronously moves a directory. Type alias for [`AsyncMoveOperation`] for API consistency.
pub type AsyncMoveDirectory = AsyncMoveOperation;

/// Asynchronous move operation (works for both files and directories).
///
/// Attempts `tokio::fs::rename` first. On `ErrorKind::CrossesDevices`, falls back to
/// copy-then-delete transparently. Rollback uses the same strategy in reverse.
pub struct AsyncMoveOperation {
    source: PathBuf,
    dest: PathBuf,
    was_cross_device: bool,
}

impl AsyncMoveOperation {
    /// Constructs a new `AsyncMoveOperation`.
    pub fn new<S: AsRef<Path>, T: AsRef<Path>>(source: S, dest: T) -> Self {
        Self {
            source: source.as_ref().into(),
            dest: dest.as_ref().into(),
            was_cross_device: false,
        }
    }
}

impl AsyncRollbackableOperation for AsyncMoveOperation {
    fn execute(&mut self) -> AsyncOpFuture<'_> {
        Box::pin(async move {
            match tokio::fs::rename(&self.source, &self.dest).await {
                Ok(()) => {
                    self.was_cross_device = false;
                    Ok(())
                }
                Err(e) if e.kind() == io::ErrorKind::CrossesDevices => {
                    self.was_cross_device = true;
                    async_cross_device_move(&self.source, &self.dest).await
                }
                Err(e) => Err(e),
            }
        })
    }

    fn rollback(&mut self) -> AsyncOpFuture<'_> {
        Box::pin(async move {
            let result = if self.was_cross_device {
                async_cross_device_move(&self.dest, &self.source).await
            } else {
                tokio::fs::rename(&self.dest, &self.source).await
            };
            match result {
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                other => other,
            }
        })
    }
}

async fn async_cross_device_move(from: &Path, to: &Path) -> io::Result<()> {
    let meta = tokio::fs::metadata(from).await?;
    if meta.is_dir() {
        async_copy_dir(from, to).await?;
        tokio::fs::remove_dir_all(from).await
    } else {
        tokio::fs::copy(from, to).await.map(|_| ())?;
        tokio::fs::remove_file(from).await
    }
}
