//! TFIO is a library that provides a Transaction-like interface traditionally used in databases,
//! applied to FileIO operations. It gives the flexibility to execute and rollback singular
//! operations as well as transactions on the fly. The library also provides a builder-pattern
//! interface to chain operations and execute them in one go.
//!
//! # Examples
//!
//! Create a transaction and execute it. If any `Error` is encountered, rollback the entire transaction:
//! ```no_run
//! use std::io;
//! use tfio::*;
//!
//! fn main() -> io::Result<()> {
//!     let mut tr = Transaction::new()
//!         .create_file("./foo.txt")
//!         .create_dir("./bar")
//!         .write_file("./foo.txt", b"Hello World".to_vec())
//!         .move_file("./foo.txt", "./bar/foo.txt")
//!         .append_file("./bar/foo.txt", b"dlroW olleH".to_vec());
//!
//!     // Execute the transaction
//!     if let Err(e) = tr.execute() {
//!         eprintln!("Error during execution: {}", e);
//!
//!         // All operations can be reverted in reverse-order if `Error` is encountered
//!         if let Err(ee) = tr.rollback() {
//!             panic!("Error during transaction rollback: {}", ee);
//!         }
//!     }
//!     Ok(())
//! }
//! ```
//!
//! You can also import single operations to use:
//! ```no_run
//! use std::io;
//! use std::fs;
//! use tfio::{CopyFile, RollbackableOperation};
//!
//! fn main() -> io::Result<()> {
//!     fs::File::create("./foo.txt")?;
//!     fs::create_dir_all("./bar/baz")?;
//!
//!     let mut copy_operation = CopyFile::new("./foo.txt", "./bar/baz/foo.txt");
//!
//!     // Execute the operation
//!     if let Err(e) = copy_operation.execute() {
//!         eprintln!("Error during execution: {}", e);
//!
//!         // Rollback the operation
//!         if let Err(ee) = copy_operation.rollback() {
//!             panic!("Error during rollback: {}", ee);
//!         }
//!     }
//!     Ok(())
//! }
//! ```

#![deny(missing_docs)]

mod append;
mod copy;
mod create;
mod delete;
mod r#move;
mod touch;
mod write;

/// Async versions of all operations, backed by `tokio::fs`. Enable with the `tokio` feature.
#[cfg(feature = "tokio")]
pub mod async_;

#[cfg(feature = "tokio")]
pub use async_::{
    AsyncAppendFile, AsyncCopyDirectory, AsyncCopyFile, AsyncCreateDirectory, AsyncCreateFile,
    AsyncDeleteDirectory, AsyncDeleteFile, AsyncMoveDirectory, AsyncMoveFile, AsyncMoveOperation,
    AsyncOpFuture, AsyncRollbackableOperation, AsyncTouchFile, AsyncTransaction, AsyncWriteFile,
};

use std::env;
use std::fs;
use std::io::{self, Error};
use std::path::{Path, PathBuf};

use uuid::Uuid;

pub use append::AppendFile;
pub use copy::{CopyDirectory, CopyFile};
pub use create::{CreateDirectory, CreateFile};
pub use delete::{DeleteDirectory, DeleteFile};
pub use r#move::{MoveDirectory, MoveFile, MoveOperation};
pub use touch::TouchFile;
pub use write::WriteFile;

/// Trait that represents a rollbackable operation.
pub trait RollbackableOperation {
    /// Executes the operation.
    fn execute(&mut self) -> io::Result<()>;

    /// Rolls back the operation.
    ///
    /// Safe to call even if `execute` was never called or failed partway through.
    fn rollback(&mut self) -> io::Result<()>;
}

/// Trait that represents a directory operation.
pub trait DirectoryOperation: RollbackableOperation {
    /// Returns the path to the source directory.
    fn get_path(&self) -> &Path;

    /// Returns the path to the backup directory. Empty if no backup has been created.
    fn get_backup_path(&self) -> &Path;

    /// Sets the backup path.
    fn set_backup_path<S: AsRef<Path>>(&mut self, path: S);

    /// Returns the path to the temp dir.
    fn get_temp_dir(&self) -> &Path;

    /// Cleans up the backup directory. Should be called from [`Drop`].
    fn dispose(&self) -> io::Result<()> {
        let bp = self.get_backup_path();
        if bp.as_os_str().is_empty() || !bp.exists() {
            return Ok(());
        }
        fs::remove_dir_all(bp)
    }

    /// Creates a backup copy of the source directory and records the backup path.
    fn create_backup_folder(&mut self) -> io::Result<()> {
        fs::create_dir_all(self.get_temp_dir())?;
        let backup_path = self.get_temp_dir().join(Uuid::new_v4().to_string());
        copy_dir(self.get_path(), &backup_path)?;
        self.set_backup_path(backup_path);
        Ok(())
    }
}

/// Trait that represents a single-file operation.
pub trait SingleFileOperation: RollbackableOperation {
    /// Returns the path to the source file.
    fn get_path(&self) -> &Path;

    /// Returns the path to the backup file. Empty if no backup has been created.
    fn get_backup_path(&self) -> &Path;

    /// Sets the backup path.
    fn set_backup_path<S: AsRef<Path>>(&mut self, path: S);

    /// Returns the path to the temp dir.
    fn get_temp_dir(&self) -> &Path;

    /// Cleans up the backup file. Should be called from [`Drop`].
    fn dispose(&self) -> io::Result<()> {
        let bp = self.get_backup_path();
        if bp.as_os_str().is_empty() || !bp.exists() {
            return Ok(());
        }
        fs::remove_file(bp)
    }

    /// Creates a backup copy of the source file and records the backup path.
    fn create_backup_file(&mut self) -> io::Result<()> {
        fs::create_dir_all(self.get_temp_dir())?;
        let backup_path = self.get_temp_dir().join(Uuid::new_v4().to_string());
        fs::copy(self.get_path(), &backup_path)?;
        self.set_backup_path(&backup_path);
        Ok(())
    }
}

pub(crate) fn copy_dir<U: AsRef<Path>, V: AsRef<Path>>(from: U, to: V) -> io::Result<()> {
    let mut stack = vec![from.as_ref().to_path_buf()];
    let output_root = to.as_ref().to_path_buf();
    let input_root = from.as_ref().components().count();

    while let Some(working_path) = stack.pop() {
        let src: PathBuf = working_path.components().skip(input_root).collect();
        let dest = if src.components().count() == 0 {
            output_root.clone()
        } else {
            output_root.join(&src)
        };

        fs::create_dir_all(&dest)?;

        for entry in fs::read_dir(&working_path)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                stack.push(path);
            } else {
                match path.file_name() {
                    Some(filename) => fs::copy(&path, dest.join(filename)).map(|_| ())?,
                    None => {
                        return Err(Error::other("could not extract filename from path"))
                    }
                }
            }
        }
    }

    Ok(())
}

/// A rollbackable transaction that executes a sequence of [`RollbackableOperation`]s.
#[must_use = "build then call .execute()"]
pub struct Transaction {
    ops: Vec<Box<dyn RollbackableOperation>>,
    execution_count: usize,
    temp_dir: PathBuf,
}

impl Transaction {
    /// Constructs a new, empty `Transaction` using the OS temporary directory for backups.
    pub fn new() -> Self {
        Self {
            ops: vec![],
            execution_count: 0,
            temp_dir: env::temp_dir(),
        }
    }

    /// Constructs a new, empty `Transaction` with a custom backup directory.
    pub fn with_temp_dir<P: AsRef<Path>>(temp_dir: P) -> Self {
        Self {
            ops: vec![],
            execution_count: 0,
            temp_dir: temp_dir.as_ref().to_path_buf(),
        }
    }

    /// Adds a [`CreateFile`] operation.
    pub fn create_file<S: AsRef<Path>>(mut self, path: S) -> Self {
        self.ops.push(Box::new(CreateFile::new(path)));
        self
    }

    /// Adds a [`CreateDirectory`] operation.
    pub fn create_dir<S: AsRef<Path>>(mut self, path: S) -> Self {
        self.ops.push(Box::new(CreateDirectory::new(path)));
        self
    }

    /// Adds an [`AppendFile`] operation.
    pub fn append_file<S: AsRef<Path>>(mut self, path: S, data: Vec<u8>) -> Self {
        self.ops
            .push(Box::new(AppendFile::with_temp_dir(path, self.temp_dir.clone(), data)));
        self
    }

    /// Adds a [`CopyFile`] operation.
    pub fn copy_file<S: AsRef<Path>, T: AsRef<Path>>(mut self, source: S, dest: T) -> Self {
        self.ops
            .push(Box::new(CopyFile::with_temp_dir(source, dest, self.temp_dir.clone())));
        self
    }

    /// Adds a [`CopyDirectory`] operation.
    pub fn copy_dir<S: AsRef<Path>, T: AsRef<Path>>(mut self, source: S, dest: T) -> Self {
        self.ops
            .push(Box::new(CopyDirectory::with_temp_dir(source, dest, self.temp_dir.clone())));
        self
    }

    /// Adds a [`DeleteFile`] operation.
    pub fn delete_file<S: AsRef<Path>>(mut self, source: S) -> Self {
        self.ops
            .push(Box::new(DeleteFile::with_temp_dir(source, self.temp_dir.clone())));
        self
    }

    /// Adds a [`DeleteDirectory`] operation.
    pub fn delete_dir<S: AsRef<Path>>(mut self, source: S) -> Self {
        self.ops
            .push(Box::new(DeleteDirectory::with_temp_dir(source, self.temp_dir.clone())));
        self
    }

    /// Adds a [`MoveFile`] operation.
    pub fn move_file<S: AsRef<Path>, T: AsRef<Path>>(mut self, source: S, dest: T) -> Self {
        self.ops.push(Box::new(MoveFile::new(source, dest)));
        self
    }

    /// Adds a [`MoveDirectory`] operation.
    pub fn move_dir<S: AsRef<Path>, T: AsRef<Path>>(mut self, source: S, dest: T) -> Self {
        self.ops.push(Box::new(MoveDirectory::new(source, dest)));
        self
    }

    /// Adds a [`WriteFile`] operation.
    pub fn write_file<S: AsRef<Path>>(mut self, path: S, data: Vec<u8>) -> Self {
        self.ops
            .push(Box::new(WriteFile::with_temp_dir(path, self.temp_dir.clone(), data)));
        self
    }

    /// Adds a [`TouchFile`] operation.
    pub fn touch_file<S: AsRef<Path>>(mut self, path: S) -> Self {
        self.ops.push(Box::new(TouchFile::new(path)));
        self
    }
}

impl Default for Transaction {
    fn default() -> Self {
        Self::new()
    }
}

impl RollbackableOperation for Transaction {
    /// Executes all operations in order, stopping at the first error.
    fn execute(&mut self) -> io::Result<()> {
        self.execution_count = 0;
        for i in 0..self.ops.len() {
            self.ops[i].execute()?;
            self.execution_count += 1;
        }
        Ok(())
    }

    /// Rolls back all executed operations in reverse order, then resets the counter.
    ///
    /// Only the operations that completed successfully are rolled back.
    fn rollback(&mut self) -> io::Result<()> {
        for i in (0..self.execution_count).rev() {
            self.ops[i].rollback()?;
        }
        self.execution_count = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn transaction_works() {
        let dir = tempdir().unwrap();
        let d = dir.path();

        let mut tr = Transaction::with_temp_dir(d)
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
            .move_dir(d.join("for_moving"), d.join("inner/magic_dir/for_moving"));

        tr.execute().expect("Cannot execute");
        tr.rollback().expect("Cannot rollback");
    }
}
