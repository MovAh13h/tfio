use std::io;
use std::fs;

use crate::{RollbackableOperation, SingleFileOperation};

pub struct CopyFile {
	source: String,
	dest: String,
	temp_dir: String,
	backup_path: String,
}

impl CopyFile {
	pub fn new<S: Into<String>>(source: S, dest: S, temp_dir: S) -> Self {
		Self {
			source: source.into(),
			dest: dest.into(),
			temp_dir: temp_dir.into(),
			backup_path: String::new(),
		}
	}
}

impl RollbackableOperation for CopyFile {
	fn execute(&mut self) -> io::Result<()> {
		self.ensure_temp_dir_exists();
		self.create_backup_file();

		match fs::copy(&self.source, &self.dest) {
			Ok(_v) => Ok(()),
			Err(e) => Err(e),
		}
	}

	fn rollback(&self) -> io::Result<()> {
		fs::remove_file(&self.dest)
	}
}

impl SingleFileOperation for CopyFile {
	fn get_path(&self) -> &String {
		&self.source
	}

	fn get_backup_path(&self) -> &String {
		&self.backup_path
	}

	fn set_backup_path<S: Into<String>>(&mut self, uuid: S) {
		self.backup_path = uuid.into();
	}

	fn get_temp_dir(&self) -> &String {
		&self.temp_dir
	}
}

impl Drop for CopyFile {
	fn drop(&mut self) {
		self.dispose();
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const FILE_SOURCE: &str = "./test/copy/file/out.txt";
	const FILE_DEST: &str = "./test/copy/file/inner/out.txt";
	const TEMP_DIR: &str = "./tmp/";

	#[test]
	fn copy_file_execute() {
		let mut op = CopyFile::new(FILE_SOURCE, FILE_DEST, TEMP_DIR);
		assert_eq!((), op.execute().unwrap());
	}

	#[test]
	fn copy_file_rollback() {
		let mut op = CopyFile::new(FILE_SOURCE, FILE_DEST, TEMP_DIR);
		assert_eq!((), op.rollback().unwrap());
	}
}