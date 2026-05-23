use std::fs;
use std::{
    env,
    io,
    path::{Path, PathBuf},
};

use uuid::Uuid;

use crate::{copy_dir, RollbackableOperation};

/// Copies a file to the destination.
///
/// If the destination already exists its original content is backed up and
/// restored on rollback. If it did not exist, rollback removes the copy.
pub struct CopyFile {
    source: PathBuf,
    dest: PathBuf,
    temp_dir: PathBuf,
    backup_path: PathBuf,
    dest_existed: bool,
}

impl CopyFile {
    /// Constructs a new `CopyFile` operation, using the OS temp directory for backups.
    pub fn new<S: AsRef<Path>, T: AsRef<Path>>(source: S, dest: T) -> Self {
        Self::with_temp_dir(source, dest, env::temp_dir())
    }

    /// Constructs a new `CopyFile` operation with a custom backup directory.
    pub fn with_temp_dir<S: AsRef<Path>, T: AsRef<Path>, U: AsRef<Path>>(
        source: S,
        dest: T,
        temp_dir: U,
    ) -> Self {
        Self {
            source: source.as_ref().into(),
            dest: dest.as_ref().into(),
            temp_dir: temp_dir.as_ref().into(),
            backup_path: PathBuf::new(),
            dest_existed: false,
        }
    }
}

impl RollbackableOperation for CopyFile {
    fn execute(&mut self) -> io::Result<()> {
        if self.dest.exists() {
            self.dest_existed = true;
            fs::create_dir_all(&self.temp_dir)?;
            self.backup_path = self.temp_dir.join(Uuid::new_v4().to_string());
            fs::copy(&self.dest, &self.backup_path)?;
        }
        fs::copy(&self.source, &self.dest).map(|_| ())
    }

    fn rollback(&mut self) -> io::Result<()> {
        if self.dest_existed {
            fs::copy(&self.backup_path, &self.dest).map(|_| ())
        } else {
            match fs::remove_file(&self.dest) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                other => other,
            }
        }
    }
}

impl Drop for CopyFile {
    fn drop(&mut self) {
        let bp = &self.backup_path;
        if !bp.as_os_str().is_empty() && bp.exists() {
            if let Err(e) = fs::remove_file(bp) {
                eprintln!("{}", e);
            }
        }
    }
}

/// Copies a directory to the destination.
///
/// If the destination already exists its content is backed up and restored on
/// rollback. If it did not exist, rollback removes the copy.
pub struct CopyDirectory {
    source: PathBuf,
    dest: PathBuf,
    temp_dir: PathBuf,
    backup_path: PathBuf,
    dest_existed: bool,
}

impl CopyDirectory {
    /// Constructs a new `CopyDirectory` operation, using the OS temp directory for backups.
    pub fn new<S: AsRef<Path>, T: AsRef<Path>>(source: S, dest: T) -> Self {
        Self::with_temp_dir(source, dest, env::temp_dir())
    }

    /// Constructs a new `CopyDirectory` operation with a custom backup directory.
    pub fn with_temp_dir<S: AsRef<Path>, T: AsRef<Path>, U: AsRef<Path>>(
        source: S,
        dest: T,
        temp_dir: U,
    ) -> Self {
        Self {
            source: source.as_ref().into(),
            dest: dest.as_ref().into(),
            temp_dir: temp_dir.as_ref().into(),
            backup_path: PathBuf::new(),
            dest_existed: false,
        }
    }
}

impl RollbackableOperation for CopyDirectory {
    fn execute(&mut self) -> io::Result<()> {
        if self.dest.exists() {
            self.dest_existed = true;
            fs::create_dir_all(&self.temp_dir)?;
            self.backup_path = self.temp_dir.join(Uuid::new_v4().to_string());
            copy_dir(&self.dest, &self.backup_path)?;
        }
        copy_dir(&self.source, &self.dest)
    }

    fn rollback(&mut self) -> io::Result<()> {
        if self.dest_existed {
            fs::remove_dir_all(&self.dest)?;
            copy_dir(&self.backup_path, &self.dest)
        } else {
            match fs::remove_dir_all(&self.dest) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                other => other,
            }
        }
    }
}

impl Drop for CopyDirectory {
    fn drop(&mut self) {
        let bp = &self.backup_path;
        if !bp.as_os_str().is_empty() && bp.exists() {
            if let Err(e) = fs::remove_dir_all(bp) {
                eprintln!("{}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn copy_file_works() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("source.txt");
        let dest_dir = dir.path().join("dest_dir");
        let dest = dest_dir.join("source.txt");

        fs::write(&src, b"data").unwrap();
        fs::create_dir(&dest_dir).unwrap();

        let mut op = CopyFile::with_temp_dir(&src, &dest, dir.path());
        assert!(!dest.exists());
        op.execute().unwrap();
        assert!(src.exists());
        assert!(dest.exists());

        op.rollback().unwrap();
        assert!(src.exists());
        assert!(!dest.exists());
    }

    #[test]
    fn copy_file_restores_overwritten_dest() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");

        fs::write(&src, b"new").unwrap();
        fs::write(&dest, b"original").unwrap();

        let mut op = CopyFile::with_temp_dir(&src, &dest, dir.path());
        op.execute().unwrap();
        assert_eq!(b"new", fs::read(&dest).unwrap().as_slice());

        op.rollback().unwrap();
        assert_eq!(b"original", fs::read(&dest).unwrap().as_slice());
    }

    #[test]
    fn copy_dir_works() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src_dir");
        let dest = dir.path().join("dest_dir");

        fs::create_dir(&src).unwrap();
        fs::write(src.join("file.txt"), b"hello").unwrap();

        let mut op = CopyDirectory::with_temp_dir(&src, &dest, dir.path());
        op.execute().unwrap();
        assert!(src.exists());
        assert!(dest.exists());
        assert!(dest.join("file.txt").exists());

        op.rollback().unwrap();
        assert!(src.exists());
        assert!(!dest.exists());
    }

    #[test]
    fn copy_dir_restores_overwritten_dest() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src_dir");
        let dest = dir.path().join("dest_dir");

        fs::create_dir(&src).unwrap();
        fs::write(src.join("new.txt"), b"new").unwrap();
        fs::create_dir(&dest).unwrap();
        fs::write(dest.join("original.txt"), b"original").unwrap();

        let mut op = CopyDirectory::with_temp_dir(&src, &dest, dir.path());
        op.execute().unwrap();
        assert!(dest.join("new.txt").exists());

        op.rollback().unwrap();
        assert!(dest.join("original.txt").exists());
        assert!(!dest.join("new.txt").exists());
    }
}
