use std::fs;
use std::{
    env,
    io,
    path::{Path, PathBuf},
};

use crate::{copy_dir, DirectoryOperation, RollbackableOperation, SingleFileOperation};

/// Deletes a file (backed up so it can be restored on rollback).
pub struct DeleteFile {
    source: PathBuf,
    temp_dir: PathBuf,
    backup_path: PathBuf,
}

impl DeleteFile {
    /// Constructs a new `DeleteFile` operation, using the OS temp directory for backups.
    pub fn new<S: AsRef<Path>>(source: S) -> Self {
        Self::with_temp_dir(source, env::temp_dir())
    }

    /// Constructs a new `DeleteFile` operation with a custom backup directory.
    pub fn with_temp_dir<S: AsRef<Path>, T: AsRef<Path>>(source: S, temp_dir: T) -> Self {
        Self {
            source: source.as_ref().into(),
            temp_dir: temp_dir.as_ref().into(),
            backup_path: PathBuf::new(),
        }
    }
}

impl RollbackableOperation for DeleteFile {
    fn execute(&mut self) -> io::Result<()> {
        self.create_backup_file()?;
        fs::remove_file(self.get_path())
    }

    fn rollback(&mut self) -> io::Result<()> {
        if self.backup_path.as_os_str().is_empty() {
            return Ok(());
        }
        fs::copy(self.get_backup_path(), self.get_path()).map(|_| ())
    }
}

impl SingleFileOperation for DeleteFile {
    fn get_path(&self) -> &Path {
        &self.source
    }

    fn get_backup_path(&self) -> &Path {
        &self.backup_path
    }

    fn set_backup_path<S: AsRef<Path>>(&mut self, path: S) {
        self.backup_path = path.as_ref().into();
    }

    fn get_temp_dir(&self) -> &Path {
        &self.temp_dir
    }
}

impl Drop for DeleteFile {
    fn drop(&mut self) {
        if let Err(e) = self.dispose() {
            eprintln!("{}", e);
        }
    }
}

/// Deletes a directory (backed up so it can be restored on rollback).
pub struct DeleteDirectory {
    source: PathBuf,
    backup_path: PathBuf,
    temp_dir: PathBuf,
}

impl DeleteDirectory {
    /// Constructs a new `DeleteDirectory` operation, using the OS temp directory for backups.
    pub fn new<S: AsRef<Path>>(source: S) -> Self {
        Self::with_temp_dir(source, env::temp_dir())
    }

    /// Constructs a new `DeleteDirectory` operation with a custom backup directory.
    pub fn with_temp_dir<S: AsRef<Path>, T: AsRef<Path>>(source: S, temp_dir: T) -> Self {
        Self {
            source: source.as_ref().into(),
            temp_dir: temp_dir.as_ref().into(),
            backup_path: PathBuf::new(),
        }
    }
}

impl RollbackableOperation for DeleteDirectory {
    fn execute(&mut self) -> io::Result<()> {
        self.create_backup_folder()?;
        fs::remove_dir_all(&self.source)
    }

    fn rollback(&mut self) -> io::Result<()> {
        if self.backup_path.as_os_str().is_empty() {
            return Ok(());
        }
        match fs::rename(self.get_backup_path(), &self.source) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::CrossesDevices => {
                copy_dir(self.get_backup_path(), &self.source)?;
                fs::remove_dir_all(self.get_backup_path())
            }
            Err(e) => Err(e),
        }
    }
}

impl DirectoryOperation for DeleteDirectory {
    fn get_path(&self) -> &Path {
        &self.source
    }

    fn get_backup_path(&self) -> &Path {
        &self.backup_path
    }

    fn set_backup_path<S: AsRef<Path>>(&mut self, path: S) {
        self.backup_path = path.as_ref().into();
    }

    fn get_temp_dir(&self) -> &Path {
        &self.temp_dir
    }
}

impl Drop for DeleteDirectory {
    fn drop(&mut self) {
        if let Err(e) = self.dispose() {
            eprintln!("{}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn delete_file_works() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("delete_me.txt");
        fs::write(&file_path, b"data").unwrap();

        let mut op = DeleteFile::with_temp_dir(&file_path, dir.path());
        assert!(file_path.exists());
        op.execute().unwrap();
        assert!(!file_path.exists());
        op.rollback().unwrap();
        assert!(file_path.exists());
    }

    #[test]
    fn delete_dir_works() {
        let dir = tempdir().unwrap();
        let target_dir = dir.path().join("delete_me_dir");
        fs::create_dir(&target_dir).unwrap();

        let mut op = DeleteDirectory::with_temp_dir(&target_dir, dir.path());
        assert!(target_dir.exists());
        op.execute().unwrap();
        assert!(!target_dir.exists());
        op.rollback().unwrap();
        assert!(target_dir.exists());
    }
}
