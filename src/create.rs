use std::io;
use std::fs::{self, File};

use crate::{RollbackableOperation};

pub struct CreateFile {
	path: String,
}

impl CreateFile {
	pub fn new<S: Into<String>>(path: S) -> Self {
		Self {
			path: path.into()
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

pub struct CreateDirectory {
	path: String,
}

impl CreateDirectory {
	pub fn new<S: Into<String>>(path: S) -> Self {
		Self {
			path: path.into()
		}
	}
}

impl RollbackableOperation for CreateDirectory {
	fn execute(&mut self) -> io::Result<()> {
		fs::create_dir_all(&self.path)
	}

	fn rollback(&self) -> io::Result<()> {
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