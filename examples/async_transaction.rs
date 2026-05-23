//! Async version of the transaction example — requires the `tokio` feature.
//!
//! Run with:
//!   cargo run --example async_transaction --features tokio

use std::fs;
use tfio::AsyncTransaction;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let tmp = tempfile::tempdir()?;
    let work = tmp.path().join("work");
    let backups = tmp.path().join("backups");
    fs::create_dir_all(&work)?;

    fs::write(work.join("template.txt"), b"[template]")?;

    println!("── before ──────────────────────────────────────");
    println!("  template.txt exists: {}", work.join("template.txt").exists());
    println!("  deploy/ exists     : {}", work.join("deploy").exists());

    let mut tr = AsyncTransaction::with_temp_dir(&backups)
        .create_dir(work.join("deploy"))
        .copy_file(work.join("template.txt"), work.join("deploy/config.txt"))
        .write_file(work.join("deploy/config.txt"), b"host=localhost\nport=8080\n".to_vec())
        .create_file(work.join("deploy/pid"))
        .append_file(work.join("deploy/pid"), b"12345".to_vec())
        .move_file(work.join("template.txt"), work.join("deploy/template.txt"));

    tr.execute().await?;

    println!("\n── after execute ───────────────────────────────");
    println!("  template.txt exists       : {}", work.join("template.txt").exists());
    println!("  deploy/config.txt         : {:?}", fs::read_to_string(work.join("deploy/config.txt"))?);
    println!("  deploy/pid                : {:?}", fs::read_to_string(work.join("deploy/pid"))?);
    println!("  deploy/template.txt exists: {}", work.join("deploy/template.txt").exists());

    tr.rollback().await?;

    println!("\n── after rollback ──────────────────────────────");
    println!("  template.txt exists: {}", work.join("template.txt").exists());
    println!("  deploy/ exists     : {}", work.join("deploy").exists());

    Ok(())
}
