use std::fs;
use std::{
    io,
    path::{Path, PathBuf},
};

use crate::{copy_dir, RollbackableOperation};

/// Moves a file from source to destination. Type alias for [`MoveOperation`] for API consistency.
pub type MoveFile = MoveOperation;

/// Moves a directory from source to destination. Type alias for [`MoveOperation`] for API consistency.
pub type MoveDirectory = MoveOperation;

/// Move operation (works for both files and directories).
///
/// Attempts `fs::rename` first. On `ErrorKind::CrossesDevices`, automatically falls back to
/// copy-then-delete. Rollback uses the same strategy in reverse.
pub struct MoveOperation {
    source: PathBuf,
    dest: PathBuf,
    was_cross_device: bool,
}

impl MoveOperation {
    /// Constructs a new `MoveOperation`.
    pub fn new<S: AsRef<Path>, T: AsRef<Path>>(source: S, dest: T) -> Self {
        Self {
            source: source.as_ref().into(),
            dest: dest.as_ref().into(),
            was_cross_device: false,
        }
    }
}

impl RollbackableOperation for MoveOperation {
    fn execute(&mut self) -> io::Result<()> {
        match fs::rename(&self.source, &self.dest) {
            Ok(()) => {
                self.was_cross_device = false;
                Ok(())
            }
            Err(e) if e.kind() == io::ErrorKind::CrossesDevices => {
                self.was_cross_device = true;
                cross_device_move(&self.source, &self.dest)
            }
            Err(e) => Err(e),
        }
    }

    fn rollback(&mut self) -> io::Result<()> {
        let result = if self.was_cross_device {
            cross_device_move(&self.dest, &self.source)
        } else {
            fs::rename(&self.dest, &self.source)
        };
        match result {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            other => other,
        }
    }
}

fn cross_device_move(from: &Path, to: &Path) -> io::Result<()> {
    if from.is_dir() {
        copy_dir(from, to)?;
        fs::remove_dir_all(from)
    } else {
        fs::copy(from, to).map(|_| ())?;
        fs::remove_file(from)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn move_file_works() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("source.txt");
        let dest_dir = dir.path().join("out_dir");
        let dest = dest_dir.join("source.txt");
        fs::write(&src, b"data").unwrap();
        fs::create_dir(&dest_dir).unwrap();

        let mut op = MoveFile::new(&src, &dest);
        assert!(src.exists());
        assert!(!dest.exists());

        op.execute().unwrap();
        assert!(!src.exists());
        assert!(dest.exists());

        op.rollback().unwrap();
        assert!(src.exists());
        assert!(!dest.exists());
    }

    #[test]
    fn move_dir_works() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src_dir");
        let dest_parent = dir.path().join("dest_parent");
        let dest = dest_parent.join("src_dir");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("file.txt"), b"hello").unwrap();
        fs::create_dir_all(&dest_parent).unwrap();

        let mut op = MoveDirectory::new(&src, &dest);
        assert!(src.exists());
        assert!(!dest.exists());

        op.execute().unwrap();
        assert!(!src.exists());
        assert!(dest.exists());

        op.rollback().unwrap();
        assert!(src.exists());
        assert!(!dest.exists());
    }
}
