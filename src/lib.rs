mod copy;
mod r#move;
mod delete;
mod write;
mod append;
mod create;

use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write, Error, ErrorKind};
use std::path::{PathBuf, Path};

use uuid::Uuid;

pub use append::AppendFile;
pub use write::WriteFile;
pub use copy::{CopyFile, CopyDirectory};
pub use r#move::{MoveFile, MoveDirectory};
pub use delete::{DeleteFile, DeleteDirectory};
pub use create::{CreateFile, CreateDirectory};

pub trait RollbackableOperation {
	fn execute(&mut self) -> io::Result<()>;
	fn rollback(&self) -> io::Result<()>;
}

pub trait DirectoryOperation : RollbackableOperation + Drop {
	// Path to folder
	fn get_path(&self) -> &String;

	// Getters/Setters for backup path
	fn get_backup_path(&self) -> &String;
	fn set_backup_path<S: Into<String>>(&mut self, uuid: S);

	// Path to temp dir
	fn get_temp_dir(&self) -> &String;

	// Dispose off resources used by the operation
	// It is called once all operations in the Transaction are completed successfully
	fn dispose(&self) -> io::Result<()> {
		fs::remove_dir(self.get_backup_path())
	}

	fn create_backup_folder(&mut self) -> io::Result<()> {
		fs::create_dir_all(&self.get_temp_dir())?;

		let uuid = Uuid::new_v4();
		let mut buffer = [b' '; 36];
		
		uuid.to_hyphenated().encode_lower(&mut buffer);

		let uuid_str = String::from_utf8(buffer.to_vec()).expect(format!("Could not convert buffer to String").as_str());
		let backup_path = Path::new(&self.get_temp_dir()).join(uuid_str).to_str().unwrap().to_string();

		copy_dir(self.get_path(), &backup_path)?;
		
		self.set_backup_path(&backup_path);

		Ok(())
	}
}

pub trait SingleFileOperation: RollbackableOperation + Drop {
	// Path to the file
	fn get_path(&self) -> &String;

	// Getters/Setters for backup path
	fn get_backup_path(&self) -> &String;
	fn set_backup_path<S: Into<String>>(&mut self, uuid: S);

	// Path to temp dir
	fn get_temp_dir(&self) -> &String;

	// Dispose off resources used by the operation
	// It is called once all operations in the Transaction are completed successfully
	fn dispose(&self) -> io::Result<()> {
		fs::remove_file(self.get_backup_path())
	}

	// Create a temp file that is just a clone of the source file
	// If backup file is successfully created, method should call `self.set_backup_path`
	fn create_backup_file(&mut self) -> io::Result<()> {
		fs::create_dir_all(&self.get_temp_dir())?;

		let uuid = Uuid::new_v4();
		let mut buffer = [b' '; 36];
		
		uuid.to_hyphenated().encode_lower(&mut buffer);

		let uuid_str = String::from_utf8(buffer.to_vec()).expect(format!("Could not convert buffer to String").as_str());
		let backup_path = Path::new(&self.get_temp_dir()).join(uuid_str).to_str().unwrap().to_string();

		let mut buffer = Vec::new();
		let mut dest_file = OpenOptions::new().write(true).create(true).open(&backup_path)?;
		let mut source_file = OpenOptions::new().read(true).open(&self.get_path())?;

		source_file.read_to_end(&mut buffer)?;
		dest_file.write_all(&buffer)?;
		self.set_backup_path(&backup_path);

		Ok(())
	}
}

fn copy_dir<U: AsRef<Path>, V: AsRef<Path>>(from: U, to: V) -> io::Result<()> {
	let mut stack = Vec::new();
	stack.push(PathBuf::from(from.as_ref()));

	let output_root = PathBuf::from(to.as_ref());
	let input_root = PathBuf::from(from.as_ref()).components().count();

	while let Some(working_path) = stack.pop() {
		let src: PathBuf = working_path.components().skip(input_root).collect();

		let dest = if src.components().count() == 0 {
			output_root.clone()
		} else {
			output_root.join(&src)
		};

		if fs::metadata(&dest).is_err() {
			fs::create_dir_all(&dest)?;
		}

		for entry in fs::read_dir(working_path)? {
			let path = entry?.path();

			if path.is_dir() {
				stack.push(path);
			} else {
				match path.file_name() {
					Some(filename) => {
						let dest_path = dest.join(filename);
						fs::copy(&path, &dest_path)?;
					}
					None => return Err(Error::new(ErrorKind::Other, "Could not extract filename from path")),
				}
			}
		}
	}

	Ok(())
}


pub struct Transaction {
	ops: Vec<Box<dyn RollbackableOperation>>,
	execution_count: usize,
}

impl Transaction {
	pub fn new() -> Self {
		Self {
			ops: vec![],
			execution_count: 0,
		}
	}

	pub fn create_file<S: Into<String>>(mut self, path: S) -> Self {
		self.ops.push(Box::new(CreateFile::new(path)));
		self
	}

	pub fn create_dir<S: Into<String>>(mut self, path: S) -> Self {
		self.ops.push(Box::new(CreateDirectory::new(path)));
		self
	}

	pub fn append_file<S: Into<String>>(mut self, source: S, temp_dir: S, data: Vec<u8>) -> Self {
		self.ops.push(Box::new(AppendFile::new(source, temp_dir, data)));
		self
	}

	pub fn copy_file<S: Into<String>>(mut self, source: S, dest: S) -> Self {
		self.ops.push(Box::new(CopyFile::new(source, dest)));
		self
	}

	pub fn copy_dir<S: Into<String>>(mut self, source: S, dest: S, temp_dir: S) -> Self {
		self.ops.push(Box::new(CopyDirectory::new(source, dest, temp_dir)));
		self
	}

	pub fn delete_file<S: Into<String>>(mut self, source: S, temp_dir: S) -> Self {
		self.ops.push(Box::new(DeleteFile::new(source, temp_dir)));
		self
	}

	pub fn delete_dir<S: Into<String>>(mut self, source: S, temp_dir: S) -> Self {
		self.ops.push(Box::new(DeleteDirectory::new(source, temp_dir)));
		self
	}

	pub fn move_file<S: Into<String>>(mut self, source: S, dest: S) -> Self {
		self.ops.push(Box::new(MoveFile::new(source, dest)));
		self
	}

	pub fn move_dir<S: Into<String>>(mut self, source: S, dest: S) -> Self {
		self.ops.push(Box::new(MoveDirectory::new(source, dest)));
		self
	}

	pub fn write_file<S: Into<String>>(mut self, source: S, temp_dir: S, data: Vec<u8>) -> Self {
		self.ops.push(Box::new(WriteFile::new(source, temp_dir, data)));
		self
	}
}

impl RollbackableOperation for Transaction {
	fn execute(&mut self) -> io::Result<()> {
		for i in 0..self.ops.len() {
			self.execution_count += 1;
			if let Err(e) = self.ops[i].execute() {
				return Err(e);
			}
		}

		Ok(())
	}

	fn rollback(&self) -> io::Result<()> {
		for i in (0..self.execution_count).rev() {
			if let Err(e) = self.ops[i].rollback() {
				return Err(e);
			}
		}

		Ok(())
	}
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	#[allow(unused)]
	fn transaction_works() {
		let temp_dir = "./tmp";
		let mut tr = Transaction::new()
						.create_file("./created_file_transaction.txt")
						.create_file("./for_delete.txt")
						.create_dir("./inner/create_dir_transaction")
						.create_dir("./for_delete_dir")
						.create_dir("./magic_dir")
						.write_file("./created_file_transaction.txt", temp_dir, b"Hello World".to_vec())
						.append_file("./created_file_transaction.txt", temp_dir, b"Hello World".to_vec())
						.copy_file("./created_file_transaction.txt", "./inner/created_file_transaction.txt")
						.copy_dir("./magic_dir", "./inner/magic_dir", temp_dir)
						.delete_file("./for_delete.txt", temp_dir)
						.delete_dir("./for_delete_dir", temp_dir)
						.move_file("./inner/created_file_transaction.txt", "./inner/magic_dir/created_file_transaction.txt")
						.create_dir("./for_moving")
						.move_dir("./for_moving", "./inner/magic_dir/for_moving");


		assert_eq!((), tr.execute().expect("Cannot execute"));
		assert_eq!((), tr.rollback().expect("Cannot Rollback"));
	}
}