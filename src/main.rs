use arloader::{
    error::Error,
    file_stem_is_valid_txid,
    solana::{FLOOR, RATE, SOL_AR_BASE_URL},
    status::{OutputFormat, StatusCode},
    transaction::{Base64, FromUtf8Strs, Tag},
    update_bundle_statuses_stream, update_statuses_stream, upload_bundles_stream,
    upload_bundles_stream_with_sol, upload_files_stream, upload_files_with_sol_stream, Arweave,
    BLOCK_SIZE, WINSTONS_PER_AR,
};
use clap::{
    self, crate_description, crate_name, crate_version, value_t, App, AppSettings, Arg, SubCommand,
    Values,
};
use futures::StreamExt;
use glob::glob;
use num_traits::cast::ToPrimitive;
use solana_sdk::signer::keypair;
use std::{fmt::Display, path::PathBuf, str::FromStr};
use tokio::{
    fs,
    time::{sleep, Duration},
};
use url::Url;

pub type CommandResult = Result<(), Error>;

fn is_parsable_generic<U, T>(string: T) -> Result<(), String>
where
    T: AsRef<str> + Display,
    U: FromStr,
    U::Err: Display,
{
    string
        .as_ref()
        .parse::<U>()
        .map(|_| ())
        .map_err(|err| format!("error parsing '{}': {}", string, err))
}

pub fn is_parsable<T>(string: String) -> Result<(), String>
where
    T: FromStr,
    T::Err: Display,
{
    is_parsable_generic::<T, String>(string)
}

fn is_valid_tag<T>(tag: T) -> Result<(), String>
where
    T: AsRef<str> + Display,
{
    let split: Vec<_> = tag.as_ref().split(":").collect();
    match Tag::<Base64>::from_utf8_strs(split[0], split[1]) {
        Ok(_) => Ok(()),
        Err(_) => Err(format!("Not a valid tag.")),
    }
}

fn is_valid_url(url_str: String) -> Result<(), String> {
    match url_str.parse::<Url>() {
        Ok(_) => match url_str.chars().last() {
            Some(_) => Ok(()),
            None => Err(format!("Url must have trailing slash.")),
        },
        Err(_) => Err(format!("Not a valid url.")),
    }
}

fn is_valid_reward_multiplier(reward_mult: String) -> Result<(), String> {
    match reward_mult.parse::<f32>() {
        Ok(n) => {
            if n > 0. && n < 10. {
                Ok(())
            } else {
                Err(format!("Multiplier must be between 0 and 10."))
            }
        }
        Err(_) => Err(format!("Not a valid multiplier.")),
    }
}

fn is_valid_bundle_size(bundle_size: String) -> Result<(), String> {
    match bundle_size.parse::<u64>() {
        Ok(n) => {
            if n <= 200000000 {
                Ok(())
            } else {
                Err(format!("Bundle data size must not be greater than 200MB."))
            }
        }
        Err(_) => Err(format!("Not a valid bundle size.")),
    }
}

fn is_valid_log_dir(log_dir: String) -> Result<(), String> {
    match log_dir.parse::<PathBuf>() {
        Ok(p) => {
            if p.exists() {
                Ok(())
            } else {
                Err(format!("Directory does not exist."))
            }
        }
        Err(_) => Err(format!("Not a valid directory.")),
    }
}

fn get_tags_vec<T>(tag_values: Option<Values>) -> Option<Vec<T>>
where
    T: FromUtf8Strs<T>,
{
    if let Some(tag_strings) = tag_values {
        let tags = tag_strings
            .into_iter()
            .map(|t| {
                let split: Vec<&str> = t.split(":").collect();
                T::from_utf8_strs(split[0], split[1])
            })
            .flat_map(Result::ok)
            .collect();
        Some(tags)
    } else {
        None
    }
}

fn get_output_format(output: &str) -> OutputFormat {
    match output {
        "quiet" => OutputFormat::DisplayQuiet,
        "verbose" => OutputFormat::DisplayVerbose,
        "json" => OutputFormat::Json,
        "json_compact" => OutputFormat::JsonCompact,
        _ => OutputFormat::Display,
    }
}

fn get_status_code(output: &str) -> StatusCode {
    match output {
        "Submitted" => StatusCode::Submitted,
        "Pending" => StatusCode::Pending,
        "Confirmed" => StatusCode::Confirmed,
        "NotFound" => StatusCode::NotFound,
        _ => StatusCode::NotFound,
    }
}

fn glob_arg<'a, 'b>(required: bool) -> Arg<'a, 'b> {
    Arg::with_name("glob")
        .value_name("GLOB")
        .takes_value(true)
        .required(required)
        .help(
            "Glob pattern of files. \
            PATTERN MUST BE IN \
            QUOTES TO AVOID SHELL EXPANSION.",
        )
}

fn id_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("id")
        .value_name("ID")
        .takes_value(true)
        .required(true)
        .validator(is_parsable::<Base64>)
        .help("Specify the transaction id.")
}

fn log_dir_arg<'a, 'b>(required: bool) -> Arg<'a, 'b> {
    Arg::with_name("log_dir")
        .long("log-dir")
        .value_name("LOG_DIR")
        .takes_value(true)
        .takes_value(required)
        .validator(is_valid_log_dir)
        .help(
            "Directory that status updates will be written to. If not \
        provided, status updates will not be written.",
        )
}

fn max_confirms_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("max_confirms")
        .long("min-confirms")
        .value_name("MAX_CONFIRM")
        .takes_value(true)
        .help("Provide maximum number of confirmations to filter statuses by.")
}

fn statuses_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("statuses")
        .long("statuses")
        .value_name("STATUSES")
        .takes_value(true)
        .multiple(true)
        .possible_values(&["Submitted", "Pending", "Confirmed", "NotFound"])
        .help("Status codes to filter by. Multiple Ok.")
}

fn tags_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("tags")
        .long("tags")
        .value_name("TAGS")
        .multiple(true)
        .takes_value(true)
        .validator(is_valid_tag)
        .help(
            "Specify additional tags for the files as \
        <NAME>:<VALUE>, separated by spaces. Content-Type tag \
        will be inferred automatically so not necessary so \
        include here. Additional tags will be applied
        to all of the uploaded files.",
        )
}

fn reward_multiplier_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("reward_multiplier")
        .long("reward-multiplier")
        .short("rx")
        .value_name("REWARD_MULT")
        .default_value("1.0")
        .validator(is_valid_reward_multiplier)
        .help(
            "Specify a reward multiplier as float. \
        The reward from the network will be multiplied \
        by this amount for submission.",
        )
}

fn with_sol_arg<'a, 'b>(req_sol_key: bool) -> Arg<'a, 'b> {
    let mut arg = Arg::with_name("with_sol")
        .long("with-sol")
        .value_name("WITH_SOL")
        .required(false)
        .takes_value(false)
        .help(
            "Specify whether to pay for transaction(s) \
            with SOL.",
        );
    if req_sol_key {
        arg = arg.requires("sol_keypair_path");
    }
    arg
}

fn sol_keypair_path_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("sol_keypair_path")
        .long("sol-keypair-path")
        .value_name("SOL_KEYPAIR_PATH")
        .required(false)
        .validator(is_parsable::<PathBuf>)
        .env("SOL_KEYPAIR_PATH")
        .help(
            "Path of Solana keypair file to use to pay for transactions. \
        Will use value from SOL_KEYPAIR_PATH environment variable \
        if it exists",
        )
}

fn no_bundle_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("no_bundle")
        .long("no-bundle")
        .short("nb")
        .value_name("NO_BUNDLE")
        .required(false)
        .takes_value(false)
        .help(
            "Specify whether to upload with individual \
            transactions intead of in a bundle.",
        )
}

fn buffer_arg<'a, 'b>(default: &'a str) -> Arg<'a, 'b> {
    Arg::with_name("buffer")
        .long("buffer")
        .value_name("BUFFER")
        .takes_value(true)
        .validator(is_parsable::<usize>)
        .default_value(default)
        .help("Sets the maximum number of concurrent network requests")
}

fn bundle_size_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("bundle_size")
        .long("bundle-size")
        .value_name("BUNDLE_SIZE")
        .takes_value(true)
        .validator(is_valid_bundle_size)
        .default_value("10000000")
        .help("Sets the maximum file data bytes to include in a bundle.")
}

fn get_app() -> App<'static, 'static> {
    let app_matches = App::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg(
            Arg::with_name("base_url")
                .long("base-url")
                .value_name("AR_BASE_URL")
                .validator(is_valid_url)
                .default_value("https://arweave.net/")
                .env("AR_BASE_URL")
                .help(
                    "Base url for network requests. \
                Can also be set with AR_BASE_URL environment \
                variable",
                ),
        )
        .arg(
            Arg::with_name("ar_keypair_path")
                .long("ar-keypair-path")
                .value_name("AR_KEYPAIR_PATH")
                .validator(is_parsable::<PathBuf>)
                .env("AR_KEYPAIR_PATH")
                .required(true)
                .help(
                    "Path of keypair file to use to pay for transactions. \
                Will use value from AR_KEYPAIR_PATH environment variable \
                if it exists",
                ),
        )
        .arg(
            Arg::with_name("output_format")
                .long("output")
                .short("o")
                .value_name("FORMAT")
                .global(true)
                .takes_value(true)
                .possible_values(&["quiet", "verbose", "json", "json-compact"])
                .help("Return information in specified output format."),
        )
        .subcommand(
            SubCommand::with_name("estimate")
                .about(
                    "Prints the estimated cost of uploading file(s) \
                matching provided glob.",
                )
                .arg(glob_arg(true))
                .arg(reward_multiplier_arg())
                .arg(with_sol_arg(false))
                .arg(no_bundle_arg()),
        )
        .subcommand(
            SubCommand::with_name("wallet-balance")
                .about("Prints the balance of an Arweave wallet.")
                .arg(
                    Arg::with_name("wallet_address")
                        .value_name("WALLET_ADDRESS")
                        .takes_value(true)
                        .validator(is_parsable::<Base64>)
                        .help(
                            "Specify the address of the wallet to print \
                            the balance for. Defaults to the keypair
                            specified in `ar_keypair_path`.",
                        ),
                ),
        )
        .subcommand(
            SubCommand::with_name("pending")
                .about("Displays the count of pending transactions in the mempool."),
        )
        .subcommand(
            SubCommand::with_name("get-transaction")
                .about("Gets a transaction from the network and writes to disk as a file.")
                .arg(id_arg()),
        )
        .subcommand(
            SubCommand::with_name("upload")
                .about("Uploads one or more files that match the specified glob.")
                .arg(glob_arg(true))
                .arg(log_dir_arg(true))
                .arg(tags_arg())
                .arg(reward_multiplier_arg())
                .arg(with_sol_arg(true))
                .arg(sol_keypair_path_arg())
                .arg(no_bundle_arg())
                .arg(buffer_arg("5"))
                .arg(bundle_size_arg()),
        )
        .subcommand(
            SubCommand::with_name("get-status")
                .about("Prints the status of a transaction from the network.")
                .arg(id_arg()),
        )
        .subcommand(
            SubCommand::with_name("update-status")
                .about("Updates statuses stored in `log_dir`. Glob arg only used for --no-bundle.")
                .arg(log_dir_arg(true))
                .arg(glob_arg(false))
                .arg(no_bundle_arg())
                .arg(buffer_arg("10")),
        )
        .subcommand(
            SubCommand::with_name("upload-manifest")
                .about("Uploads a manifest for files uploaded in bundles with statuses stored in `log_dir`.")
                .arg(log_dir_arg(true))
                .arg(reward_multiplier_arg())
        )
        .subcommand(
            SubCommand::with_name("status-report")
                .about("Prints a summary of statuses stored in `log_dir`.")
                .arg(glob_arg(true))
                .arg(log_dir_arg(true)),
        )
        .subcommand(
            SubCommand::with_name("upload-filter")
                .about("Re-uploads files that meet filter criteria.")
                .arg(glob_arg(true))
                .arg(log_dir_arg(true))
                .arg(reward_multiplier_arg())
                .arg(statuses_arg())
                .arg(max_confirms_arg()),
        )
        .subcommand(
            SubCommand::with_name("list-status")
                .about("Lists statuses as currently store in `log_dir`.")
                .help("")
                .arg(glob_arg(true))
                .arg(log_dir_arg(true))
                .arg(statuses_arg())
                .arg(max_confirms_arg()),
        );
    app_matches
}

#[tokio::main]
async fn main() -> CommandResult {
    env_logger::init();
    let app_matches = get_app().get_matches();
    let ar_keypair_path = app_matches.value_of("ar_keypair_path").unwrap();
    let base_url = app_matches
        .value_of("base_url")
        .map(Url::from_str)
        .transpose()?;

    let arweave = Arweave::from_keypair_path(PathBuf::from(ar_keypair_path), base_url)
        .await
        .unwrap();

    let (sub_command, arg_matches) = app_matches.subcommand();

    match (sub_command, arg_matches) {
        ("estimate", Some(sub_arg_matches)) => {
            let glob_str = sub_arg_matches.value_of("glob").unwrap();
            let reward_mult = value_t!(sub_arg_matches.value_of("reward_multiplier"), f32).unwrap();
            let with_sol = sub_arg_matches.is_present("with_sol");
            let no_bundle = sub_arg_matches.is_present("no_bundle");
            command_get_cost(&arweave, glob_str, reward_mult, with_sol, no_bundle).await
        }
        ("wallet-balance", Some(sub_arg_matches)) => {
            let wallet_address = sub_arg_matches
                .value_of("wallet_address")
                .map(|v| v.to_string());
            command_wallet_balance(&arweave, wallet_address).await
        }
        ("pending", Some(_)) => command_get_pending_count(&arweave).await,
        ("get-transaction", Some(sub_arg_matches)) => {
            let id = sub_arg_matches.value_of("id").unwrap();
            command_get_transaction(&arweave, id).await
        }
        ("upload", Some(sub_arg_matches)) => {
            let glob_str = sub_arg_matches.value_of("glob").unwrap();
            let log_dir = sub_arg_matches.value_of("log_dir");
            let reward_mult = value_t!(sub_arg_matches.value_of("reward_multiplier"), f32).unwrap();
            let bundle_size = value_t!(sub_arg_matches.value_of("bundle_size"), u64).unwrap();
            let with_sol = sub_arg_matches.is_present("with_sol");
            let no_bundle = sub_arg_matches.is_present("no_bundle");
            let buffer = value_t!(sub_arg_matches.value_of("buffer"), usize).unwrap();
            let output_format = app_matches.value_of("output_format");

            match (with_sol, no_bundle) {
                (false, false) => {
                    command_upload_bundles(
                        &arweave,
                        glob_str,
                        log_dir,
                        get_tags_vec(sub_arg_matches.values_of("tags")),
                        bundle_size,
                        reward_mult,
                        output_format,
                        buffer,
                    )
                    .await
                }
                (false, true) => {
                    command_upload(
                        &arweave,
                        glob_str,
                        log_dir,
                        get_tags_vec(sub_arg_matches.values_of("tags")),
                        reward_mult,
                        output_format,
                        buffer,
                    )
                    .await
                }
                (true, false) => {
                    command_upload_bundles_with_sol(
                        &arweave,
                        glob_str,
                        log_dir,
                        get_tags_vec(sub_arg_matches.values_of("tags")),
                        bundle_size,
                        reward_mult,
                        output_format,
                        buffer,
                        sub_arg_matches.value_of("sol_keypair_path").unwrap(),
                    )
                    .await
                }
                (true, true) => {
                    command_upload_with_sol(
                        &arweave,
                        glob_str,
                        log_dir,
                        get_tags_vec(sub_arg_matches.values_of("tags")),
                        reward_mult,
                        sub_arg_matches.value_of("sol_keypair_path").unwrap(),
                        output_format,
                        buffer,
                    )
                    .await
                }
            }
        }
        ("get-status", Some(sub_arg_matches)) => {
            let id = sub_arg_matches.value_of("id").unwrap();
            let output_format = app_matches.value_of("output_format").unwrap_or("");
            command_get_status(&arweave, id, output_format).await
        }
        ("list-status", Some(sub_arg_matches)) => {
            let glob_str = sub_arg_matches.value_of("glob").unwrap();
            let log_dir = sub_arg_matches.value_of("log_dir").unwrap();

            let statuses = if let Some(values) = sub_arg_matches.values_of("statuses") {
                Some(values.map(get_status_code).collect())
            } else {
                None
            };

            let max_confirms = sub_arg_matches.value_of("max_confirms");
            let output_format = app_matches.value_of("output_format");
            command_list_statuses(
                &arweave,
                glob_str,
                log_dir,
                statuses,
                max_confirms,
                output_format,
            )
            .await
        }
        ("update-status", Some(sub_arg_matches)) => {
            let log_dir = sub_arg_matches.value_of("log_dir").unwrap();
            let glob_str = sub_arg_matches.value_of("glob");
            let no_bundle = sub_arg_matches.is_present("no_bundle");
            let output_format = app_matches.value_of("output_format");
            let buffer = value_t!(sub_arg_matches.value_of("buffer"), usize).unwrap();

            match no_bundle {
                true => {
                    command_update_statuses(
                        &arweave,
                        glob_str.unwrap(),
                        log_dir,
                        output_format,
                        buffer,
                    )
                    .await
                }
                false => {
                    command_update_bundle_statuses(&arweave, log_dir, output_format, buffer).await
                }
            }
        }
        ("status-report", Some(sub_arg_matches)) => {
            let glob_str = sub_arg_matches.value_of("glob").unwrap();
            let log_dir = sub_arg_matches.value_of("log_dir").unwrap();
            command_status_report(&arweave, glob_str, log_dir).await
        }
        ("upload-manifest", Some(sub_arg_matches)) => {
            let log_dir = sub_arg_matches.value_of("log_dir").unwrap();
            let reward_mult = value_t!(sub_arg_matches.value_of("reward_multiplier"), f32).unwrap();
            command_upload_manifest(&arweave, log_dir, reward_mult).await
        }
        ("upload-filter", Some(sub_arg_matches)) => {
            let glob_str = sub_arg_matches.value_of("glob").unwrap();
            let log_dir = sub_arg_matches.value_of("log_dir").unwrap();
            let reward_mult = value_t!(sub_arg_matches.value_of("reward_multiplier"), f32).unwrap();

            let statuses = if let Some(values) = sub_arg_matches.values_of("statuses") {
                Some(values.map(get_status_code).collect())
            } else {
                None
            };

            let max_confirms = sub_arg_matches.value_of("max_confirms");
            let output_format = app_matches.value_of("output_format");
            let buffer = app_matches.value_of("buffer");
            command_upload_filter(
                &arweave,
                glob_str,
                log_dir,
                reward_mult,
                statuses,
                max_confirms,
                output_format,
                buffer,
            )
            .await
        }
        _ => unreachable!(),
    }
}

async fn command_get_cost(
    arweave: &Arweave,
    glob_str: &str,
    reward_mult: f32,
    with_sol: bool,
    no_bundle: bool,
) -> CommandResult {
    let paths_iter = glob(glob_str)?.filter_map(Result::ok);
    let (base, incremental) = arweave.get_price_terms(reward_mult).await?;
    let (_, usd_per_ar, usd_per_sol) = arweave.get_price(&1).await?;

    // set units
    let units = match with_sol {
        true => "lamports",
        false => "winstons",
    };

    // get total number of file and bytes and cost if not bundled
    let (num, mut cost, bytes) = paths_iter.fold((0, 0, 0), |(n, c, b), p| {
        (
            n + 1,
            c + {
                let data_len = p.metadata().unwrap().len();
                let blocks_len = data_len / BLOCK_SIZE + (data_len % BLOCK_SIZE != 0) as u64;
                match with_sol {
                    true => {
                        std::cmp::max((base + incremental * (blocks_len - 1)) / RATE, FLOOR) + 5000
                    }
                    false => base + incremental * (blocks_len - 1),
                }
            },
            b + p.metadata().unwrap().len(),
        )
    });

    if num == 0 {
        println!("No files matched glob.");
    } else {
        // adjust cost if bundling
        if !no_bundle {
            let blocks_len = bytes / BLOCK_SIZE + (bytes % BLOCK_SIZE != 0) as u64;
            match with_sol {
                true => {
                    cost =
                        std::cmp::max((base + incremental * (blocks_len - 1)) / RATE, FLOOR) + 5000;
                }
                false => {
                    cost = base + incremental * (blocks_len - 1);
                }
            }
        }

        // get usd cost based on calculated cost
        let usd_cost = match with_sol {
            true => (&cost * &usd_per_sol).to_f32().unwrap() / 1e11_f32,
            false => (&cost * &usd_per_ar).to_f32().unwrap() / 1e14_f32,
        };

        println!(
            "The price to upload {} files with {} total bytes is {} {} (${:.4}).",
            num, bytes, cost, units, usd_cost
        );
    }
    Ok(())
}

async fn command_get_transaction(arweave: &Arweave, id: &str) -> CommandResult {
    let id = Base64::from_str(id)?;
    let transaction = arweave.get_transaction(&id).await?;
    println!("Fetched transaction {}", transaction.id);
    Ok(())
}

async fn command_get_status(arweave: &Arweave, id: &str, output_format: &str) -> CommandResult {
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

async fn command_wallet_balance(
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

async fn command_get_pending_count(arweave: &Arweave) -> CommandResult {
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

async fn command_upload(
    arweave: &Arweave,
    glob_str: &str,
    log_dir: Option<&str>,
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

async fn command_upload_bundles(
    arweave: &Arweave,
    glob_str: &str,
    log_dir: Option<&str>,
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

async fn command_upload_with_sol(
    arweave: &Arweave,
    glob_str: &str,
    log_dir: Option<&str>,
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

async fn command_upload_bundles_with_sol(
    arweave: &Arweave,
    glob_str: &str,
    log_dir: Option<&str>,
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

async fn command_list_statuses(
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

async fn command_update_statuses(
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

async fn command_update_bundle_statuses(
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

async fn command_status_report(arweave: &Arweave, glob_str: &str, log_dir: &str) -> CommandResult {
    let paths_iter = glob(glob_str)?.filter_map(Result::ok);
    let log_dir = PathBuf::from(log_dir);

    let summary = arweave.status_summary(paths_iter, log_dir).await?;

    println!("{}", summary);

    Ok(())
}

async fn command_upload_filter(
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

async fn command_upload_manifest(
    arweave: &Arweave,
    log_dir: &str,
    reward_mult: f32,
) -> CommandResult {
    let price_terms = arweave.get_price_terms(reward_mult).await?;
    let output = arweave
        .upload_manifest_from_log_dir(log_dir, price_terms)
        .await?;

    println!("{}", output);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::get_app;
    use clap::{value_t, ErrorKind};
    use std::env;

    #[test]
    fn estimate_command() {
        env::set_var("AR_KEYPAIR_PATH", "some_path/some_keypair.json");
        let m = get_app().get_matches_from(vec!["arloader", "estimate", "tests/fixtures/*.png"]);
        let sub_m = m.subcommand_matches("estimate").unwrap();
        assert_eq!(sub_m.value_of("glob").unwrap(), "tests/fixtures/*.png");
        assert_eq!(
            value_t!(sub_m.value_of("reward_multiplier"), f32).unwrap(),
            1f32
        );
    }

    #[test]
    fn estimate_command_with_sol() {
        env::set_var("AR_KEYPAIR_PATH", "some_path/some_keypair.json");
        let m = get_app().get_matches_from(vec![
            "arloader",
            "estimate",
            "tests/fixtures/*.png",
            "--with-sol",
            "-n",
            "--reward-multiplier",
            "1.5",
        ]);
        let sub_m = m.subcommand_matches("estimate").unwrap();
        assert_eq!(sub_m.value_of("glob").unwrap(), "tests/fixtures/*.png");
        assert_eq!(sub_m.is_present("with_sol"), true);
        assert_eq!(sub_m.is_present("no_bundle"), true);
        assert_eq!(
            value_t!(sub_m.value_of("reward_multiplier"), f32).unwrap(),
            1.5f32
        );
    }

    #[test]
    fn upload_command() {
        env::set_var("AR_KEYPAIR_PATH", "some_path/some_keypair.json");
        let m = get_app().get_matches_from(vec!["arloader", "upload", "tests/fixtures/*.png"]);
        let sub_m = m.subcommand_matches("upload").unwrap();
        assert_eq!(sub_m.value_of("glob").unwrap(), "tests/fixtures/*.png");
        assert_eq!(
            value_t!(sub_m.value_of("reward_multiplier"), f32).unwrap(),
            1f32
        );
    }

    #[test]
    fn upload_command_with_sol() {
        env::remove_var("SOL_KEYPAIR_PATH");
        env::set_var("AR_KEYPAIR_PATH", "some_path/some_keypair.json");

        let resp = get_app().get_matches_from_safe(vec![
            "arloader",
            "upload",
            "tests/fixtures/*.png",
            "--with-sol",
        ]);

        assert_eq!(resp.unwrap_err().kind, ErrorKind::MissingRequiredArgument);

        env::set_var("SOL_KEYPAIR_PATH", "some_path/some_sol_keypair.json");
        let m = get_app().get_matches_from(vec![
            "arloader",
            "upload",
            "tests/fixtures/*.png",
            "--with-sol",
        ]);
        let sub_m = m.subcommand_matches("upload").unwrap();
        assert_eq!(
            sub_m.value_of("sol_keypair_path").unwrap(),
            "some_path/some_sol_keypair.json"
        );
        assert!(sub_m.is_present("with_sol"));
    }
}
