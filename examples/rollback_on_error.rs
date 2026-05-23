//! Demonstrates automatic rollback after a mid-transaction failure.
//!
//! The transaction modifies two files, then tries to move a file that does not
//! exist. The move fails, rollback fires, and both files are restored exactly
//! to their original contents.

use std::fs;
use tfio::{RollbackableOperation, Transaction};

fn main() -> std::io::Result<()> {
    let tmp = tempfile::tempdir()?;
    let d = tmp.path();

    // Two pre-existing files with known content.
    fs::write(d.join("important.txt"), b"original important data")?;
    fs::write(d.join("log.txt"), b"original log entries")?;

    println!("── before ──────────────────────────────────────");
    println!("  important.txt : {:?}", read(d, "important.txt"));
    println!("  log.txt       : {:?}", read(d, "log.txt"));

    // The third operation references a file that doesn't exist — it will fail.
    let mut tr = Transaction::with_temp_dir(d)
        .write_file(d.join("important.txt"), b"NEW important data".to_vec())
        .write_file(d.join("log.txt"), b"NEW log entries".to_vec())
        .move_file(d.join("ghost.txt"), d.join("dest.txt")); // <── will fail

    println!("\n── executing ───────────────────────────────────");
    println!("  op 1: write_file important.txt");
    println!("  op 2: write_file log.txt");
    println!("  op 3: move_file ghost.txt → dest.txt  (ghost.txt does not exist)");

    match tr.execute() {
        Ok(()) => println!("  [ok] — unexpected success"),
        Err(e) => {
            println!("  [err] op 3 failed: {e}");
            println!("\n── rolling back ────────────────────────────────");
            tr.rollback()?;
            println!("  rollback complete");
        }
    }

    println!("\n── after rollback ──────────────────────────────");
    println!("  important.txt : {:?}", read(d, "important.txt"));
    println!("  log.txt       : {:?}", read(d, "log.txt"));
    println!("  dest.txt exists: {}", d.join("dest.txt").exists());

    assert_eq!(read(d, "important.txt"), "original important data");
    assert_eq!(read(d, "log.txt"), "original log entries");
    println!("\n  ✓ both files restored to original content");

    Ok(())
}

fn read(base: &std::path::Path, name: &str) -> String {
    fs::read_to_string(base.join(name)).unwrap_or_else(|_| "<missing>".into())
}
