use std::io;
use std::fs;

use crate::{RollbackableOperation, SingleFileOperation};

pub struct MoveFile {
	source: String,
	dest: String,
	temp_dir: String,
	backup_path: String,
}

impl MoveFile {
	pub fn new<S: Into<String>>(source: S, dest: S, temp_dir: S) -> Self {
		Self {
			source: source.into(),
			dest: dest.into(),
			temp_dir: temp_dir.into(),
			backup_path: String::new(),
		}
	}
}

impl RollbackableOperation for MoveFile {
	fn execute(&mut self) -> io::Result<()> {
		self.ensure_temp_dir_exists();
		self.create_backup_file();

		fs::rename(&self.source, &self.dest)
	}

	fn rollback(&self) -> io::Result<()> {
		fs::rename(&self.dest, &self.source)
	}
}

impl SingleFileOperation for MoveFile {
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

impl Drop for MoveFile {
	fn drop(&mut self) {
		self.dispose();
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const FILE_SOURCE: &str = "./test/move/file/out.txt";
	const FILE_DEST: &str = "./test/move/file/inner/out.txt";
	const TEMP_DIR: &str = "./tmp/";

	#[test]
	fn move_file_execute() {
		let mut op = MoveFile::new(FILE_SOURCE, FILE_DEST, TEMP_DIR);
		assert_eq!((), op.execute().unwrap());
	}

	#[test]
	fn move_file_rollback() {
		let mut op = MoveFile::new(FILE_SOURCE, FILE_DEST, TEMP_DIR);
		assert_eq!((), op.rollback().unwrap());
	}
}