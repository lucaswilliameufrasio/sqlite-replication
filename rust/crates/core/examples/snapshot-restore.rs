use std::env;
use std::error::Error;
use std::io;
use std::process;

use sqlite_replication_core::{restore_if_missing, snapshot};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let database_path = arguments.next().ok_or_else(usage_error)?;
    let repository_path = arguments.next().ok_or_else(usage_error)?;
    let restore_path = arguments.next().ok_or_else(usage_error)?;
    if arguments.next().is_some() {
        return Err(usage_error().into());
    }

    let ltx_path = snapshot(&database_path, &repository_path)?;
    restore_if_missing(&repository_path, &restore_path)?;
    println!(
        "published {} and restored {}",
        ltx_path.display(),
        restore_path.to_string_lossy()
    );
    Ok(())
}

fn usage_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "usage: snapshot-restore DATABASE REPOSITORY RESTORE_DESTINATION",
    )
}
