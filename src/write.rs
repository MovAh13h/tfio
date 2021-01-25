use std::{fs::OpenOptions, path::{Path, PathBuf}};
use std::io::{self, Write, Read};

use crate::{RollbackableOperation, SingleFileOperation};

/// Writes data to a file
pub struct WriteFile {
	source: PathBuf,
	temp_dir: PathBuf,
	backup_path: PathBuf,
	data: Vec<u8>,
}

impl WriteFile {
	/// Constructs a new WriteFile operation
	pub fn new<S: AsRef<Path>, T: AsRef<Path>>(source: S, temp_dir: T, data: Vec<u8>) -> Self {
		Self {
			source: source.as_ref().into(),
			temp_dir: temp_dir.as_ref().into(),
			backup_path: PathBuf::new(),
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

impl Drop for WriteFile {
	fn drop(&mut self) {
		match self.dispose() {
			Err(e) => eprintln!("{}", e),
			_ => {}
		}
	}
}

#[cfg(test)]
mod tests {
	use std::fs::{self, File};

	use super::*;

	const FILE_SOURCE: &str = "./write_file_source.txt";
	const TEMP_DIR: &str = "./tmp/";
	const INITIAL_DATA: &[u8] = "Yellow World".as_bytes();
	const WRITTEN_DATA: &[u8] = "Hello World".as_bytes();

	fn write_setup() -> std::io::Result<()> {
		match File::create(FILE_SOURCE) {
			Ok(_f) => Ok(()),
			Err(e) => Err(e),
		}
	}

	#[test]
	#[allow(unused_must_use)]
	fn write_file_works() {
		assert_eq!((), write_setup().unwrap());

		fs::write(FILE_SOURCE, INITIAL_DATA).expect("Unable to write file");

		let mut op = WriteFile::new(FILE_SOURCE, TEMP_DIR, WRITTEN_DATA.to_vec());
		
		assert_eq!((), op.execute().unwrap());
		let data = fs::read_to_string(FILE_SOURCE).expect("Unable to read file");
		assert_eq!(String::from("Hello Worldd"), data);

		assert_eq!((), op.rollback().unwrap());
		let data = fs::read_to_string(FILE_SOURCE).expect("Unable to read file");
		assert_eq!(String::from_utf8(INITIAL_DATA.to_vec()).unwrap(), data);

		fs::remove_file(FILE_SOURCE);
	}
}
