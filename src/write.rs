use std::fs;
use std::{
    env,
    io,
    path::{Path, PathBuf},
};

use crate::{RollbackableOperation, SingleFileOperation};

/// Writes data to a file, replacing its existing contents.
pub struct WriteFile {
    path: PathBuf,
    temp_dir: PathBuf,
    backup_path: PathBuf,
    data: Vec<u8>,
}

impl WriteFile {
    /// Constructs a new `WriteFile` operation, using the OS temp directory for backups.
    pub fn new<S: AsRef<Path>>(path: S, data: Vec<u8>) -> Self {
        Self::with_temp_dir(path, env::temp_dir(), data)
    }

    /// Constructs a new `WriteFile` operation with a custom backup directory.
    pub fn with_temp_dir<S: AsRef<Path>, T: AsRef<Path>>(path: S, temp_dir: T, data: Vec<u8>) -> Self {
        Self {
            path: path.as_ref().into(),
            temp_dir: temp_dir.as_ref().into(),
            backup_path: PathBuf::new(),
            data,
        }
    }
}

impl RollbackableOperation for WriteFile {
    fn execute(&mut self) -> io::Result<()> {
        self.create_backup_file()?;
        fs::write(&self.path, &self.data)
    }

    fn rollback(&mut self) -> io::Result<()> {
        if self.backup_path.as_os_str().is_empty() {
            return Ok(());
        }
        fs::copy(&self.backup_path, &self.path).map(|_| ())
    }
}

impl SingleFileOperation for WriteFile {
    fn get_path(&self) -> &Path {
        &self.path
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

impl Drop for WriteFile {
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

    const INITIAL_DATA: &[u8] = b"Yellow World";
    const WRITTEN_DATA: &[u8] = b"Hello World";

    #[test]
    fn write_file_works() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("write_file.txt");
        fs::write(&file_path, INITIAL_DATA).unwrap();

        let mut op = WriteFile::with_temp_dir(&file_path, dir.path(), WRITTEN_DATA.to_vec());

        op.execute().expect("Unable to perform execute");
        assert_eq!(WRITTEN_DATA, fs::read(&file_path).unwrap().as_slice());

        op.rollback().expect("Unable to perform rollback");
        assert_eq!(INITIAL_DATA, fs::read(&file_path).unwrap().as_slice());
    }

    #[test]
    fn rollback_before_execute_is_noop() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("write_noop.txt");
        fs::write(&file_path, INITIAL_DATA).unwrap();

        let mut op = WriteFile::with_temp_dir(&file_path, dir.path(), WRITTEN_DATA.to_vec());
        op.rollback().expect("rollback before execute should be a no-op");
        assert_eq!(INITIAL_DATA, fs::read(&file_path).unwrap().as_slice());
    }
}
