use std::{io, path::{Path, PathBuf}};
use std::fs::{self, File};

use crate::RollbackableOperation;

/// Creates a new file
pub struct CreateFile {
	path: PathBuf,
}

impl CreateFile {
	/// Constructs a new CreateFile operation
	pub fn new<S: AsRef<Path>>(path: S) -> Self {
		Self {
			path: path.as_ref().into()
		}
	}
}

impl RollbackableOperation for CreateFile {
	fn execute(&mut self) -> io::Result<()> {
		match File::create(&self.path) {
			Ok(_f) => Ok(()),
			Err(e) => Err(e),
		}
	}

	fn rollback(&self) -> io::Result<()> {
		fs::remove_file(&self.path)
	}
}

/// Creates a new directory
pub struct CreateDirectory {
	path: PathBuf,
}

impl CreateDirectory {
	/// Constructs a new CreateDirectory operation
	pub fn new<S: AsRef<Path>>(path: S) -> Self {
		Self {
			path: path.as_ref().into()
		}
	}
}

impl RollbackableOperation for CreateDirectory {
	fn execute(&mut self) -> io::Result<()> {
		fs::create_dir_all(&self.path)
	}

	fn rollback(&self) -> io::Result<()> {
		// TODO: So bad
		fs::remove_dir_all(&self.path)
	}
}

#[cfg(test)]
mod tests {
	use std::path::Path;
	use super::*;

	const FILE_SOURCE: &str = "./create_file_source.txt";

	#[test]
	fn create_file_works() {
		let mut op = CreateFile::new(FILE_SOURCE);

		assert_eq!(false, Path::new(FILE_SOURCE).exists());
		assert_eq!((), op.execute().unwrap());
		assert_eq!(true, Path::new(FILE_SOURCE).exists());
		assert_eq!((), op.rollback().unwrap());
		assert_eq!(false, Path::new(FILE_SOURCE).exists());
	}

	const DIR_SOURCE: &str = "./create_dir";

	#[test]
	fn create_dir_works() {
		let mut op = CreateDirectory::new(DIR_SOURCE);

		assert_eq!(false, Path::new(DIR_SOURCE).exists());
		assert_eq!((), op.execute().unwrap());
		assert_eq!(true, Path::new(DIR_SOURCE).exists());
		assert_eq!((), op.rollback().unwrap());
		assert_eq!(false, Path::new(DIR_SOURCE).exists());
	}
}
