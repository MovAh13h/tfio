use std::io::{self, Write, Read};
use std::fs::{self, OpenOptions};

use crate::{RollbackableOperation, SingleFileOperation};

pub struct WriteFile<'a> {
	source: String,
	temp_dir: String,
	backup_path: String,
	data: &'a [u8],
}

impl<'a> WriteFile<'a> {
	pub fn new<S: Into<String>>(source: S, temp_dir: S, data: &'a [u8]) -> Self {
		Self {
			source: source.into(),
			temp_dir: temp_dir.into(),
			backup_path: String::new(),
			data: data,
		}
	}
}

impl RollbackableOperation for WriteFile<'_> {
	fn execute(&mut self) -> io::Result<()> {
		self.ensure_temp_dir_exists();
		self.create_backup_file();

		OpenOptions::new().write(true).open(&self.source)?.write_all(&self.data)
	}

	fn rollback(&self) -> io::Result<()> {
		let mut buffer = Vec::<u8>::new();
		let mut backup_file = OpenOptions::new().read(true).open(self.get_backup_path())?;
		
		backup_file.read_to_end(&mut buffer)?;

		OpenOptions::new().write(true).create(true).open(self.get_path())?.write_all(&buffer)
	}
}

impl SingleFileOperation for WriteFile<'_> {
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

impl Drop for WriteFile<'_> {
	fn drop(&mut self) {
		self.dispose();
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
		let mut op = WriteFile::new(FILE_SOURCE, TEMP_DIR, DATA);
		assert_eq!((), op.execute().unwrap());
		assert_eq!((), op.rollback().unwrap());
	}
}