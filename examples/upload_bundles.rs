use arloader::{commands::*, error::Error, status::OutputFormat, Arweave};
use rand::Rng;
use rayon::prelude::*;
use std::env;
use std::{fs, path::PathBuf, str::FromStr, time::Instant};
use tempdir::TempDir;
use url::Url;

// For smaller sample sizes, you may have to increase this to have the transactions mined.
const REWARD_MULTIPLIER: f32 = 2.0;
const NUM_FILES: usize = 10;
const FILE_SIZE: usize = 10_000_000;
const BUNDLE_SIZE: u64 = 200_000_000;
const BUFFER: usize = 100;

#[tokio::main]
async fn main() -> CommandResult {
    let ar_keypair_path = env::var("AR_KEYPAIR_PATH").ok().map(PathBuf::from);
    let sol_keypair_path = env::var("SOL_KEYPAIR_PATH").ok().map(PathBuf::from);

    let arweave = if let Some(ar_keypair_path) = ar_keypair_path {
        Arweave::from_keypair_path(
            ar_keypair_path,
            Url::from_str("https://arweave.net").unwrap(),
        )
        .await?
    } else {
        if sol_keypair_path.is_none() {
            println!("Example requires either AR_KEYPAIR_PATH or SOL_KEYPAIR_PATH environment variable to be set.");
            return Ok(());
        };
        Arweave::default()
    };

    let ext = "bin";
    let temp_dir = files_setup(FILE_SIZE, NUM_FILES, ext)?;
    let paths_iter = (0..NUM_FILES).map(|i| temp_dir.path().join(format!("{}.bin", i)));
    let path_chunks = arweave.chunk_file_paths(paths_iter, BUNDLE_SIZE)?;
    let log_dir = temp_dir.path().join("status/");
    fs::create_dir(log_dir.clone()).unwrap();
    let output_format = &OutputFormat::Display;

    let start = Instant::now();
    if sol_keypair_path.is_none() {
        command_upload_bundles(
            &arweave,
            path_chunks,
            Some(log_dir.clone()),
            None,
            REWARD_MULTIPLIER,
            output_format,
            BUFFER,
        )
        .await?;
    } else {
        command_upload_bundles_with_sol(
            &arweave,
            path_chunks,
            Some(log_dir.clone()),
            None,
            REWARD_MULTIPLIER,
            output_format,
            BUFFER,
            sol_keypair_path.unwrap(),
        )
        .await?;
    }

    let duration = start.elapsed();

    println!(
        "\n\nUpload completed in: {:?}\n\nUpdating statuses..\n\n",
        duration
    );

    command_update_bundle_statuses(&arweave, log_dir, output_format, 10).await?;
    Ok(())
}

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
