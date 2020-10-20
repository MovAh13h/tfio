use std::fs::OpenOptions;
use std::io::{self, Write, Read};

use crate::{RollbackableOperation, SingleFileOperation};

pub struct AppendFile {
	source: String,
	temp_dir: String,
	backup_path: String,
	data: Vec<u8>,
}

impl AppendFile {
	pub fn new<S: Into<String>>(source: S, temp_dir: S, data: Vec<u8>) -> Self {
		Self {
			source: source.into(),
			temp_dir: temp_dir.into(),
			backup_path: String::new(),
			data: data,
		}
	}
}

impl RollbackableOperation for AppendFile {
	fn execute(&mut self) -> io::Result<()> {
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

impl SingleFileOperation for AppendFile {
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

impl Drop for AppendFile {
	fn drop(&mut self) {
		match self.dispose() {
			Err(e) => eprintln!("{}", e),
			_ => {}
		}
	}
}

#[cfg(test)]
mod tests {
	use std::io;
	use std::fs::{self, File};
	
	use super::*;

	const FILE_SOURCE: &str = "./append.txt";
	const TEMP_DIR: &str = "./tmp/";
	const DATA: &[u8] = "Hello World".as_bytes();

	fn setup() -> io::Result<()> {
		match File::create(FILE_SOURCE) {
			Ok(_f) => Ok(()),
			Err(e) => Err(e),
		}
	}

	#[test]
	#[allow(unused_must_use)]
	fn append_file_works() {
		assert_eq!((), setup().expect("Unable to setup test"));

		fs::write(FILE_SOURCE, DATA).expect("Unable to write file");

		let mut op = AppendFile::new(FILE_SOURCE, TEMP_DIR, DATA.to_vec());
		assert_eq!((), op.execute().expect("Unable to perform execute"));

		let data = fs::read_to_string(FILE_SOURCE).expect("Unable to read file");
		assert_eq!(String::from_utf8([DATA, DATA].concat()).unwrap(), data);
		
		assert_eq!((), op.rollback().expect("Unable to perform rollback"));
		let data = fs::read_to_string(FILE_SOURCE).expect("Unable to read file");
		assert_eq!(String::from_utf8(DATA.to_vec()).unwrap(), data);
		
		fs::remove_file(FILE_SOURCE);
	}
}