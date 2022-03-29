use arloader::{commands::*, error::Error, status::OutputFormat, Arweave};
use rayon::prelude::*;
use std::env;
use std::{fs, path::PathBuf, str::FromStr, time::Instant};
use tempdir::TempDir;
use url::Url;

// For smaller sample sizes, you may have to increase this to have the transactions mined.
const REWARD_MULTIPLIER: f32 = 2.0;
const NUM_FILES: usize = 1;
const FILE_SIZE: usize = 300_000_000;
const BUFFER: usize = 5;

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
    println!("Setting up files...\n");
    let temp_dir = files_setup(FILE_SIZE, NUM_FILES, ext)?;
    let paths_iter = (0..NUM_FILES).map(|i| temp_dir.path().join(format!("{}.bin", i)));
    // let path_chunks = arweave.chunk_file_paths(paths_iter, BUNDLE_SIZE)?;
    let log_dir = temp_dir.path().join("status/");
    fs::create_dir(log_dir.clone()).unwrap();
    let output_format = &OutputFormat::Display;

    let start = Instant::now();
    if sol_keypair_path.is_none() {
        println!("Starting upload with AR...\n");
        command_upload(
            &arweave,
            paths_iter,
            Some(log_dir.clone()),
            None,
            REWARD_MULTIPLIER,
            output_format,
            BUFFER,
        )
        .await?;
    } else {
        println!("Starting upload with SOL...\n");
        command_upload_with_sol(
            &arweave,
            paths_iter,
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

    let paths_iter = (0..NUM_FILES).map(|i| temp_dir.path().join(format!("{}.bin", i)));
    command_update_statuses(&arweave, paths_iter, log_dir, output_format, 10).await?;
    Ok(())
}

fn files_setup(file_size: usize, num_files: usize, ext: &str) -> Result<TempDir, Error> {
    let bytes = vec![0; file_size];

    let temp_dir = TempDir::new("tmp")?;

    let _ = (0..num_files).into_par_iter().for_each(|i| {
        fs::write(
            temp_dir.path().join(format!("{}", i)).with_extension(ext),
            &bytes,
        )
        .unwrap();
    });
    Ok(temp_dir)
}
