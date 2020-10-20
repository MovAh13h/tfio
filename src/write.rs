use std::io::{self, Write, Read};
use std::fs::{OpenOptions};

use crate::{RollbackableOperation, SingleFileOperation};

pub struct WriteFile {
	source: String,
	temp_dir: String,
	backup_path: String,
	data: Vec<u8>,
}

impl WriteFile {
	pub fn new<S: Into<String>>(source: S, temp_dir: S, data: Vec<u8>) -> Self {
		Self {
			source: source.into(),
			temp_dir: temp_dir.into(),
			backup_path: String::new(),
			data: data,
		}
	}
}

impl RollbackableOperation for WriteFile {
	fn execute(&mut self) -> io::Result<()> {
		self.create_backup_file()?;

		OpenOptions::new().write(true).open(self.get_path())?.write_all(&self.data)
	}

	fn rollback(&self) -> io::Result<()> {
		let mut buffer = Vec::<u8>::new();
		let mut backup_file = OpenOptions::new().read(true).open(self.get_backup_path())?;
		
		backup_file.read_to_end(&mut buffer)?;

		OpenOptions::new().write(true).truncate(true).open(self.get_path())?.write_all(&buffer)
	}
}

impl SingleFileOperation for WriteFile {
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

impl Drop for WriteFile {
	fn drop(&mut self) {
		match self.dispose() {
			Err(e) => eprintln!("{}", e),
			_ => {}
		}
	}
}

pub struct WriteAndCreateFile {
	source: String,
	temp_dir: String,
	backup_path: String,
	data: Vec<u8>,
}

impl WriteAndCreateFile {
	pub fn new<S: Into<String>>(source: S, temp_dir: S, data: Vec<u8>) -> Self {
		Self {
			source: source.into(),
			temp_dir: temp_dir.into(),
			backup_path: String::new(),
			data: data,
		}
	}
}

impl RollbackableOperation for WriteAndCreateFile {
	fn execute(&mut self) -> io::Result<()> {
		self.create_backup_file()?;

		OpenOptions::new().write(true).create(true).open(self.get_path())?.write_all(&self.data)
	}

	fn rollback(&self) -> io::Result<()> {
		let mut buffer = Vec::<u8>::new();
		let mut backup_file = OpenOptions::new().read(true).open(self.get_backup_path())?;
		
		backup_file.read_to_end(&mut buffer)?;

		OpenOptions::new().write(true).open(self.get_path())?.write_all(&buffer)
	}
}

impl SingleFileOperation for WriteAndCreateFile {
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

impl Drop for WriteAndCreateFile {
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

	const FILE_SOURCE: &str = "./test/write/file/out.txt";
	const TEMP_DIR: &str = "./tmp/";
	const DATA: &[u8] = "Updated 123 45632434".as_bytes();

	#[test]
	fn write_file_execute_rollback() {
		let mut op = WriteFile::new(FILE_SOURCE, TEMP_DIR, DATA.to_vec());
		assert_eq!((), op.execute().unwrap());
		assert_eq!((), op.rollback().unwrap());
	}
}