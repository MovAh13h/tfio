use std::io;
use std::fs;

use crate::{RollbackableOperation, DirectoryOperation, copy_dir};

pub struct CopyFile {
	source: String,
	dest: String,
}

impl CopyFile {
	pub fn new<S: Into<String>>(source: S, dest: S) -> Self {
		Self {
			source: source.into(),
			dest: dest.into(),
		}
	}
}

impl RollbackableOperation for CopyFile {
	fn execute(&mut self) -> io::Result<()> {
		match fs::copy(&self.source, &self.dest) {
			Ok(_v) => Ok(()),
			Err(e) => Err(e),
		}
	}

	fn rollback(&self) -> io::Result<()> {
		fs::remove_file(&self.dest)
	}
}

pub struct CopyDirectory {
	source: String,
	dest: String,
	backup_path: String,
	temp_dir: String,
}

impl CopyDirectory {
	pub fn new<S: Into<String>>(source: S, dest: S, temp_dir: S) -> Self {
		Self {
			source: source.into(),
			dest: dest.into(),
			temp_dir: temp_dir.into(),
			backup_path: String::new(),
		}
	}
}

impl RollbackableOperation for CopyDirectory {
	fn execute(&mut self) -> io::Result<()> {
		self.create_backup_folder()?;
		copy_dir(&self.source, &self.dest)
	}

	fn rollback(&self) -> io::Result<()> {
		fs::remove_dir_all(&self.dest)
	}
}

impl DirectoryOperation for CopyDirectory {
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

impl Drop for CopyDirectory {
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
	use std::fs::{self, File};
	use super::*;

	const FILE_SOURCE: &str = "./copy_file_source.txt";
	const DEST_DIR: &str = "./copy_file_dir";
	const FILE_DEST: &str = "./copy_file_dir/copy_file_source.txt";

	fn file_setup() -> std::io::Result<()> {
		File::create(FILE_SOURCE)?;
		fs::create_dir(DEST_DIR)
	}

	#[test]
	#[allow(unused_must_use)]
	fn copy_file_works() {
		assert_eq!((), file_setup().unwrap());

		let mut op = CopyFile::new(FILE_SOURCE, FILE_DEST);
		
		assert_eq!(false, Path::new(FILE_DEST).exists());
		assert_eq!((), op.execute().unwrap());
		assert_eq!(true, Path::new(FILE_SOURCE).exists());
		assert_eq!(true, Path::new(FILE_DEST).exists());

		assert_eq!((), op.rollback().unwrap());
		assert_eq!(true, Path::new(FILE_SOURCE).exists());
		assert_eq!(false, Path::new(FILE_DEST).exists());

		fs::remove_file(FILE_SOURCE);
		fs::remove_dir_all(DEST_DIR);
	}

	const DIR_SOURCE: &str = "./copy_dir_source";
	const DIR_DIR: &str = "./copy_dest_dir";
	const DIR_DEST: &str = "./copy_dest_dir/copy_dir_source";
	const DIR_TEMP: &str = "./tmp";

	fn folder_setup() -> std::io::Result<()> {
		fs::create_dir(DIR_SOURCE)?;
		fs::create_dir(DIR_DIR)
	}

	#[test]
	#[allow(unused_must_use)]
	fn copy_dir_works() {
		assert_eq!((), folder_setup().unwrap());

		let mut op = CopyDirectory::new(DIR_SOURCE, DIR_DEST, DIR_TEMP);
		
		assert_eq!((), op.execute().unwrap());
		assert_eq!(true, Path::new(DIR_SOURCE).exists());
		assert_eq!(true, Path::new(DIR_DEST).exists());

		assert_eq!((), op.rollback().unwrap());
		assert_eq!(true, Path::new(DIR_SOURCE).exists());
		assert_eq!(false, Path::new(DIR_DEST).exists());

		fs::remove_dir_all(DIR_SOURCE);
		fs::remove_dir(DIR_DIR);
	}
}