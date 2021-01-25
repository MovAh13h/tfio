use std::fs;
use std::{
    io,
    path::{Path, PathBuf},
};

use crate::RollbackableOperation;

/// Moves a file from source to destination. A type alias for [MoveOperation](MoveOperation) for consistency in the API
pub type MoveFile = MoveOperation;

/// Moves a directory from source to destination. A type alias for [MoveOperation](MoveOperation) for consistency in the API
pub type MoveDirectory = MoveOperation;

/// Move operation
///
/// This is a type-independent operation ie. it works with both files and directories since [std::fs::rename](std::fs::rename) is also independent
pub struct MoveOperation {
    source: PathBuf,
    dest: PathBuf,
}

impl MoveOperation {
    /// Constructs a new MoveOperation operation
    ///
    /// This operation is directly called by [MoveFile](MoveFile) and [MoveDirectory](MoveDirectory) and hence only available as a single operation
    pub fn new<S: AsRef<Path>, T: AsRef<Path>>(source: S, dest: T) -> Self {
        Self {
            source: source.as_ref().into(),
            dest: dest.as_ref().into(),
        }
    }
}

impl RollbackableOperation for MoveOperation {
    fn execute(&mut self) -> io::Result<()> {
        fs::rename(&self.source, &self.dest)
    }

    fn rollback(&self) -> io::Result<()> {
        fs::rename(&self.dest, &self.source)
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::path::Path;

    use super::*;

    const FILE_SOURCE: &str = "./move_file_source.txt";
    const FILE_DEST_DIR: &str = "./move_file_out_dir";
    const FILE_DEST: &str = "./move_file_out_dir/move_file_source.txt";

    fn file_setup() -> std::io::Result<()> {
        File::create(FILE_SOURCE)?;
        fs::create_dir_all(FILE_DEST_DIR)
    }

    #[test]
    #[allow(unused_must_use)]
    fn move_file_works() {
        assert_eq!((), file_setup().unwrap());

        let mut op = MoveFile::new(FILE_SOURCE, FILE_DEST);

        assert_eq!(true, Path::new(FILE_SOURCE).exists());
        assert_eq!(false, Path::new(FILE_DEST).exists());

        assert_eq!((), op.execute().unwrap());
        assert_eq!(false, Path::new(FILE_SOURCE).exists());
        assert_eq!(true, Path::new(FILE_DEST).exists());

        assert_eq!((), op.rollback().unwrap());
        assert_eq!(true, Path::new(FILE_SOURCE).exists());
        assert_eq!(false, Path::new(FILE_DEST).exists());

        fs::remove_file(FILE_SOURCE);
        fs::remove_dir_all(FILE_DEST_DIR);
    }

    const DIR_SOURCE: &str = "./move_dir_source";
    const DIR_DIR: &str = "./move_dir_dest_dir";
    const DIR_DEST: &str = "./move_dir_dest_dir/move_dir_source";

    fn dir_setup() -> std::io::Result<()> {
        fs::create_dir_all(DIR_SOURCE)?;
        fs::create_dir_all(DIR_DIR)
    }

    #[test]
    #[allow(unused_must_use)]
    fn move_dir_works() {
        assert_eq!((), dir_setup().unwrap());

        let mut op = MoveDirectory::new(DIR_SOURCE, DIR_DEST);

        assert_eq!(true, Path::new(DIR_SOURCE).exists());
        assert_eq!(false, Path::new(DIR_DEST).exists());

        assert_eq!((), op.execute().unwrap());
        assert_eq!(false, Path::new(DIR_SOURCE).exists());
        assert_eq!(true, Path::new(DIR_DEST).exists());

        assert_eq!((), op.rollback().unwrap());
        assert_eq!(true, Path::new(DIR_SOURCE).exists());
        assert_eq!(false, Path::new(DIR_DEST).exists());

        fs::remove_dir_all(DIR_SOURCE);
        fs::remove_dir_all(DIR_DIR);
    }
}
