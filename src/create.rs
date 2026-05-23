use std::fs;
use std::{
    io,
    path::{Path, PathBuf},
};

use crate::RollbackableOperation;

/// Creates a new file. Fails if the file already exists.
pub struct CreateFile {
    path: PathBuf,
}

impl CreateFile {
    /// Constructs a new `CreateFile` operation.
    pub fn new<S: AsRef<Path>>(path: S) -> Self {
        Self {
            path: path.as_ref().into(),
        }
    }
}

impl RollbackableOperation for CreateFile {
    fn execute(&mut self) -> io::Result<()> {
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&self.path)
            .map(|_| ())
    }

    fn rollback(&mut self) -> io::Result<()> {
        match fs::remove_file(&self.path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            other => other,
        }
    }
}

/// Creates a new directory (and any missing parent directories).
pub struct CreateDirectory {
    path: PathBuf,
    created_root: Option<PathBuf>,
}

impl CreateDirectory {
    /// Constructs a new `CreateDirectory` operation.
    pub fn new<S: AsRef<Path>>(path: S) -> Self {
        Self {
            path: path.as_ref().into(),
            created_root: None,
        }
    }
}

impl RollbackableOperation for CreateDirectory {
    fn execute(&mut self) -> io::Result<()> {
        // Find the topmost path component that doesn't already exist so rollback
        // removes only what this operation created, not pre-existing parent dirs.
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
        fs::create_dir_all(&self.path)
    }

    fn rollback(&mut self) -> io::Result<()> {
        match &self.created_root {
            Some(root) => match fs::remove_dir_all(root) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                other => other,
            },
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn create_file_works() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("create_file.txt");

        let mut op = CreateFile::new(&file_path);
        assert!(!file_path.exists());
        op.execute().unwrap();
        assert!(file_path.exists());
        op.rollback().unwrap();
        assert!(!file_path.exists());
    }

    #[test]
    fn create_file_fails_if_exists() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("existing.txt");
        std::fs::write(&file_path, b"data").unwrap();

        let mut op = CreateFile::new(&file_path);
        assert!(op.execute().is_err());
    }

    #[test]
    fn create_dir_works() {
        let dir = tempdir().unwrap();
        let new_dir = dir.path().join("create_dir");

        let mut op = CreateDirectory::new(&new_dir);
        assert!(!new_dir.exists());
        op.execute().unwrap();
        assert!(new_dir.exists());
        op.rollback().unwrap();
        assert!(!new_dir.exists());
    }

    #[test]
    fn create_dir_nested_rollback_only_removes_created() {
        let dir = tempdir().unwrap();
        let parent = dir.path().join("existing_parent");
        let nested = parent.join("new_leaf");

        std::fs::create_dir_all(&parent).unwrap();

        let mut op = CreateDirectory::new(&nested);
        op.execute().unwrap();
        assert!(nested.exists());
        assert!(parent.exists());

        op.rollback().unwrap();
        assert!(!nested.exists());
        assert!(parent.exists());
    }
}
