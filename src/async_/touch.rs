use std::fs::{FileTimes, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::{AsyncOpFuture, AsyncRollbackableOperation};

enum TouchState {
    NotExecuted,
    Created,
    Touched {
        original_accessed: SystemTime,
        original_modified: SystemTime,
    },
}

/// Asynchronously creates a file if absent, or updates its access and modification times to now.
pub struct AsyncTouchFile {
    path: PathBuf,
    state: TouchState,
}

impl AsyncTouchFile {
    /// Constructs a new `AsyncTouchFile` operation.
    pub fn new<S: AsRef<Path>>(path: S) -> Self {
        Self {
            path: path.as_ref().into(),
            state: TouchState::NotExecuted,
        }
    }
}

impl AsyncRollbackableOperation for AsyncTouchFile {
    fn execute(&mut self) -> AsyncOpFuture<'_> {
        Box::pin(async move {
            match tokio::fs::metadata(&self.path).await {
                Ok(meta) => {
                    let original_accessed = meta.accessed()?;
                    let original_modified = meta.modified()?;
                    self.state = TouchState::Touched {
                        original_accessed,
                        original_modified,
                    };
                    let path = self.path.clone();
                    tokio::task::spawn_blocking(move || {
                        OpenOptions::new().write(true).open(&path)?.set_times(
                            FileTimes::new()
                                .set_accessed(SystemTime::now())
                                .set_modified(SystemTime::now()),
                        )
                    })
                    .await
                    .map_err(io::Error::other)?
                }
                Err(_) => {
                    tokio::fs::File::create(&self.path).await?;
                    self.state = TouchState::Created;
                    Ok(())
                }
            }
        })
    }

    fn rollback(&mut self) -> AsyncOpFuture<'_> {
        Box::pin(async move {
            match &self.state {
                TouchState::NotExecuted => Ok(()),
                TouchState::Created => tokio::fs::remove_file(&self.path).await,
                TouchState::Touched {
                    original_accessed,
                    original_modified,
                } => {
                    let path = self.path.clone();
                    let accessed = *original_accessed;
                    let modified = *original_modified;
                    tokio::task::spawn_blocking(move || {
                        OpenOptions::new().write(true).open(&path)?.set_times(
                            FileTimes::new()
                                .set_accessed(accessed)
                                .set_modified(modified),
                        )
                    })
                    .await
                    .map_err(io::Error::other)?
                }
            }
        })
    }
}
