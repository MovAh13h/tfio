#![allow(dead_code, unused)]

mod copy;
mod r#move;
mod delete;
mod write;

use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{PathBuf, Path};

use uuid::Uuid;

pub use copy::*;
pub use r#move::*;
pub use delete::*;

#[cfg(debug_assertions)]
macro_rules! debug {
	($x:expr) => { println!("{}", $x) }
}

#[cfg(not(debug_assertions))]
macro_rules! debug {
	($x:expr) => {}
}


// Create: CreateFile, CreateDirectory
// Update: WriteFile, AppendFile, ?BufWriteFile, ?BufAppendFile
// Delete: DeleteFile, DeleteDirectory
// Copy: CopyFile, CopyDirectory
// Move: MoveFile, MoveDirectory

// MoveFile: SingleFileOperation + RollbackableOperation *
// CopyFile: SingleFileOperation + RollbackableOperation *
// DeleteFile: SingleFileOperation + RollbackableOperation *
// WriteFile: SingleFileOperation + RollbackableOperation
// AppendFile: SingleFileOperation + RollbackableOperation
// BufWriteFile: SingleFileOperation + RollbackableOperation
// BufAppendFile: SingleFileOperation + RollbackableOperation

// CreateDirectory: DirectoryOperation + RollbackableOperation
// MoveDirectory: DirectoryOperation + RollbackableOperation
// CopyDirectory: DirectoryOperation + RollbackableOperation
// DeleteDirectory: DirectoryOperation + RollbackableOperation


pub trait RollbackableOperation {
	fn execute(&mut self) -> io::Result<()>;
	fn rollback(&self) -> io::Result<()>;
}

pub trait SingleFileOperation: RollbackableOperation + Drop {
	// Path to the file
	fn get_path(&self) -> &String;

	// Getters/Setters for backup path
	fn get_backup_path(&self) -> &String;
	fn set_backup_path<S: Into<String>>(&mut self, uuid: S);

	// Path to temp dir
	fn get_temp_dir(&self) -> &String;

	// Ensure temp dir exists
	fn ensure_temp_dir_exists(&self) -> io::Result<()> {
		fs::create_dir_all(&self.get_temp_dir())
	}

	// Dispose off resources used by the operation
	// It is called once all operations in the Transaction are completed successfully
	fn dispose(&self) -> io::Result<()> {
		fs::remove_file(self.get_backup_path())
	}

	// Create a temp file that is just a clone of the source file
	// If backup file is successfully created, method should call `self.set_backup_path`
	fn create_backup_file(&mut self) -> io::Result<()> {
		let uuid = Uuid::new_v4();
		let mut buffer = [b' '; 36];
		
		uuid.to_hyphenated().encode_lower(&mut buffer);

		let uuid_str = String::from_utf8(buffer.to_vec()).expect(format!("Could not convert buffer to String").as_str());
		let backup_path = Path::new(&self.get_temp_dir()).join(uuid_str).to_str().unwrap().to_string();

		let mut buffer = Vec::new();
		let mut dest_file = OpenOptions::new().write(true).create(true).open(&backup_path)?;
		let mut source_file = OpenOptions::new().read(true).open(&self.get_path())?;

		self.set_backup_path(&backup_path);
		source_file.read_to_end(&mut buffer)?;
		dest_file.write_all(&buffer)
	}
}


struct FTransaction {
	ops: Vec<Box<dyn RollbackableOperation>>,
}