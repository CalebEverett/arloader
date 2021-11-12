use arloader::{
    error::Error,
    solana::{FLOOR, RATE, SOL_AR_BASE_URL},
    status::{OutputFormat, OutputHeader, Status, StatusCode},
    transaction::{Base64, FromStrs, Tag},
    update_statuses_stream, upload_files_stream, upload_files_with_sol_stream, Arweave,
    WINSTONS_PER_AR,
};
use clap::{
    self, crate_description, crate_name, crate_version, App, AppSettings, Arg, SubCommand, Values,
};
use futures::StreamExt;
use glob::glob;
use num_traits::cast::ToPrimitive;
use solana_sdk::signer::keypair;
use std::{fmt::Display, path::PathBuf, str::FromStr};
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
    match Tag::from_utf8_strs(split[0], split[1]) {
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

fn get_tags_vec(tag_values: Option<Values>) -> Option<Vec<Tag>> {
    if let Some(tag_strings) = tag_values {
        let tags = tag_strings
            .into_iter()
            .map(|t| {
                let split: Vec<&str> = t.split(":").collect();
                Tag::from_utf8_strs(split[0], split[1])
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

fn log_dir_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("log_dir")
        .long("log-dir")
        .value_name("LOG_DIR")
        .takes_value(true)
        .validator(is_parsable::<PathBuf>)
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

fn sol_keypair_path_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("sol_keypair_path")
        .long("sol-keypair-path")
        .value_name("SOL_KEYPAIR_PATH")
        .validator(is_parsable::<PathBuf>)
        .env("SOL_KEYPAIR_PATH")
        .required(true)
        .help(
            "Path of Solana keypair file to use to pay for transactions. \
        Will use value from SOL_KEYPAIR_PATH environment variable \
        if it exists",
        )
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
            Arg::with_name("keypair_path")
                .long("keypair-path")
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
                .value_name("FORMAT")
                .global(true)
                .takes_value(true)
                .possible_values(&["quiet", "verbose", "json", "json-compact"])
                .help("Return information in specified output format."),
        )
        .arg(
            Arg::with_name("buffer")
                .long("buffer")
                .value_name("BUFFER")
                .global(true)
                .takes_value(true)
                .validator(is_parsable::<usize>)
                .help(
                    "Sets the maximum number of concurrent network requests. \
                Defaults to 1.",
                ),
        )
        .subcommand(
            SubCommand::with_name("estimate")
                .about(
                    "Prints the estimated cost of uploading file(s) \
                matching provided glob.",
                )
                .arg(glob_arg(true)),
        )
        .subcommand(
            SubCommand::with_name("estimate-with-sol")
                .about(
                    "Prints the estimated cost of uploading file(s) \
                matching provided glob in SOL.",
                )
                .arg(glob_arg(true)),
        )
        .subcommand(
            SubCommand::with_name("wallet-balance")
                .about("Prints the balance of a wallet.")
                .arg(
                    Arg::with_name("wallet_address")
                        .value_name("WALLET_ADDRESS")
                        .takes_value(true)
                        .validator(is_parsable::<Base64>)
                        .help(
                            "Specify the address of the wallet to print \
                            the balance for. Defaults to the keypair
                            specified in `keypair_path`.",
                        ),
                ),
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
                .arg(log_dir_arg())
                .arg(tags_arg()),
        )
        .subcommand(
            SubCommand::with_name("upload-with-sol")
                .about("Uploads one or more files that match the specified glob, paying with SOL.")
                .arg(glob_arg(true))
                .arg(log_dir_arg())
                .arg(tags_arg())
                .arg(sol_keypair_path_arg()),
        )
        .subcommand(
            SubCommand::with_name("get-raw-status")
                .about("Prints the raw status of a transaction from the network.")
                .arg(id_arg()),
        )
        .subcommand(
            SubCommand::with_name("update-status")
                .about("Updates statuses stored in `log_dir` from the network.")
                .arg(glob_arg(true))
                .arg(log_dir_arg()),
        )
        .subcommand(
            SubCommand::with_name("status-report")
                .about("Prints a summary of statuses stored in `log_dir`.")
                .arg(glob_arg(false))
                .arg(log_dir_arg()),
        )
        .subcommand(
            SubCommand::with_name("generate-manifest")
                .about("Writes a manifest.json to `log_dir`.")
                .arg(glob_arg(true))
                .arg(log_dir_arg()),
        )
        .subcommand(
            SubCommand::with_name("upload-filter")
                .about("Re-uploads files that meet filter criteria.")
                .arg(glob_arg(true))
                .arg(log_dir_arg())
                .arg(statuses_arg())
                .arg(max_confirms_arg()),
        )
        .subcommand(
            SubCommand::with_name("list-status")
                .about("Lists statuses as currently store in `log_dir`.")
                .help("")
                .arg(glob_arg(true))
                .arg(log_dir_arg())
                .arg(statuses_arg())
                .arg(max_confirms_arg()),
        );
    app_matches
}

#[tokio::main]
async fn main() -> CommandResult {
    env_logger::init();
    let app_matches = get_app().get_matches();
    let keypair_path = app_matches.value_of("keypair_path").unwrap();
    let base_url = app_matches
        .value_of("base_url")
        .map(Url::from_str)
        .transpose()?;

    let arweave = Arweave::from_keypair_path(PathBuf::from(keypair_path), base_url)
        .await
        .unwrap();

    let (sub_command, arg_matches) = app_matches.subcommand();

    match (sub_command, arg_matches) {
        ("estimate", Some(sub_arg_matches)) => {
            let glob_str = sub_arg_matches.value_of("glob").unwrap();
            command_get_cost(&arweave, glob_str).await
        }
        ("estimate-with-sol", Some(sub_arg_matches)) => {
            let glob_str = sub_arg_matches.value_of("glob").unwrap();
            command_get_cost_with_sol(&arweave, glob_str).await
        }
        ("wallet-balance", Some(sub_arg_matches)) => {
            let wallet_address = sub_arg_matches
                .value_of("wallet_address")
                .map(|v| v.to_string());
            command_wallet_balance(&arweave, wallet_address).await
        }
        ("get-transaction", Some(sub_arg_matches)) => {
            let id = sub_arg_matches.value_of("id").unwrap();
            command_get_transaction(&arweave, id).await
        }
        ("upload", Some(sub_arg_matches)) => {
            let glob_str = sub_arg_matches.value_of("glob").unwrap();
            let log_dir = sub_arg_matches.value_of("log_dir");
            let tags = get_tags_vec(sub_arg_matches.values_of("tags"));
            let buffer = app_matches.value_of("buffer");
            let output_format = app_matches.value_of("output_format");
            command_upload(&arweave, glob_str, log_dir, tags, output_format, buffer).await
        }
        ("upload-with-sol", Some(sub_arg_matches)) => {
            let glob_str = sub_arg_matches.value_of("glob").unwrap();
            let log_dir = sub_arg_matches.value_of("log_dir");
            let tags = get_tags_vec(sub_arg_matches.values_of("tags"));
            let sol_keypair_path = sub_arg_matches.value_of("sol_keypair_path").unwrap();
            let buffer = app_matches.value_of("buffer");
            let output_format = app_matches.value_of("output_format");
            command_upload_with_sol(
                &arweave,
                glob_str,
                log_dir,
                tags,
                sol_keypair_path,
                output_format,
                buffer,
            )
            .await
        }
        ("get-raw-status", Some(sub_arg_matches)) => {
            let id = sub_arg_matches.value_of("id").unwrap();
            command_get_raw_status(&arweave, id).await
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
            let glob_str = sub_arg_matches.value_of("glob").unwrap();
            let log_dir = sub_arg_matches.value_of("log_dir").unwrap();
            let output_format = app_matches.value_of("output_format");
            let buffer = app_matches.value_of("buffer");
            command_update_statuses(&arweave, glob_str, log_dir, output_format, buffer).await
        }
        ("status-report", Some(sub_arg_matches)) => {
            let glob_str = sub_arg_matches.value_of("glob").unwrap();
            let log_dir = sub_arg_matches.value_of("log_dir").unwrap();
            command_status_report(&arweave, glob_str, log_dir).await
        }
        ("generate-manifest", Some(sub_arg_matches)) => {
            let glob_str = sub_arg_matches.value_of("glob").unwrap();
            let log_dir = sub_arg_matches.value_of("log_dir").unwrap();
            command_generate_manifest(&arweave, glob_str, log_dir).await
        }
        ("upload-filter", Some(sub_arg_matches)) => {
            let glob_str = sub_arg_matches.value_of("glob").unwrap();
            let log_dir = sub_arg_matches.value_of("log_dir").unwrap();

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

async fn command_get_cost(arweave: &Arweave, glob_str: &str) -> CommandResult {
    let paths_iter = glob(glob_str)?.filter_map(Result::ok);
    let (base, incremental) = arweave.get_price_terms().await?;

    let (num, cost, bytes) = paths_iter.fold((0, 0, 0), |(n, c, b), p| {
        (
            n + 1,
            c + {
                let data_len = p.metadata().unwrap().len();
                base + incremental * (data_len - 1)
            },
            b + p.metadata().unwrap().len(),
        )
    });

    let (_, usd_per_ar, _) = arweave.get_price(&1).await?;
    let usd_per_bytes = (&cost * &usd_per_ar).to_f32().unwrap() / 1e14_f32;
    println!(
        "The price to upload {} files with {} total bytes is {} {} (${}).",
        num, bytes, cost, arweave.units, usd_per_bytes
    );
    Ok(())
}

async fn command_get_cost_with_sol(arweave: &Arweave, glob_str: &str) -> CommandResult {
    let paths_iter = glob(glob_str)?.filter_map(Result::ok);
    let (base, incremental) = arweave.get_price_terms().await?;

    let (num, cost, bytes) = paths_iter.fold((0, 0, 0), |(n, c, b), p| {
        (
            n + 1,
            c + {
                let data_len = p.metadata().unwrap().len();
                std::cmp::max((base + incremental * (data_len - 1)) / RATE, FLOOR) + 5000
            },
            b + p.metadata().unwrap().len(),
        )
    });

    let (_, _, usd_per_sol) = arweave.get_price(&1).await?;
    let usd_per_bytes = (&cost * &usd_per_sol).to_f32().unwrap() / 1e11_f32;
    println!(
        "The price to upload {} files with {} total bytes is {} {} (${}).",
        num, bytes, cost, "lamports", usd_per_bytes
    );
    Ok(())
}

async fn command_get_transaction(arweave: &Arweave, id: &str) -> CommandResult {
    let id = Base64::from_str(id)?;
    let transaction = arweave.get_transaction(&id).await?;
    println!("Fetched transaction {}", transaction.id);
    Ok(())
}

async fn command_get_raw_status(arweave: &Arweave, id: &str) -> CommandResult {
    let id = Base64::from_str(id)?;
    let resp = arweave.get_raw_status(&id).await?;
    println!("{}", resp.text().await?);
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
            "Wallet balance is {} {units} (${balance_usd}). At the current price of {price} {units} (${usd_price:.4}) per MB, you can upload {max} MB of data.",
            &balance,
            units = arweave.units,
            max = &balance / &winstons_per_kb,
            price = &winstons_per_kb,
            balance_usd = balance_usd,
            usd_price = usd_per_kb
    );
    Ok(())
}

async fn command_upload(
    arweave: &Arweave,
    glob_str: &str,
    log_dir: Option<&str>,
    _tags: Option<Vec<Tag>>,
    output_format: Option<&str>,
    buffer: Option<&str>,
) -> CommandResult {
    let paths_iter = glob(glob_str)?.filter_map(Result::ok);
    let log_dir = log_dir.map(|s| PathBuf::from(s));
    let output_format = get_output_format(output_format.unwrap_or(""));
    let buffer = buffer.map(|b| b.parse::<usize>().unwrap()).unwrap_or(1);
    let price_terms = arweave.get_price_terms().await?;

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
                    println!("{}", Status::header_string(&output_format));
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
            "Uploaded {} files. Run `arloader update-status \"{}\" --log-dir {} to confirm transaction(s).",
            counter,
            glob_str,
            &log_dir.unwrap_or(PathBuf::from("")).display()
        );
    }

    Ok(())
}

async fn command_upload_with_sol(
    arweave: &Arweave,
    glob_str: &str,
    log_dir: Option<&str>,
    _tags: Option<Vec<Tag>>,
    sol_keypair_path: &str,
    output_format: Option<&str>,
    buffer: Option<&str>,
) -> CommandResult {
    let paths_iter = glob(glob_str)?.filter_map(Result::ok);
    let log_dir = log_dir.map(|s| PathBuf::from(s));
    let output_format = get_output_format(output_format.unwrap_or(""));
    let buffer = buffer.map(|b| b.parse::<usize>().unwrap()).unwrap_or(1);
    let price_terms = arweave.get_price_terms().await?;
    let solana_url = "https://api.mainnet-beta.solana.com/".parse::<Url>()?;
    let sol_ar_url = SOL_AR_BASE_URL.parse::<Url>()?.join("sol")?;
    let from_keypair = keypair::read_keypair_file(sol_keypair_path)?;

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
                    println!("{}", Status::header_string(&output_format));
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
            "Uploaded {} files. Run `arloader update-status \"{}\" --log-dir {} to confirm transaction(s).",
            counter,
            glob_str,
            &log_dir.unwrap_or(PathBuf::from("")).display()
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
            println!("{}", Status::header_string(&output_format));
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
    buffer: Option<&str>,
) -> CommandResult {
    let paths_iter = glob(glob_str)?.filter_map(Result::ok);
    let log_dir = PathBuf::from(log_dir);
    let output_format = get_output_format(output_format.unwrap_or(""));
    let buffer = buffer.map(|b| b.parse::<usize>().unwrap()).unwrap_or(1);

    let mut stream = update_statuses_stream(arweave, paths_iter, log_dir.clone(), buffer);

    let mut counter = 0;
    while let Some(Ok(status)) = stream.next().await {
        if counter == 0 {
            println!("{}", Status::header_string(&output_format));
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

async fn command_status_report(arweave: &Arweave, glob_str: &str, log_dir: &str) -> CommandResult {
    let paths_iter = glob(glob_str)?.filter_map(Result::ok);
    let log_dir = PathBuf::from(log_dir);

    let summary = arweave.status_summary(paths_iter, log_dir).await?;

    println!("{}", summary);

    Ok(())
}

async fn command_generate_manifest(
    arweave: &Arweave,
    glob_str: &str,
    log_dir: &str,
) -> CommandResult {
    let paths_iter = glob(glob_str)?.filter_map(Result::ok);
    let log_dir = PathBuf::from(log_dir);

    let _ = arweave.write_manifest(paths_iter, log_dir.clone()).await?;

    println!("manifest.json written to {}", log_dir.display());

    Ok(())
}

async fn command_upload_filter(
    arweave: &Arweave,
    glob_str: &str,
    log_dir: &str,
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
    let price_terms = arweave.get_price_terms().await?;

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
            println!("{}", Status::header_string(&output_format));
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
