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
	use super::*;

	const FILE_SOURCE: &str = "./test/copy/file/out.txt";
	const FILE_DEST: &str = "./test/copy/file/inner/out.txt";

	#[test]
	fn copy_file_execute_rollback() {
		let mut op = CopyFile::new(FILE_SOURCE, FILE_DEST);
		assert_eq!((), op.execute().unwrap());
		assert_eq!((), op.rollback().unwrap());
	}

	const DIR_SOURCE: &str = "./test/copy/folder/out";
	const DIR_DEST: &str = "./test/copy/folder/inner";
	const DIR_TEMP: &str = "./tmp";

	#[test]
	fn copy_folder_execute_rollback() {
		let mut op = CopyDirectory::new(DIR_SOURCE, DIR_DEST, DIR_TEMP);
		assert_eq!((), op.execute().unwrap());
		assert_eq!((), op.rollback().unwrap());
	}
}