use std::io::{self, Write, Read};
use std::fs::{OpenOptions};

use crate::{RollbackableOperation, SingleFileOperation};


pub struct AppendFile<'a> {
	source: String,
	temp_dir: String,
	backup_path: String,
	data: &'a [u8],
}

impl<'a> AppendFile<'a> {
	pub fn new<S: Into<String>>(source: S, temp_dir: S, data: &'a [u8]) -> Self {
		Self {
			source: source.into(),
			temp_dir: temp_dir.into(),
			backup_path: String::new(),
			data: data,
		}
	}
}

impl RollbackableOperation for AppendFile<'_> {
	fn execute(&mut self) -> io::Result<()> {
		self.ensure_temp_dir_exists()?;
		self.create_backup_file()?;

		OpenOptions::new().append(true).open(self.get_path())?.write_all(&self.data)
	}

	fn rollback(&self) -> io::Result<()> {
		let mut buffer = Vec::<u8>::new();
		let mut backup_file = OpenOptions::new().read(true).open(self.get_backup_path())?;
		
		backup_file.read_to_end(&mut buffer)?;

		OpenOptions::new().write(true).truncate(true).open(self.get_path())?.write_all(&buffer)
	}
}

impl SingleFileOperation for AppendFile<'_> {
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

impl Drop for AppendFile<'_> {
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

	const FILE_SOURCE: &str = "./test/append/file/out.txt";
	const TEMP_DIR: &str = "./tmp/";
	const DATA: &[u8] = "6789".as_bytes();

	#[test]
	fn append_file_execute_rollback() {
		let mut op = AppendFile::new(FILE_SOURCE, TEMP_DIR, DATA);
		assert_eq!((), op.execute().unwrap());
		assert_eq!((), op.rollback().unwrap());
	}
}