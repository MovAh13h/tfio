use std::future::Future;
use std::io::{self, Error};
use std::path::{Path, PathBuf};
use std::pin::Pin;

use uuid::Uuid;

mod append;
mod copy;
mod create;
mod delete;
mod move_;
mod touch;
mod write;

pub use append::AsyncAppendFile;
pub use copy::{AsyncCopyDirectory, AsyncCopyFile};
pub use create::{AsyncCreateDirectory, AsyncCreateFile};
pub use delete::{AsyncDeleteDirectory, AsyncDeleteFile};
pub use move_::{AsyncMoveDirectory, AsyncMoveFile, AsyncMoveOperation};
pub use touch::AsyncTouchFile;
pub use write::AsyncWriteFile;

/// Boxed future type used by [`AsyncRollbackableOperation`].
pub type AsyncOpFuture<'a> =
    Pin<Box<dyn Future<Output = io::Result<()>> + Send + 'a>>;

/// Trait that represents an asynchronous rollbackable operation.
pub trait AsyncRollbackableOperation: Send + Sync {
    /// Executes the operation.
    fn execute(&mut self) -> AsyncOpFuture<'_>;

    /// Rolls back the operation.
    ///
    /// Safe to call even if `execute` was never called or failed partway through.
    fn rollback(&mut self) -> AsyncOpFuture<'_>;
}

/// An asynchronous rollbackable transaction.
#[must_use = "build then call .execute().await"]
pub struct AsyncTransaction {
    ops: Vec<Box<dyn AsyncRollbackableOperation>>,
    execution_count: usize,
    temp_dir: PathBuf,
}

impl AsyncTransaction {
    /// Constructs a new, empty `AsyncTransaction` using the OS temporary directory for backups.
    pub fn new() -> Self {
        Self {
            ops: vec![],
            execution_count: 0,
            temp_dir: std::env::temp_dir(),
        }
    }

    /// Constructs a new, empty `AsyncTransaction` with a custom backup directory.
    pub fn with_temp_dir<P: AsRef<Path>>(temp_dir: P) -> Self {
        Self {
            ops: vec![],
            execution_count: 0,
            temp_dir: temp_dir.as_ref().to_path_buf(),
        }
    }

    /// Adds an [`AsyncCreateFile`] operation.
    pub fn create_file<S: AsRef<Path>>(mut self, path: S) -> Self {
        self.ops.push(Box::new(AsyncCreateFile::new(path)));
        self
    }

    /// Adds an [`AsyncCreateDirectory`] operation.
    pub fn create_dir<S: AsRef<Path>>(mut self, path: S) -> Self {
        self.ops.push(Box::new(AsyncCreateDirectory::new(path)));
        self
    }

    /// Adds an [`AsyncAppendFile`] operation.
    pub fn append_file<S: AsRef<Path>>(mut self, path: S, data: Vec<u8>) -> Self {
        self.ops
            .push(Box::new(AsyncAppendFile::with_temp_dir(path, self.temp_dir.clone(), data)));
        self
    }

    /// Adds an [`AsyncCopyFile`] operation.
    pub fn copy_file<S: AsRef<Path>, T: AsRef<Path>>(mut self, source: S, dest: T) -> Self {
        self.ops
            .push(Box::new(AsyncCopyFile::with_temp_dir(source, dest, self.temp_dir.clone())));
        self
    }

    /// Adds an [`AsyncCopyDirectory`] operation.
    pub fn copy_dir<S: AsRef<Path>, T: AsRef<Path>>(mut self, source: S, dest: T) -> Self {
        self.ops
            .push(Box::new(AsyncCopyDirectory::with_temp_dir(source, dest, self.temp_dir.clone())));
        self
    }

    /// Adds an [`AsyncDeleteFile`] operation.
    pub fn delete_file<S: AsRef<Path>>(mut self, source: S) -> Self {
        self.ops
            .push(Box::new(AsyncDeleteFile::with_temp_dir(source, self.temp_dir.clone())));
        self
    }

    /// Adds an [`AsyncDeleteDirectory`] operation.
    pub fn delete_dir<S: AsRef<Path>>(mut self, source: S) -> Self {
        self.ops
            .push(Box::new(AsyncDeleteDirectory::with_temp_dir(source, self.temp_dir.clone())));
        self
    }

    /// Adds an [`AsyncMoveFile`] operation.
    pub fn move_file<S: AsRef<Path>, T: AsRef<Path>>(mut self, source: S, dest: T) -> Self {
        self.ops.push(Box::new(AsyncMoveFile::new(source, dest)));
        self
    }

    /// Adds an [`AsyncMoveDirectory`] operation.
    pub fn move_dir<S: AsRef<Path>, T: AsRef<Path>>(mut self, source: S, dest: T) -> Self {
        self.ops.push(Box::new(AsyncMoveDirectory::new(source, dest)));
        self
    }

    /// Adds an [`AsyncWriteFile`] operation.
    pub fn write_file<S: AsRef<Path>>(mut self, path: S, data: Vec<u8>) -> Self {
        self.ops
            .push(Box::new(AsyncWriteFile::with_temp_dir(path, self.temp_dir.clone(), data)));
        self
    }

    /// Adds an [`AsyncTouchFile`] operation.
    pub fn touch_file<S: AsRef<Path>>(mut self, path: S) -> Self {
        self.ops.push(Box::new(AsyncTouchFile::new(path)));
        self
    }

    /// Executes all operations in order, stopping at the first error.
    pub async fn execute(&mut self) -> io::Result<()> {
        self.execution_count = 0;
        for i in 0..self.ops.len() {
            self.ops[i].execute().await?;
            self.execution_count += 1;
        }
        Ok(())
    }

    /// Rolls back all successfully-executed operations in reverse order, then resets the counter.
    pub async fn rollback(&mut self) -> io::Result<()> {
        for i in (0..self.execution_count).rev() {
            self.ops[i].rollback().await?;
        }
        self.execution_count = 0;
        Ok(())
    }
}

impl Default for AsyncTransaction {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Shared async helpers used by the submodules
// ---------------------------------------------------------------------------

pub(super) async fn async_copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    let mut stack = vec![from.to_path_buf()];
    let output_root = to.to_path_buf();
    let input_root = from.components().count();

    while let Some(working_path) = stack.pop() {
        let src: PathBuf = working_path.components().skip(input_root).collect();
        let dest = if src.components().count() == 0 {
            output_root.clone()
        } else {
            output_root.join(&src)
        };

        tokio::fs::create_dir_all(&dest).await?;

        let mut entries = tokio::fs::read_dir(&working_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let file_type = entry.file_type().await?;
            let path = entry.path();
            if file_type.is_dir() {
                stack.push(path);
            } else {
                match path.file_name() {
                    Some(filename) => {
                        tokio::fs::copy(&path, &dest.join(filename)).await?;
                    }
                    None => {
                        return Err(Error::other("could not extract filename from path"))
                    }
                }
            }
        }
    }
    Ok(())
}

pub(super) async fn create_async_backup_file(
    source: &Path,
    temp_dir: &Path,
) -> io::Result<PathBuf> {
    tokio::fs::create_dir_all(temp_dir).await?;
    let backup_path = temp_dir.join(Uuid::new_v4().to_string());
    tokio::fs::copy(source, &backup_path).await?;
    Ok(backup_path)
}

pub(super) async fn create_async_backup_folder(
    source: &Path,
    temp_dir: &Path,
) -> io::Result<PathBuf> {
    tokio::fs::create_dir_all(temp_dir).await?;
    let backup_path = temp_dir.join(Uuid::new_v4().to_string());
    async_copy_dir(source, &backup_path).await?;
    Ok(backup_path)
}

pub(super) fn cleanup_backup_file(path: &PathBuf) {
    if !path.as_os_str().is_empty() && path.exists() {
        if let Err(e) = std::fs::remove_file(path) {
            eprintln!("{}", e);
        }
    }
}

pub(super) fn cleanup_backup_dir(path: &PathBuf) {
    if !path.as_os_str().is_empty() && path.exists() {
        if let Err(e) = std::fs::remove_dir_all(path) {
            eprintln!("{}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[tokio::test]
    async fn async_transaction_works() {
        let dir = tempdir().unwrap();
        let d = dir.path();

        let mut tr = AsyncTransaction::with_temp_dir(d)
            .create_file(d.join("file.txt"))
            .create_file(d.join("for_delete.txt"))
            .create_dir(d.join("inner/sub"))
            .create_dir(d.join("for_delete_dir"))
            .create_dir(d.join("magic_dir"))
            .write_file(d.join("file.txt"), b"Hello World".to_vec())
            .append_file(d.join("file.txt"), b"Hello World".to_vec())
            .copy_file(d.join("file.txt"), d.join("inner/file.txt"))
            .copy_dir(d.join("magic_dir"), d.join("inner/magic_dir"))
            .delete_file(d.join("for_delete.txt"))
            .delete_dir(d.join("for_delete_dir"))
            .move_file(d.join("inner/file.txt"), d.join("inner/magic_dir/file.txt"))
            .create_dir(d.join("for_moving"))
            .move_dir(d.join("for_moving"), d.join("inner/magic_dir/for_moving"))
            .touch_file(d.join("touched.txt"));

        tr.execute().await.expect("Cannot execute");
        tr.rollback().await.expect("Cannot rollback");
    }
}
