<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/MovAh13h/tfio/master/assets/tfio-header-a-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/MovAh13h/tfio/master/assets/tfio-header-a-light.svg">
  <img alt="TFIO - Transactional File I/O" src="https://raw.githubusercontent.com/MovAh13h/tfio/master/assets/tfio-header-a-light.svg">
</picture>

![tfio](https://github.com/MovAh13h/tfio/workflows/tfio/badge.svg)

**TFIO** is a library that provides a Transaction-like interface traditionally used in databases, applied to FileIO operations. It gives the flexibility to execute and rollback singular operations as well as transactions on the fly. The library also provides a builder-pattern interface to chain operations and execute them in one go.

## Features

1. 100% safe code (thanks to [Rust](https://www.rust-lang.org/))
2. 11 rollback-able File/Directory operations
3. Transparent cross-filesystem move fallback
4. Pre-existing destinations backed up and restored on rollback
5. All `Errors` exposed for handling
6. 100% tests passing
7. Optional async support via the `tokio` feature flag

**Minimum Supported Rust Version (MSRV):** 1.85

## Usage

Import the library in your Rust project:

```toml
[dependencies]
tfio = "0.3"
```

Create a transaction and execute it. If any `Error` is encountered, rollback the entire transaction:

```rust
use std::io;
use tfio::*;

fn main() -> io::Result<()> {
    let mut tr = Transaction::new()
        .create_file("./foo.txt")
        .create_dir("./bar")
        .write_file("./foo.txt", b"Hello World".to_vec())
        .move_file("./foo.txt", "./bar/foo.txt")
        .append_file("./bar/foo.txt", b"dlroW olleH".to_vec());

    if let Err(e) = tr.execute() {
        eprintln!("Error during execution: {}", e);

        if let Err(ee) = tr.rollback() {
            panic!("Error during transaction rollback: {}", ee);
        }
    }
    Ok(())
}
```

`Transaction::new()` uses the OS temp directory for backups. To specify a custom backup directory:

```rust
let mut tr = Transaction::with_temp_dir("./my_tmp")
    .delete_file("./important.txt")
    .delete_dir("./logs");
```

You can also use single operations directly:

```rust
use std::io;
use tfio::{CopyFile, RollbackableOperation};

fn main() -> io::Result<()> {
    let mut op = CopyFile::new("./foo.txt", "./bar/baz/foo.txt");

    if let Err(e) = op.execute() {
        eprintln!("Error during execution: {}", e);

        if let Err(ee) = op.rollback() {
            panic!("Error during rollback: {}", ee);
        }
    }
    Ok(())
}
```

## Async Support

Enable the `tokio` feature in `Cargo.toml`:

```toml
[dependencies]
tfio = { version = "0.3", features = ["tokio"] }
```

Then use `AsyncTransaction` which mirrors the sync API:

```rust
use tfio::AsyncTransaction;

#[tokio::main]
async fn main() {
    let mut tr = AsyncTransaction::new()
        .create_file("./foo.txt")
        .write_file("./foo.txt", b"Hello".to_vec())
        .touch_file("./bar.txt");

    if let Err(e) = tr.execute().await {
        eprintln!("Error: {}", e);
        tr.rollback().await.expect("Rollback failed");
    }
}
```

## Available Operations

| Operation | Sync | Async |
|-----------|------|-------|
| `CreateFile` | ✓ | ✓ |
| `CreateDirectory` | ✓ | ✓ |
| `DeleteFile` | ✓ | ✓ |
| `DeleteDirectory` | ✓ | ✓ |
| `CopyFile` | ✓ | ✓ |
| `CopyDirectory` | ✓ | ✓ |
| `MoveFile` | ✓ | ✓ |
| `MoveDirectory` | ✓ | ✓ |
| `WriteFile` | ✓ | ✓ |
| `AppendFile` | ✓ | ✓ |
| `TouchFile` | ✓ | ✓ |

Move operations automatically fall back to copy-then-delete when source and destination are on different filesystems. Copy and delete operations store a backup in `temp_dir` and restore the original on rollback.

## Notes

- `CreateFile` fails if the target file already exists (use `WriteFile` to overwrite).
- `rollback()` is safe to call even if `execute()` was never called or failed partway through.
- All operations that require a backup directory default to the OS temp dir. Use the `::with_temp_dir(...)` constructor to specify a custom location (e.g. same filesystem as the target for atomic moves).
- Async types (`AsyncTransaction`, `AsyncCopyFile`, etc.) are re-exported directly from the crate root under the `tokio` feature — no need to import from `tfio::async_::*`.

## Running Tests

```shell
git clone https://github.com/MovAh13h/tfio
cd tfio
cargo test                          # sync tests
cargo test --features tokio         # sync + async tests
```
