//! Functions for Cli commands composed from library functions.

use crate::{
    error::Error,
    file_stem_is_valid_txid,
    solana::{FLOOR, SOL_AR_BASE_URL},
    status::{OutputFormat, StatusCode},
    transaction::{Base64, Tag},
    update_bundle_statuses_stream, update_statuses_stream, upload_bundles_stream,
    upload_bundles_stream_with_sol, upload_files_stream, upload_files_with_sol_stream, Arweave,
    PathsChunk, BLOCK_SIZE, WINSTONS_PER_AR,
};

use futures::StreamExt;
use glob::glob;
use num_traits::cast::ToPrimitive;
use solana_sdk::signer::keypair;
use std::{path::PathBuf, str::FromStr};
use tokio::{
    fs,
    time::{sleep, Duration},
};
use url::Url;

pub type CommandResult = Result<(), Error>;

/// Gets cost of uploading a list of files.
pub async fn command_chunk_files(glob_str: &str, num_chunks: usize) -> CommandResult {
    let paths_iter = glob(glob_str)?.filter_map(Result::ok);
    let paths: Vec<PathBuf> = paths_iter.collect();
    let paths_chunks = paths.chunks(num_chunks);

    for (i, paths) in paths_chunks.enumerate() {
        let new_parent = paths[0]
            .clone()
            .parent()
            .unwrap()
            .join(format!("temp_{}", i));
        fs::create_dir(new_parent.clone()).await?;
        for p in paths {
            fs::copy(p.clone(), new_parent.join(p.clone().file_stem().unwrap())).await?;
        }
    }
    Ok(())
}

/// Gets cost of uploading a list of files.
pub async fn command_get_cost(
    arweave: &Arweave,
    glob_str: &str,
    reward_mult: f32,
    with_sol: bool,
    bundle_size: u64,
    no_bundle: bool,
) -> CommandResult {
    let paths_iter = glob(glob_str)?.filter_map(Result::ok);
    let (base, incremental) = arweave.get_price_terms(reward_mult).await?;
    let (_, usd_per_ar, usd_per_sol) = arweave.get_price(&1).await?;

    let units = match with_sol {
        true => "lamports",
        false => "winstons",
    };

    let (num_trans, num_files, cost, bytes) = if no_bundle {
        paths_iter.fold((0, 0, 0, 0), |(n_t, n_f, c, b), p| {
            let data_len = p.metadata().unwrap().len();
            (
                n_t + 1,
                n_f + 1,
                c + {
                    let blocks_len = data_len / BLOCK_SIZE + (data_len % BLOCK_SIZE != 0) as u64;
                    match with_sol {
                        true => {
                            std::cmp::max((base + incremental * (blocks_len - 1)) * 0, FLOOR) + 5000
                        }
                        false => base + incremental * (blocks_len - 1),
                    }
                },
                b + data_len,
            )
        })
    } else {
        let path_chunks = arweave.chunk_file_paths(paths_iter, bundle_size)?;
        path_chunks.iter().fold(
            (0, 0, 0, 0),
            |(n_t, n_f, c, b), PathsChunk(paths, data_len)| {
                (
                    n_t + 1,
                    n_f + paths.len(),
                    c + {
                        let blocks_len =
                            data_len / BLOCK_SIZE + (data_len % BLOCK_SIZE != 0) as u64;
                        match with_sol {
                            true => {
                                std::cmp::max((base + incremental * (blocks_len - 1)) * 0, FLOOR)
                                    + 5000
                            }
                            false => base + incremental * (blocks_len - 1),
                        }
                    },
                    b + data_len,
                )
            },
        )
    };
    if num_files == 0 {
        println!("No files matched glob.");
    } else {
        // get usd cost based on calculated cost
        let usd_cost = match with_sol {
            true => (&cost * &usd_per_sol).to_f32().unwrap() / 1e11_f32,
            false => (&cost * &usd_per_ar).to_f32().unwrap() / 1e14_f32,
        };

        println!(
            "The price to upload {} files with {} total bytes in {} transaction(s) is {} {} (${:.4}).",
            num_files, bytes, num_trans, cost, units, usd_cost
        );
    }
    Ok(())
}

/// Displays pending transaction count every second for one minute.
pub async fn command_get_pending_count(arweave: &Arweave) -> CommandResult {
    println!(" {}\n{:-<84}", "pending tx", "");

    let mut counter = 0;
    while counter < 60 {
        sleep(Duration::from_secs(1)).await;
        let count = arweave.get_pending_count().await?;
        println!(
            "{:>5} {} {}",
            count,
            124u8 as char,
            std::iter::repeat('\u{25A5}')
                .take(count / 50 + 1)
                .collect::<String>()
        );
        counter += 1;
    }
    Ok(())
}

/// Gets status from the network for the provided transaction id.
pub async fn command_get_status(arweave: &Arweave, id: &str, output_format: &str) -> CommandResult {
    let id = Base64::from_str(id)?;
    let output_format = get_output_format(output_format);
    let status = arweave.get_status(&id).await?;
    println!(
        "{}",
        status
            .header_string(&output_format)
            .split_at(32)
            .1
            .split_at(132)
            .0
    );
    print!("{}", output_format.formatted_string(&status).split_at(32).1);
    Ok(())
}

/// Retrieves transaction from the network.
pub async fn command_get_transaction(arweave: &Arweave, id: &str) -> CommandResult {
    let id = Base64::from_str(id)?;
    let transaction = arweave.get_transaction(&id).await?;
    println!("Fetched transaction {}", transaction.id);
    Ok(())
}

/// Lists transaction statuses, filtered by statuses and max confirmations if provided.
pub async fn command_list_statuses(
    arweave: &Arweave,
    glob_str: &str,
    log_dir: &str,
    statuses: Option<Vec<StatusCode>>,
    max_confirms: Option<&str>,
    output_format: Option<&str>,
) -> CommandResult {
    let paths_iter = glob(glob_str)?.filter_map(Result::ok);
    let log_dir = PathBuf::from(log_dir);
    let output_format = get_output_format(output_format.unwrap_or(""));
    let max_confirms = max_confirms.map(|m| m.parse::<u64>().unwrap());

    let mut counter = 0;
    for status in arweave
        .filter_statuses(paths_iter, log_dir.clone(), statuses, max_confirms)
        .await?
        .iter()
    {
        if counter == 0 {
            println!("{}", status.header_string(&output_format));
        }
        print!("{}", output_format.formatted_string(status));
        counter += 1;
    }
    if counter == 0 {
        println!("Didn't find match any statuses.");
    } else {
        println!("Found {} files matching filter criteria.", counter);
    }
    Ok(())
}

/// Prints a count of transactions by status.
pub async fn command_status_report(
    arweave: &Arweave,
    glob_str: &str,
    log_dir: &str,
) -> CommandResult {
    let paths_iter = glob(glob_str)?.filter_map(Result::ok);
    let log_dir = PathBuf::from(log_dir);

    let summary = arweave.status_summary(paths_iter, log_dir).await?;

    println!("{}", summary);

    Ok(())
}

/// Updates bundle statuses for provided files in provided directory.
pub async fn command_update_bundle_statuses(
    arweave: &Arweave,
    log_dir: &str,
    output_format: Option<&str>,
    buffer: usize,
) -> CommandResult {
    let paths_iter = glob(&format!("{}/*.json", log_dir))?
        .filter_map(Result::ok)
        .filter(|p| file_stem_is_valid_txid(p));
    let output_format = get_output_format(output_format.unwrap_or(""));

    let mut stream = update_bundle_statuses_stream(arweave, paths_iter, buffer);

    let mut counter = 0;
    while let Some(Ok(status)) = stream.next().await {
        if counter == 0 {
            println!("{}", status.header_string(&output_format));
        }
        print!("{}", output_format.formatted_string(&status));
        counter += 1;
    }
    if counter == 0 {
        println!("The `log_dir`you provided didn't have any statuses in it.");
    } else {
        println!("Updated {} statuses.", counter);
    }

    Ok(())
}

/// Updates NFT metadata files from a manifest file.
pub async fn command_update_metadata(
    arweave: &Arweave,
    glob_str: &str,
    manifest_str: &str,
    link_file: bool,
) -> CommandResult {
    let paths_iter = glob(glob_str)?.filter_map(Result::ok);
    let num_paths: usize = paths_iter.collect::<Vec<PathBuf>>().len();
    let manifest_path = PathBuf::from(manifest_str);

    arweave
        .update_metadata(
            glob(glob_str)?.filter_map(Result::ok),
            manifest_path,
            link_file,
        )
        .await?;

    println!("Successfully updated {} metadata files.", num_paths);
    Ok(())
}

/// Updates statuses for provided files in provided directory.
pub async fn command_update_statuses(
    arweave: &Arweave,
    glob_str: &str,
    log_dir: &str,
    output_format: Option<&str>,
    buffer: usize,
) -> CommandResult {
    let paths_iter = glob(glob_str)?.filter_map(Result::ok);
    let log_dir = PathBuf::from(log_dir);
    let output_format = get_output_format(output_format.unwrap_or(""));

    let mut stream = update_statuses_stream(arweave, paths_iter, log_dir.clone(), buffer);

    let mut counter = 0;
    while let Some(Ok(status)) = stream.next().await {
        if counter == 0 {
            println!("{}", status.header_string(&output_format));
        }
        print!("{}", output_format.formatted_string(&status));
        counter += 1;
    }
    if counter == 0 {
        println!("The `glob` and `log_dir` combination you provided didn't return any statuses.");
    } else {
        println!("Updated {} statuses.", counter);
    }

    Ok(())
}

/// Uploads files to Arweave.
pub async fn command_upload(
    arweave: &Arweave,
    glob_str: &str,
    log_dir: Option<String>,
    _tags: Option<Vec<Tag<Base64>>>,
    reward_mult: f32,
    output_format: Option<&str>,
    buffer: usize,
) -> CommandResult {
    let paths_iter = glob(glob_str)?.filter_map(Result::ok);
    let log_dir = log_dir.map(|s| PathBuf::from(s));
    let output_format = get_output_format(output_format.unwrap_or(""));
    let price_terms = arweave.get_price_terms(reward_mult).await?;

    let mut stream = upload_files_stream(
        arweave,
        paths_iter,
        log_dir.clone(),
        None,
        price_terms,
        buffer,
    );

    let mut counter = 0;
    while let Some(result) = stream.next().await {
        match result {
            Ok(status) => {
                if counter == 0 {
                    if let Some(log_dir) = &log_dir {
                        println!("Logging statuses to {}", &log_dir.display());
                    }
                    println!("{}", status.header_string(&output_format));
                }
                print!("{}", output_format.formatted_string(&status));
                counter += 1;
            }
            Err(e) => println!("{:#?}", e),
        }
    }

    if counter == 0 {
        println!("The pattern \"{}\" didn't match any files.", glob_str);
    } else {
        println!(
            "Uploaded {} files. Run `arloader update-status \"{}\" --log-dir \"{}\"` to confirm transaction(s).",
            counter,
            glob_str,
            &log_dir.unwrap_or(PathBuf::from("")).display()
        );
    }

    Ok(())
}

/// Uploads bundles created from provided glob to Arweave.
pub async fn command_upload_bundles(
    arweave: &Arweave,
    glob_str: &str,
    log_dir: Option<String>,
    tags: Option<Vec<Tag<String>>>,
    bundle_size: u64,
    reward_mult: f32,
    output_format: Option<&str>,
    buffer: usize,
) -> CommandResult {
    let paths_iter = glob(glob_str)?.filter_map(Result::ok);
    let log_dir = log_dir.map(|s| PathBuf::from(s)).unwrap();
    let output_format = get_output_format(output_format.unwrap_or(""));
    let tags = tags.unwrap_or(Vec::new());
    let price_terms = arweave.get_price_terms(reward_mult).await?;
    let path_chunks = arweave.chunk_file_paths(paths_iter, bundle_size)?;

    if path_chunks.len() == 0 {
        println!("The pattern \"{}\" didn't match any files.", glob_str);
        return Ok(());
    } else {
        let mut stream = upload_bundles_stream(arweave, path_chunks, tags, price_terms, buffer);

        let mut counter = 0;
        let mut number_of_files = 0;
        let mut data_size = 0;
        while let Some(result) = stream.next().await {
            match result {
                Ok(status) => {
                    number_of_files += status.number_of_files;
                    data_size += status.data_size;
                    if counter == 0 {
                        println!("{}", status.header_string(&output_format));
                    }
                    print!("{}", output_format.formatted_string(&status));
                    fs::write(
                        log_dir.join(status.id.to_string()).with_extension("json"),
                        serde_json::to_string(&status)?,
                    )
                    .await?;
                    counter += 1;
                }
                Err(e) => println!("{:#?}", e),
            }
        }

        println!(
            "\nUploaded {} KB in {} files in {} bundle transactions. Run `arloader update-status --log-dir \"{}\"` to update statuses.",
            data_size / 1000,
            number_of_files,
            counter,
            log_dir.display().to_string()
        );
    }
    Ok(())
}

/// Uploads bundles created from provided glob to Arweave, paying with SOL.
pub async fn command_upload_bundles_with_sol(
    arweave: &Arweave,
    glob_str: &str,
    log_dir: Option<String>,
    tags: Option<Vec<Tag<String>>>,
    bundle_size: u64,
    reward_mult: f32,
    output_format: Option<&str>,
    buffer: usize,
    sol_keypair_path: &str,
) -> CommandResult {
    let paths_iter = glob(glob_str)?.filter_map(Result::ok);
    let log_dir = log_dir.map(|s| PathBuf::from(s)).unwrap();
    let output_format = get_output_format(output_format.unwrap_or(""));
    let tags = tags.unwrap_or(Vec::new());
    let price_terms = arweave.get_price_terms(reward_mult).await?;
    let path_chunks = arweave.chunk_file_paths(paths_iter, bundle_size)?;
    let solana_url = "https://api.mainnet-beta.solana.com/".parse::<Url>()?;
    let sol_ar_url = SOL_AR_BASE_URL.parse::<Url>()?.join("sol")?;
    let from_keypair = keypair::read_keypair_file(sol_keypair_path)?;

    if path_chunks.len() == 0 {
        println!("The pattern \"{}\" didn't match any files.", glob_str);
        return Ok(());
    } else {
        let mut stream = upload_bundles_stream_with_sol(
            arweave,
            path_chunks,
            tags,
            price_terms,
            buffer,
            solana_url,
            sol_ar_url,
            &from_keypair,
        );

        let mut counter = 0;
        let mut number_of_files = 0;
        let mut data_size = 0;
        while let Some(result) = stream.next().await {
            match result {
                Ok(status) => {
                    number_of_files += status.number_of_files;
                    data_size += status.data_size;
                    if counter == 0 {
                        println!("{}", status.header_string(&output_format));
                    }
                    print!("{}", output_format.formatted_string(&status));
                    fs::write(
                        log_dir.join(status.id.to_string()).with_extension("json"),
                        serde_json::to_string(&status)?,
                    )
                    .await?;
                    counter += 1;
                }
                Err(e) => println!("{:#?}", e),
            }
        }

        println!(
            "\nUploaded {} KB in {} files in {} bundle transaction(s). Run `arloader update-status --log-dir \"{}\"` to update statuses.",
            data_size / 1000,
            number_of_files,
            counter,
            log_dir.display().to_string()
        );
    }
    Ok(())
}

/// Re-uploads files from status and max confirmations criteria.
pub async fn command_upload_filter(
    arweave: &Arweave,
    glob_str: &str,
    log_dir: &str,
    reward_mult: f32,
    statuses: Option<Vec<StatusCode>>,
    max_confirms: Option<&str>,
    output_format: Option<&str>,
    buffer: Option<&str>,
) -> CommandResult {
    let paths_iter = glob(glob_str)?.filter_map(Result::ok);
    let log_dir = PathBuf::from(log_dir);
    let output_format = get_output_format(output_format.unwrap_or(""));
    let max_confirms = max_confirms.map(|m| m.parse::<u64>().unwrap());
    let buffer = buffer.map(|b| b.parse::<usize>().unwrap()).unwrap_or(1);
    let price_terms = arweave.get_price_terms(reward_mult).await?;

    // Should be refactored to be included in the stream.
    let filtered_paths_iter = arweave
        .filter_statuses(paths_iter, log_dir.clone(), statuses, max_confirms)
        .await?
        .into_iter()
        .filter_map(|f| f.file_path);

    let mut stream = upload_files_stream(
        arweave,
        filtered_paths_iter,
        Some(log_dir.clone()),
        None,
        price_terms,
        buffer,
    );

    let mut counter = 0;
    while let Some(Ok(status)) = stream.next().await {
        if counter == 0 {
            println!("{}", status.header_string(&output_format));
        }
        print!("{}", output_format.formatted_string(&status));
        counter += 1;
    }
    if counter == 0 {
        println!("Didn't find any matching statuses.");
    } else {
        println!(
            "Uploaded {} files. Run `arloader update-status \"{}\" --log-dir {} to confirm transaction(s).",
            counter,
            glob_str,
            &log_dir.display()
        );
    }
    Ok(())
}

/// Creates and uploads manifest from directory of bundle statuses.
pub async fn command_upload_manifest(
    arweave: &Arweave,
    log_dir: &str,
    reward_mult: f32,
    sol_keypair_path: Option<String>,
) -> CommandResult {
    let solana_url = "https://api.mainnet-beta.solana.com/".parse::<Url>()?;
    let sol_ar_url = SOL_AR_BASE_URL.parse::<Url>()?.join("sol")?;
    let from_keypair = sol_keypair_path.map(|s| keypair::read_keypair_file(s).unwrap());

    let price_terms = arweave.get_price_terms(reward_mult).await?;
    let output = arweave
        .upload_manifest_from_bundle_log_dir(
            log_dir,
            price_terms,
            solana_url,
            sol_ar_url,
            from_keypair,
        )
        .await?;

    println!("{}", output);
    Ok(())
}

/// Uploads files to Arweave, paying with SOL.
pub async fn command_upload_with_sol(
    arweave: &Arweave,
    glob_str: &str,
    log_dir: Option<String>,
    _tags: Option<Vec<Tag<Base64>>>,
    reward_mult: f32,
    sol_keypair_path: &str,
    output_format: Option<&str>,
    buffer: usize,
) -> CommandResult {
    let paths_iter = glob(glob_str)?.filter_map(Result::ok);
    let log_dir = log_dir.map(|s| PathBuf::from(s));
    let output_format = get_output_format(output_format.unwrap_or(""));
    let solana_url = "https://api.mainnet-beta.solana.com/".parse::<Url>()?;
    let sol_ar_url = SOL_AR_BASE_URL.parse::<Url>()?.join("sol")?;
    let from_keypair = keypair::read_keypair_file(sol_keypair_path)?;

    let price_terms = arweave.get_price_terms(reward_mult).await?;

    let mut stream = upload_files_with_sol_stream(
        arweave,
        paths_iter,
        log_dir.clone(),
        None,
        price_terms,
        solana_url,
        sol_ar_url,
        &from_keypair,
        buffer,
    );

    let mut counter = 0;
    while let Some(result) = stream.next().await {
        match result {
            Ok(status) => {
                if counter == 0 {
                    if let Some(log_dir) = &log_dir {
                        println!("Logging statuses to {}", &log_dir.display());
                    }
                    println!("{}", status.header_string(&output_format));
                }
                print!("{}", output_format.formatted_string(&status));
                counter += 1;
            }
            Err(e) => println!("{:#?}", e),
        }
    }

    if counter == 0 {
        println!("The pattern \"{}\" didn't match any files.", glob_str);
    } else {
        println!(
            "Uploaded {} files. Run `arloader update-status \"{}\" --log-dir \"{}\"` to confirm transaction(s).",
            counter,
            glob_str,
            &log_dir.unwrap_or(PathBuf::from("")).display()
        );
    }

    Ok(())
}

/// Gets balance for provided wallet address.
pub async fn command_wallet_balance(
    arweave: &Arweave,
    wallet_address: Option<String>,
) -> CommandResult {
    let mb = u64::pow(1024, 2);
    let result = tokio::join!(
        arweave.get_wallet_balance(wallet_address),
        arweave.get_price(&mb)
    );
    let balance = result.0?;
    let (winstons_per_kb, usd_per_ar, _) = result.1?;

    let balance_usd = &balance.to_f32().unwrap() / &WINSTONS_PER_AR.to_f32().unwrap()
        * &usd_per_ar.to_f32().unwrap()
        / 100_f32;

    let usd_per_kb = (&winstons_per_kb * &usd_per_ar).to_f32().unwrap() / 1e14_f32;

    println!(
            "Wallet balance is {} {units} (${balance_usd:.2} at ${ar_price:.2} USD per AR). At the current price of {price} {units} per MB (${usd_price:.4}), you can upload {max} MB of data.",
            &balance,
            units = arweave.units,
            max = &balance / &winstons_per_kb,
            price = &winstons_per_kb,
            balance_usd = balance_usd,
            ar_price = &usd_per_ar.to_f32().unwrap()
            / 100_f32,
            usd_price = usd_per_kb
    );
    Ok(())
}

/// Writes metaplex link items used to create NFTs with candy machine program.
pub async fn command_write_metaplex_items(
    arweave: &Arweave,
    glob_str: &str,
    manifest_str: &str,
    log_dir: &str,
    link_file: bool,
) -> CommandResult {
    let paths_iter = glob(glob_str)?.filter_map(Result::ok);
    let num_paths: usize = paths_iter.collect::<Vec<PathBuf>>().len();
    let manifest_path = PathBuf::from(manifest_str);

    arweave
        .write_metaplex_items(
            glob(glob_str)?.filter_map(Result::ok),
            manifest_path,
            PathBuf::from(log_dir),
            link_file,
        )
        .await?;

    println!(
        "Successfully wrote metaplex items for {} metadata files to {}",
        num_paths, log_dir
    );
    Ok(())
}

/// Maps cli string argument to output format.
pub fn get_output_format(output: &str) -> OutputFormat {
    match output {
        "quiet" => OutputFormat::DisplayQuiet,
        "verbose" => OutputFormat::DisplayVerbose,
        "json" => OutputFormat::Json,
        "json_compact" => OutputFormat::JsonCompact,
        _ => OutputFormat::Display,
    }
}
