# TFIO - Transactional File I/O

![tfio](https://github.com/GandalfTheGrayOfHell/tfio/workflows/tfio/badge.svg)

**TFIO** is a library that provides a Transaction-like interface that are traditionally used in databases on FileIO operations. It gives the flexibility to execute and rollback singlular operations as well as transactions on the fly. The library also provides a builder-pattern interface to chain operations and execute them in one go.

## Features
1) 100% safe code (thanks to [Rust](https://www.rust-lang.org/))
2) 10 rollback-able File/Directory operations
3) Only 1 Third-party dependency
4) All `Errors` exposed for handling
5) 100% Tests passing

## Usage
Import the library in your Rust project:
```rust
use tfio::*;
```

Create a transaction and execute it. If any `Error` is encountered, rollback the entire transaction:
**Note**: The paths should only use forward slashes (/) and can either begin with disks or relative to the present working directory ie. begin with `./`
```rust
use std::io;
use tfio::*;

fn main() -> io::Result<()> {
	let temp_dir = "./PATH_TO_TEMP_DIR";
	let mut tr = Transaction::new()
				.create_file("./foo.txt")
				.create_dir("./bar")
				.write_file("./foo.txt", temp_dir, b"Hello World".to_vec())
				.move_file("./foo.txt", "./bar/foo.txt")
				.append_file("./bar/foo.txt", temp_dir, b"dlroW olleH".to_vec());
	
	// Execute the transaction
	if let Err(e) = tr.execute() {
		eprintln!("Error during execution: {}", e);
		
		// All operations can be reverted in reverse-order if `Error` is encountered
		if let Err(ee) = tr.rollback() {
			panic!("Error during transaction rollback: {}", ee);
		}
	}
	Ok(())
}
```

You can also import single operations to use:
```rust
use std::io;
use tfio::{CopyFile, RollbackableOperation};

fn main() -> io::Result<()> {
	let mut copy_operation = CopyFile::new("./foo.txt", "./bar/baz/foo.txt");
	
	// Execute the operation
	if let Err(e) = copy_operation.execute() {
		eprintln!("Error during execution: {}", e);
		
		// Rollback the operation
		if let Err(ee) = copy_operation.rollback() {
			panic!("Error during rollback: {}", ee);
		}
	}
	Ok(())
}
```

To run tests:
```shell
$ git clone https://github.com/GandalfTheGrayOfHell/tfio
$ cd tfio
$ cargo test
```

**Note:** Some TFIO operations create temporary files and directories in the TEMP_PATH provided. If an operation requires a path to temp dir then it will also require either the `SingleFileOperation` trait or the `DirectoryOperation` trait. Hence import them as per need.

## Roadmap
It is unlikely that this project receives any updates. It is supposed to be a building block for a future project and works just enough to get things done. There are a couple of places where this library could use help:
1) Handling of `Write` buffers
2) Path normalization

PRs are always appreciated :)