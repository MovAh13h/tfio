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
	use std::path::Path;
	use std::fs::File;
	use super::*;

	const FILE_SOURCE: &str = "./delete_file_source";
	const TEMP_DIR: &str = "./tmp/";

	fn file_setup() -> std::io::Result<()> {
		match File::create(FILE_SOURCE) {
			Ok(_f) => Ok(()),
			Err(e) => Err(e),
		}
	}

	#[test]
	#[allow(unused_must_use)]
	fn delete_file_works() {
		assert_eq!((), file_setup().unwrap());

		let mut op = DeleteFile::new(FILE_SOURCE, TEMP_DIR);
		
		assert_eq!(true, Path::new(FILE_SOURCE).exists());
		assert_eq!((), op.execute().unwrap());
		assert_eq!(false, Path::new(FILE_SOURCE).exists());
		assert_eq!((), op.rollback().unwrap());
		assert_eq!(true, Path::new(FILE_SOURCE).exists());

		fs::remove_file(FILE_SOURCE);
	}

	const DIR_SOURCE: &str = "./delete_dir_source";

	fn dir_setup() -> std::io::Result<()> {
		fs::create_dir(DIR_SOURCE)
	}

	#[test]
	#[allow(unused_must_use)]
	fn delete_dir_works() {
		assert_eq!((), dir_setup().unwrap());

		let mut op = DeleteDirectory::new(DIR_SOURCE, TEMP_DIR);

		assert_eq!(true, Path::new(DIR_SOURCE).exists());
		assert_eq!((), op.execute().unwrap());
		assert_eq!(false, Path::new(DIR_SOURCE).exists());
		assert_eq!((), op.rollback().unwrap());
		assert_eq!(true, Path::new(DIR_SOURCE).exists());

		fs::remove_dir_all(DIR_SOURCE);
	}
}