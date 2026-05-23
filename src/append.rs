use std::fs;
use std::{
    env,
    io,
    path::{Path, PathBuf},
};

use crate::{RollbackableOperation, SingleFileOperation};

/// Appends data to a file.
pub struct AppendFile {
    path: PathBuf,
    temp_dir: PathBuf,
    backup_path: PathBuf,
    data: Vec<u8>,
}

impl AppendFile {
    /// Constructs a new `AppendFile` operation, using the OS temp directory for backups.
    pub fn new<S: AsRef<Path>>(path: S, data: Vec<u8>) -> Self {
        Self::with_temp_dir(path, env::temp_dir(), data)
    }

    /// Constructs a new `AppendFile` operation with a custom backup directory.
    pub fn with_temp_dir<S: AsRef<Path>, T: AsRef<Path>>(path: S, temp_dir: T, data: Vec<u8>) -> Self {
        Self {
            path: path.as_ref().into(),
            temp_dir: temp_dir.as_ref().into(),
            backup_path: PathBuf::new(),
            data,
        }
    }
}

impl RollbackableOperation for AppendFile {
    fn execute(&mut self) -> io::Result<()> {
        self.create_backup_file()?;
        let mut file = fs::OpenOptions::new().append(true).open(&self.path)?;
        use std::io::Write;
        file.write_all(&self.data)
    }

    fn rollback(&mut self) -> io::Result<()> {
        if self.backup_path.as_os_str().is_empty() {
            return Ok(());
        }
        fs::copy(&self.backup_path, &self.path).map(|_| ())
    }
}

impl SingleFileOperation for AppendFile {
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

impl Drop for AppendFile {
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

    const DATA: &[u8] = b"Hello World";

    #[test]
    fn append_file_works() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("append.txt");
        fs::write(&file_path, DATA).unwrap();

        let mut op = AppendFile::with_temp_dir(&file_path, dir.path(), DATA.to_vec());
        op.execute().expect("Unable to perform execute");

        let data = fs::read(&file_path).expect("Unable to read file");
        assert_eq!([DATA, DATA].concat(), data);

        op.rollback().expect("Unable to perform rollback");
        let data = fs::read(&file_path).expect("Unable to read file");
        assert_eq!(DATA, data.as_slice());
    }

    #[test]
    fn rollback_before_execute_is_noop() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("append_noop.txt");
        fs::write(&file_path, DATA).unwrap();

        let mut op = AppendFile::with_temp_dir(&file_path, dir.path(), DATA.to_vec());
        op.rollback().expect("rollback before execute should be a no-op");
        assert_eq!(DATA, fs::read(&file_path).unwrap().as_slice());
    }
}
