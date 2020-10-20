use std::io;
use std::fs;

use crate::{RollbackableOperation};

pub type MoveFile = MoveOperation;
pub type MoveDirectory = MoveOperation;

pub struct MoveOperation {
	source: String,
	dest: String,
}

impl MoveFile {
	pub fn new<S: Into<String>>(source: S, dest: S) -> Self {
		Self {
			source: source.into(),
			dest: dest.into(),
		}
	}
}

impl RollbackableOperation for MoveFile {
	fn execute(&mut self) -> io::Result<()> {
		fs::rename(&self.source, &self.dest)
	}

	fn rollback(&self) -> io::Result<()> {
		fs::rename(&self.dest, &self.source)
	}
}



#[cfg(test)]
mod tests {
	use super::*;

	const FILE_SOURCE: &str = "./test/move/file/out.txt";
	const FILE_DEST: &str = "./test/move/file/inner/out.txt";

	#[test]
	fn move_file_execute_rollback() {
		let mut op = MoveFile::new(FILE_SOURCE, FILE_DEST);
		assert_eq!((), op.execute().unwrap());
		assert_eq!((), op.rollback().unwrap());
	}
}