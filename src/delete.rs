use std::io;
use std::fs;

use crate::{RollbackableOperation, SingleFileOperation, DirectoryOperation};

pub struct DeleteFile {
	source: String,
	temp_dir: String,
	backup_path: String,
}

impl DeleteFile {
	pub fn new<S: Into<String>>(source: S, temp_dir: S) -> Self {
		Self {
			source: source.into(),
			temp_dir: temp_dir.into(),
			backup_path: String::new(),
		}
	}
}

impl RollbackableOperation for DeleteFile {
	fn execute(&mut self) -> io::Result<()> {
		self.ensure_temp_dir_exists()?;
		self.create_backup_file()?;

		fs::remove_file(self.get_path())
	}

	fn rollback(&self) -> io::Result<()> {
		match fs::copy(self.get_backup_path(), self.get_path()) {
			Ok(_v) => Ok(()),
			Err(e) => Err(e),
		}
	}
}

impl SingleFileOperation for DeleteFile {
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

impl Drop for DeleteFile {
	fn drop(&mut self) {
		match self.dispose() {
			Err(e) => eprintln!("{}", e),
			_ => {}
		}
	}
}


pub struct DeleteDirectory {
	source: String,
	backup_path: String,
	temp_dir: String,
}

impl DeleteDirectory {
	pub fn new<S: Into<String>>(source: S, temp_dir: S) -> Self {
		Self {
			source: source.into(),
			temp_dir: temp_dir.into(),
			backup_path: String::new(),
		}
	}
}

impl RollbackableOperation for DeleteDirectory {
	fn execute(&mut self) -> io::Result<()> {
		self.create_backup_folder()?;
		fs::remove_dir_all(&self.source)
	}

	fn rollback(&self) -> io::Result<()> {
		fs::rename(self.get_backup_path(), &self.source)
	}
}

impl DirectoryOperation for DeleteDirectory {
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

impl Drop for DeleteDirectory {
	fn drop(&mut self) {
		match self.dispose() {
			Err(e) => eprintln!("{}", e),
			_ => {}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const FILE_SOURCE: &str = "./test/delete/file/out.txt";
	const TEMP_DIR: &str = "./tmp/";

	#[test]
	fn delete_file_execute_rollback() {
		let mut op = DeleteFile::new(FILE_SOURCE, TEMP_DIR);
		assert_eq!((), op.execute().unwrap());
		assert_eq!((), op.rollback().unwrap());
	}

	#[test]
	fn delete_dir_execute_rollback() {
		let _op = DeleteDirectory::new("!", "2");
	}
}