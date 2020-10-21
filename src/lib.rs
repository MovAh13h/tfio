//! TFIO is a library that provides a Transaction-like interface that are traditionally used in databases on FileIO operations. It gives the flexibility to execute and rollback singlular operations as well as transactions on the fly. The library also provides a builder-pattern interface to chain operations and execute them in one go.
//!
//! # Examples
//!
//! Create a transaction and execute it. If any `Error` is encountered, rollback the entire transaction:
//! Note: The paths should only use forward slashes (`/`) and can either begin with disks or relative to the present working directory ie. begin with `./`
//! ```ignore
//! use std::io;
//! use tfio::*;
//! 
//! fn main() -> io::Result<()> {
//! 	let temp_dir = "./PATH_TO_TEMP_DIR";
//! 	let mut tr = Transaction::new()
//! 				.create_file("./foo.txt")
//! 				.create_dir("./bar")
//! 				.write_file("./foo.txt", temp_dir, b"Hello World".to_vec())
//! 				.move_file("./foo.txt", "./bar/foo.txt")
//! 				.append_file("./bar/foo.txt", temp_dir, b"dlroW olleH".to_vec());
//! 	
//! 	// Execute the transaction
//! 	if let Err(e) = tr.execute() {
//! 		eprintln!("Error during execution: {}", e);
//! 		
//! 		// All operations can be reverted in reverse-order if `Error` is encountered
//! 		if let Err(ee) = tr.rollback() {
//! 			panic!("Error during transaction rollback: {}", ee);
//! 		}
//! 	}
//! 	Ok(())
//! }
//! ```
//!
//! You can also import single operations to use:
//! ```ignore
//!	use std::io;
//! use tfio::{CopyFile, RollbackableOperation};
//! 
//! fn main() -> io::Result<()> {
//! 	let mut copy_operation = CopyFile::new("./foo.txt", "./bar/baz/foo.txt");
//! 	
//! 	// Execute the operation
//! 	if let Err(e) = copy_operation.execute() {
//! 		eprintln!("Error during execution: {}", e);
//! 		
//! 		// Rollback the operation
//! 		if let Err(ee) = copy_operation.rollback() {
//! 			panic!("Error during rollback: {}", ee);
//! 		}
//! 	}
//! 	Ok(())
//! }
//! ```

#![deny(missing_docs)]

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
pub use r#move::{MoveFile, MoveDirectory, MoveOperation};
pub use delete::{DeleteFile, DeleteDirectory};
pub use create::{CreateFile, CreateDirectory};

/// Trait that represents a Rollbackable operation
pub trait RollbackableOperation {
	/// Executes the operation
	fn execute(&mut self) -> io::Result<()>;

	/// Rollbacks the operation
	fn rollback(&self) -> io::Result<()>;
}

/// Trait that represents a Directory operation
pub trait DirectoryOperation : RollbackableOperation + Drop {
	/// Returns path to source directory
	fn get_path(&self) -> &String;

	/// Returns path to backup directory
	///
	/// Defaults to ""
	fn get_backup_path(&self) -> &String;
	
	/// Sets the backup path
	fn set_backup_path<S: Into<String>>(&mut self, uuid: S);

	/// Returns path to temp dir
	fn get_temp_dir(&self) -> &String;

	/// Dispose off resources used by the operation
	///
	/// It should be called inside [Drop](std::ops::Drop)
	fn dispose(&self) -> io::Result<()> {
		fs::remove_dir(self.get_backup_path())
	}

	/// Creates a backup of the source directory
	///
	/// If backup file is successfully created, method should call [set_backup_path](#method.set_backup_path)
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

/// Trait that represents a single file operation
pub trait SingleFileOperation: RollbackableOperation + Drop {
	/// Returns path to source file
	fn get_path(&self) -> &String;

	/// Returns path to backup file
	///
	/// Defaults to ""
	fn get_backup_path(&self) -> &String;

	/// Sets the backup path
	fn set_backup_path<S: Into<String>>(&mut self, uuid: S);

	/// Returns path to temp dir
	fn get_temp_dir(&self) -> &String;

	/// Dispose off resources used by the operation
	///
	/// It should be called inside [Drop](std::ops::Drop)
	fn dispose(&self) -> io::Result<()> {
		fs::remove_file(self.get_backup_path())
	}

	/// Creates a backup of the source file
	///
	/// If backup file is successfully created, method should call [set_backup_path](#method.set_backup_path)
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

/// A rollbackable Transaction
pub struct Transaction {
	ops: Vec<Box<dyn RollbackableOperation>>,
	execution_count: usize,
}

impl Transaction {
	/// Constructs a new, empty Transaction
	pub fn new() -> Self {
		Self {
			ops: vec![],
			execution_count: 0,
		}
	}

	/// Adds a [CreateFile](struct.CreateFile.html) operation to the transaction
	pub fn create_file<S: Into<String>>(mut self, path: S) -> Transaction {
		self.ops.push(Box::new(CreateFile::new(path)));
		self
	}

	/// Adds a [CreateDirectory](struct.CreateDirectory.html) operation to the transaction
	pub fn create_dir<S: Into<String>>(mut self, path: S) -> Transaction {
		self.ops.push(Box::new(CreateDirectory::new(path)));
		self
	}

	/// Adds a [AppendFile](struct.AppendFile.html) operation to the transaction
	pub fn append_file<S: Into<String>>(mut self, source: S, temp_dir: S, data: Vec<u8>) -> Transaction {
		self.ops.push(Box::new(AppendFile::new(source, temp_dir, data)));
		self
	}

	/// Adds a [CopyFile](struct.CopyFile.html) operation to the transaction
	pub fn copy_file<S: Into<String>>(mut self, source: S, dest: S) -> Transaction {
		self.ops.push(Box::new(CopyFile::new(source, dest)));
		self
	}

	/// Adds a [CopyDirectory](struct.CopyDirectory.html) operation to the transaction
	pub fn copy_dir<S: Into<String>>(mut self, source: S, dest: S, temp_dir: S) -> Transaction {
		self.ops.push(Box::new(CopyDirectory::new(source, dest, temp_dir)));
		self
	}

	/// Adds a [DeleteFile](struct.DeleteFile.html) operation to the transaction
	pub fn delete_file<S: Into<String>>(mut self, source: S, temp_dir: S) -> Transaction {
		self.ops.push(Box::new(DeleteFile::new(source, temp_dir)));
		self
	}

	/// Adds a [DeleteDirectory](struct.DeleteDirectory.html) operation to the transaction
	pub fn delete_dir<S: Into<String>>(mut self, source: S, temp_dir: S) -> Transaction {
		self.ops.push(Box::new(DeleteDirectory::new(source, temp_dir)));
		self
	}

	/// Adds a [MoveFile](type.MoveFile.html) operation to the transaction
	pub fn move_file<S: Into<String>>(mut self, source: S, dest: S) -> Transaction {
		self.ops.push(Box::new(MoveFile::new(source, dest)));
		self
	}

	/// Adds a [MoveDirectory](type.MoveDirectory.html) operation to the transaction
	pub fn move_dir<S: Into<String>>(mut self, source: S, dest: S) -> Transaction {
		self.ops.push(Box::new(MoveDirectory::new(source, dest)));
		self
	}

	/// Adds a [WriteFile](struct.WriteFile.html) operation to the transaction
	pub fn write_file<S: Into<String>>(mut self, source: S, temp_dir: S, data: Vec<u8>) -> Transaction {
		self.ops.push(Box::new(WriteFile::new(source, temp_dir, data)));
		self
	}
}

impl RollbackableOperation for Transaction {
	/// Executes the transaction
	fn execute(&mut self) -> io::Result<()> {
		for i in 0..self.ops.len() {
			self.execution_count += 1;
			if let Err(e) = self.ops[i].execute() {
				return Err(e);
			}
		}

		Ok(())
	}

	/// Performs rollback on the transaction
	///
	/// Only the operations that were executed will be rollbacked
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