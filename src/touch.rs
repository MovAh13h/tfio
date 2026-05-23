use std::fs::{File, FileTimes, OpenOptions};
use std::time::SystemTime;
use std::{
    io,
    path::{Path, PathBuf},
};

use crate::RollbackableOperation;

enum TouchState {
    NotExecuted,
    /// File did not exist before execute(); rollback deletes it.
    Created,
    /// File existed; rollback restores its original access + modification times.
    Touched {
        original_accessed: SystemTime,
        original_modified: SystemTime,
    },
}

/// Creates a file if it does not exist, or updates its access and modification times to now
pub struct TouchFile {
    path: PathBuf,
    state: TouchState,
}

impl TouchFile {
    /// Constructs a new TouchFile operation
    pub fn new<S: AsRef<Path>>(path: S) -> Self {
        Self {
            path: path.as_ref().into(),
            state: TouchState::NotExecuted,
        }
    }
}

impl RollbackableOperation for TouchFile {
    fn execute(&mut self) -> io::Result<()> {
        if self.path.exists() {
            let meta = std::fs::metadata(&self.path)?;
            let original_accessed = meta.accessed()?;
            let original_modified = meta.modified()?;
            self.state = TouchState::Touched {
                original_accessed,
                original_modified,
            };
            OpenOptions::new()
                .write(true)
                .open(&self.path)?
                .set_times(FileTimes::new().set_accessed(SystemTime::now()).set_modified(SystemTime::now()))
        } else {
            File::create(&self.path)?;
            self.state = TouchState::Created;
            Ok(())
        }
    }

    fn rollback(&mut self) -> io::Result<()> {
        match &self.state {
            TouchState::NotExecuted => Ok(()),
            TouchState::Created => std::fs::remove_file(&self.path),
            TouchState::Touched {
                original_accessed,
                original_modified,
            } => OpenOptions::new()
                .write(true)
                .open(&self.path)?
                .set_times(
                    FileTimes::new()
                        .set_accessed(*original_accessed)
                        .set_modified(*original_modified),
                ),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::Duration;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn touch_creates_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("new_file.txt");

        assert!(!path.exists());
        let mut op = TouchFile::new(&path);
        op.execute().unwrap();
        assert!(path.exists());

        op.rollback().unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn touch_updates_mtime_and_restores() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("existing.txt");
        fs::write(&path, b"data").unwrap();

        // Set a known old mtime (1 second in the past)
        let past = SystemTime::now() - Duration::from_secs(10);
        OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_times(FileTimes::new().set_accessed(past).set_modified(past))
            .unwrap();

        let original_mtime = fs::metadata(&path).unwrap().modified().unwrap();

        let mut op = TouchFile::new(&path);
        op.execute().unwrap();

        let new_mtime = fs::metadata(&path).unwrap().modified().unwrap();
        assert!(new_mtime > original_mtime);

        op.rollback().unwrap();
        let restored_mtime = fs::metadata(&path).unwrap().modified().unwrap();
        assert_eq!(original_mtime, restored_mtime);
    }
}
