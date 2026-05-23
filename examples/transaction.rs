//! Demonstrates a successful transaction: create, write, copy, move, append — then rollback to
//! show the slate is clean.

use std::fs;
use tfio::{RollbackableOperation, Transaction};

fn main() -> std::io::Result<()> {
    let tmp = tempfile::tempdir()?;
    let work = tmp.path().join("work");
    let backups = tmp.path().join("backups");
    fs::create_dir_all(&work)?;

    // Seed one pre-existing file that will be used as a copy source.
    fs::write(work.join("template.txt"), b"[template]")?;

    println!("── before ──────────────────────────────────────");
    print_state(&work);

    // Build the transaction. Backups go in a separate directory so they never
    // appear alongside the working files.
    let mut tr = Transaction::with_temp_dir(&backups)
        .create_dir(work.join("deploy"))
        .copy_file(work.join("template.txt"), work.join("deploy/config.txt"))
        .write_file(work.join("deploy/config.txt"), b"host=localhost\nport=8080\n".to_vec())
        .create_file(work.join("deploy/pid"))
        .append_file(work.join("deploy/pid"), b"12345".to_vec())
        .move_file(work.join("template.txt"), work.join("deploy/template.txt"));

    tr.execute()?;

    println!("\n── after execute ───────────────────────────────");
    print_state(&work);

    tr.rollback()?;

    println!("\n── after rollback ──────────────────────────────");
    print_state(&work);

    Ok(())
}

fn print_state(base: &std::path::Path) {
    for entry in walkdir(base) {
        let rel = entry.strip_prefix(base).unwrap();
        if entry.is_file() {
            let contents = fs::read_to_string(&entry).unwrap_or_else(|_| "<binary>".into());
            println!("  {}: {:?}", rel.display(), contents.trim());
        } else {
            println!("  {}/", rel.display());
        }
    }
    if walkdir(base).is_empty() {
        println!("  (empty)");
    }
}

fn walkdir(base: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut entries = Vec::new();
    collect(base, base, &mut entries);
    entries.sort();
    entries
}

fn collect(root: &std::path::Path, dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.strip_prefix(root).unwrap().as_os_str().is_empty() {
            continue;
        }
        out.push(path.clone());
        if path.is_dir() {
            collect(root, &path, out);
        }
    }
}
