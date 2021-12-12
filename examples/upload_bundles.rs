use arloader::{commands::*, error::Error, Arweave};
use rand::Rng;
use rayon::prelude::*;
use std::env;
use std::str::FromStr;
use std::{fs, path::PathBuf};
use tempdir::TempDir;
use url::Url;

fn files_setup(file_size: usize, num_files: usize, ext: &str) -> Result<TempDir, Error> {
    let mut rng = rand::thread_rng();
    let mut bytes = Vec::with_capacity(file_size);
    (0..file_size).for_each(|_| bytes.push(rng.gen()));

    let temp_dir = TempDir::new("test_files")?;

    let _ = (0..num_files).into_par_iter().for_each(|i| {
        fs::write(
            temp_dir.path().join(format!("{}", i)).with_extension(ext),
            &bytes,
        )
        .unwrap();
    });
    Ok(temp_dir)
}

#[tokio::main]
async fn main() -> CommandResult {
    let ar_keypair_path = env::var("AR_KEYPAIR_PATH");

    if ar_keypair_path.is_err() {
        println!("Example requires AR_KEYPAIR_PATH environment variable to be set.");
        return Ok(());
    };
    let ar_keypair_path = ar_keypair_path.unwrap();

    let ext = "bin";
    let temp_dir = files_setup(10_000_000, 20, ext)?;
    let log_dir = temp_dir.path().join("status");
    fs::create_dir(log_dir.clone()).unwrap();

    let arweave = Arweave::from_keypair_path_sync(
        PathBuf::from(ar_keypair_path),
        Url::from_str("https://arweave.net/")?,
    )?;

    let glob_str = format!("{}/*.{}", temp_dir.path().display().to_string(), ext);
    let log_dir_str = log_dir.display().to_string();

    command_upload_bundles(
        &arweave,
        &glob_str,
        Some(log_dir_str.clone()),
        None,
        100_000_000,
        2.0,
        None,
        100,
    )
    .await?;

    command_update_bundle_statuses(&arweave, &log_dir_str, None, 10).await?;
    Ok(())
}
