use std::{io, path::{Path, PathBuf}};
use std::fs;

use crate::{RollbackableOperation, DirectoryOperation, copy_dir};

/// Copies a file to destination
pub struct CopyFile {
	source: PathBuf,
	dest: PathBuf,
}

impl CopyFile {
	/// Constructs a new CopyFile operation
	pub fn new<S: AsRef<Path>, T: AsRef<Path>>(source: S, dest: T) -> Self {
		Self {
			source: source.as_ref().into(),
			dest: dest.as_ref().into(),
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

/// Copies a directory to destination
pub struct CopyDirectory {
	source: PathBuf,
	dest: PathBuf,
	backup_path: PathBuf,
	temp_dir: PathBuf,
}

impl CopyDirectory {
	/// Constructs a new CopyDirectory operation
	pub fn new<S: AsRef<Path>, T: AsRef<Path>, U: AsRef<Path>>(source: S, dest: T, temp_dir: U) -> Self {
		Self {
			source: source.as_ref().into(),
			dest: dest.as_ref().into(),
			temp_dir: temp_dir.as_ref().into(),
			backup_path: PathBuf::new(),
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
	fn get_path(&self) -> &Path {
		&self.source
	}

	fn get_backup_path(&self) -> &Path {
		&self.backup_path
	}

	fn set_backup_path<S: AsRef<Path>>(&mut self, uuid: S) {
		self.backup_path = uuid.as_ref().into();
	}

	fn get_temp_dir(&self) -> &Path {
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
