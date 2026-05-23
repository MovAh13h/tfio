use std::io;
use std::path::{Path, PathBuf};

use super::{AsyncOpFuture, AsyncRollbackableOperation};

/// Asynchronously creates a new file. Fails if the file already exists.
pub struct AsyncCreateFile {
    path: PathBuf,
}

impl AsyncCreateFile {
    /// Constructs a new `AsyncCreateFile` operation.
    pub fn new<S: AsRef<Path>>(path: S) -> Self {
        Self {
            path: path.as_ref().into(),
        }
    }
}

impl AsyncRollbackableOperation for AsyncCreateFile {
    fn execute(&mut self) -> AsyncOpFuture<'_> {
        Box::pin(async move {
            tokio::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&self.path)
                .await
                .map(|_| ())
        })
    }

    fn rollback(&mut self) -> AsyncOpFuture<'_> {
        Box::pin(async move {
            match tokio::fs::remove_file(&self.path).await {
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                other => other,
            }
        })
    }
}

/// Asynchronously creates a new directory (and any missing parent directories).
pub struct AsyncCreateDirectory {
    path: PathBuf,
    created_root: Option<PathBuf>,
}

impl AsyncCreateDirectory {
    /// Constructs a new `AsyncCreateDirectory` operation.
    pub fn new<S: AsRef<Path>>(path: S) -> Self {
        Self {
            path: path.as_ref().into(),
            created_root: None,
        }
    }
}

impl AsyncRollbackableOperation for AsyncCreateDirectory {
    fn execute(&mut self) -> AsyncOpFuture<'_> {
        Box::pin(async move {
            // Walk up to find the topmost new component (sync metadata is fine here).
            let mut to_create: Option<&Path> = None;
            let mut cursor: &Path = &self.path;
            loop {
                if cursor.exists() {
                    break;
                }
                to_create = Some(cursor);
                cursor = match cursor.parent() {
                    Some(p) => p,
                    None => break,
                };
            }
            self.created_root = to_create.map(|p| p.to_path_buf());
            tokio::fs::create_dir_all(&self.path).await
        })
    }

    fn rollback(&mut self) -> AsyncOpFuture<'_> {
        Box::pin(async move {
            match &self.created_root {
                Some(root) => match tokio::fs::remove_dir_all(root).await {
                    Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                    other => other,
                },
                None => Ok(()),
            }
        })
    }
}
