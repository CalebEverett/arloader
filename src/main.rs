use arloader::{
    commands::*,
    status::StatusCode,
    transaction::{Base64, FromUtf8Strs, Tag},
    Arweave,
};
use clap::{
    self, crate_description, crate_name, crate_version, value_t, App, AppSettings, Arg, ArgGroup,
    SubCommand, Values,
};
use std::{fmt::Display, path::PathBuf, str::FromStr};
use url::Url;
#[tokio::main]
async fn main() -> CommandResult {
    env_logger::init();
    let app_matches = get_app().get_matches();
    let base_url = app_matches
        .value_of("base_url")
        .map(|s| Url::from_str(&s.add_trailing_slash()))
        .unwrap()
        .unwrap();

    let (sub_command, arg_matches) = app_matches.subcommand();

    match (sub_command, arg_matches) {
        ("balance", Some(sub_arg_matches)) => {
            let arweave = if let Some(ar_keypair_path) = sub_arg_matches.value_of("ar_keypair_path")
            {
                Arweave::from_keypair_path(PathBuf::from(ar_keypair_path.expand_tilde()), base_url)
                    .await
                    .unwrap()
            } else {
                Arweave::default()
            };
            let wallet_address = sub_arg_matches
                .value_of("wallet_address")
                .map(|v| v.to_string());
            command_wallet_balance(&arweave, wallet_address).await
        }
        ("estimate", Some(sub_arg_matches)) => {
            let glob_str = &sub_arg_matches.value_of("glob").unwrap().expand_tilde();
            let reward_mult = value_t!(sub_arg_matches.value_of("reward_multiplier"), f32).unwrap();
            let with_sol = sub_arg_matches.is_present("with_sol");
            let bundle_size =
                value_t!(sub_arg_matches.value_of("bundle_size"), u64).unwrap() * 1_000_000;
            let no_bundle = sub_arg_matches.is_present("no_bundle");
            command_get_cost(
                &Arweave::default(),
                glob_str,
                reward_mult,
                with_sol,
                bundle_size,
                no_bundle,
            )
            .await
        }
        ("get-status", Some(sub_arg_matches)) => {
            let id = sub_arg_matches.value_of("id").unwrap();
            let output_format = app_matches.value_of("output_format").unwrap_or("");
            command_get_status(&&Arweave::default(), id, output_format).await
        }
        ("get-transaction", Some(sub_arg_matches)) => {
            let id = sub_arg_matches.value_of("id").unwrap();
            command_get_transaction(&Arweave::default(), id).await
        }
        ("list-status", Some(sub_arg_matches)) => {
            let glob_str = &sub_arg_matches.value_of("glob").unwrap().expand_tilde();
            let log_dir = &sub_arg_matches
                .value_of("log_dir")
                .unwrap()
                .expand_tilde()
                .add_trailing_slash();

            let statuses = if let Some(values) = sub_arg_matches.values_of("statuses") {
                Some(values.map(get_status_code).collect())
            } else {
                None
            };

            let max_confirms = sub_arg_matches.value_of("max_confirms");
            let output_format = app_matches.value_of("output_format");
            command_list_statuses(
                &Arweave::default(),
                glob_str,
                log_dir,
                statuses,
                max_confirms,
                output_format,
            )
            .await
        }
        ("pending", Some(_)) => command_get_pending_count(&Arweave::default()).await,
        ("re-upload", Some(sub_arg_matches)) => {
            let arweave = if let Some(ar_keypair_path) = sub_arg_matches.value_of("ar_keypair_path")
            {
                Arweave::from_keypair_path(PathBuf::from(ar_keypair_path.expand_tilde()), base_url)
                    .await
                    .unwrap()
            } else {
                Arweave::default()
            };
            let glob_str = &sub_arg_matches.value_of("glob").unwrap().expand_tilde();
            let log_dir = &sub_arg_matches
                .value_of("log_dir")
                .unwrap()
                .expand_tilde()
                .add_trailing_slash();
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
        ("status-report", Some(sub_arg_matches)) => {
            let glob_str = &sub_arg_matches.value_of("glob").unwrap().expand_tilde();
            let log_dir = &sub_arg_matches
                .value_of("log_dir")
                .unwrap()
                .expand_tilde()
                .add_trailing_slash();
            command_status_report(&Arweave::default(), glob_str, log_dir).await
        }
        ("update-metadata", Some(sub_arg_matches)) => {
            let glob_str = &sub_arg_matches.value_of("glob").unwrap().expand_tilde();
            let manifest_str = &sub_arg_matches
                .value_of("manifest_path")
                .unwrap()
                .expand_tilde();
            let link_file = sub_arg_matches.is_present("link_file");
            command_update_metadata(&Arweave::default(), glob_str, manifest_str, link_file).await
        }
        ("update-nft-status", Some(sub_arg_matches)) => {
            let log_dir = &sub_arg_matches
                .value_of("log_dir")
                .unwrap()
                .expand_tilde()
                .add_trailing_slash();
            let arweave = Arweave::default();
            let output_format = app_matches.value_of("output_format");
            let buffer = value_t!(sub_arg_matches.value_of("buffer"), usize).unwrap();
            command_update_nft_statuses(&arweave, log_dir, output_format, buffer).await
        }
        ("update-status", Some(sub_arg_matches)) => {
            let log_dir = &sub_arg_matches
                .value_of("log_dir")
                .unwrap()
                .expand_tilde()
                .add_trailing_slash();
            let arweave = Arweave::default();
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
        ("upload", Some(sub_arg_matches)) => {
            let arweave = if let Some(ar_keypair_path) = sub_arg_matches.value_of("ar_keypair_path")
            {
                Arweave::from_keypair_path(PathBuf::from(ar_keypair_path.expand_tilde()), base_url)
                    .await
                    .unwrap()
            } else {
                Arweave::default()
            };
            let glob_str = &sub_arg_matches.value_of("glob").unwrap().expand_tilde();
            let log_dir = sub_arg_matches
                .value_of("log_dir")
                .map(|s| s.expand_tilde().add_trailing_slash());
            let reward_mult = value_t!(sub_arg_matches.value_of("reward_multiplier"), f32).unwrap();
            let bundle_size =
                value_t!(sub_arg_matches.value_of("bundle_size"), u64).unwrap() * 1_000_000;
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
        ("upload-nfts", Some(sub_arg_matches)) => {
            let arweave = if let Some(ar_keypair_path) = sub_arg_matches.value_of("ar_keypair_path")
            {
                Arweave::from_keypair_path(PathBuf::from(ar_keypair_path.expand_tilde()), base_url)
                    .await
                    .unwrap()
            } else {
                Arweave::default()
            };
            let glob_str = &sub_arg_matches.value_of("glob").unwrap().expand_tilde();
            let reward_mult = value_t!(sub_arg_matches.value_of("reward_multiplier"), f32).unwrap();
            let bundle_size =
                value_t!(sub_arg_matches.value_of("bundle_size"), u64).unwrap() * 1_000_000;
            let buffer = value_t!(sub_arg_matches.value_of("buffer"), usize).unwrap();
            let link_file = sub_arg_matches.is_present("link_file");
            let output_format = app_matches.value_of("output_format");
            command_upload_nfts(
                &arweave,
                glob_str,
                bundle_size,
                reward_mult,
                output_format,
                buffer,
                sub_arg_matches.value_of("sol_keypair_path"),
                link_file,
            )
            .await
        }
        ("upload-manifest", Some(sub_arg_matches)) => {
            let arweave = if let Some(ar_keypair_path) = sub_arg_matches.value_of("ar_keypair_path")
            {
                Arweave::from_keypair_path(PathBuf::from(ar_keypair_path.expand_tilde()), base_url)
                    .await
                    .unwrap()
            } else {
                Arweave::default()
            };
            let log_dir = &sub_arg_matches
                .value_of("log_dir")
                .unwrap()
                .expand_tilde()
                .add_trailing_slash();
            let reward_mult = value_t!(sub_arg_matches.value_of("reward_multiplier"), f32).unwrap();
            let sol_key_pair_path = sub_arg_matches
                .value_of("sol_keypair_path")
                .map(|s| s.expand_tilde());

            command_upload_manifest(&arweave, log_dir, reward_mult, sol_key_pair_path).await
        }
        ("write-metaplex-items", Some(sub_arg_matches)) => {
            let glob_str = &sub_arg_matches.value_of("glob").unwrap().expand_tilde();
            let manifest_str = &sub_arg_matches
                .value_of("manifest_path")
                .unwrap()
                .expand_tilde();
            let link_file = sub_arg_matches.is_present("link_file");
            command_write_metaplex_items(&Arweave::default(), glob_str, manifest_str, link_file)
                .await
        }
        _ => unreachable!(),
    }
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
                .validator(is_parsable::<Url>)
                .default_value("https://arweave.net/")
                .env("AR_BASE_URL")
                .help("Base url for network requests."),
        )
        .arg(
            Arg::with_name("output_format")
                .long("output")
                .short("o")
                .value_name("FORMAT")
                .global(true)
                .takes_value(true)
                .possible_values(&["quiet", "verbose", "json", "json-compact"])
                .help("Specify output format."),
        )
        .subcommand(
            SubCommand::with_name("balance")
                .about("Prints the balance of an Arweave wallet.")
                .arg(
                    Arg::with_name("wallet_address")
                        .value_name("WALLET_ADDRESS")
                        .takes_value(true)
                        .validator(is_parsable::<Base64>)
                        .required_unless("ar_keypair_path")
                        .help(
                            "Specify the address of the wallet. \
                            Defaults to <AR_KEYPAIR_PATH>.",
                        ),
                )
                .arg(ar_keypair_path_arg()),
        )
        .subcommand(
            SubCommand::with_name("estimate")
                .about("Prints the estimated cost of uploading files.")
                .arg(glob_arg(true))
                .arg(reward_multiplier_arg())
                .arg(with_sol_arg())
                .arg(bundle_size_arg())
                .arg(no_bundle_arg()),
        )
        .subcommand(
            SubCommand::with_name("get-status")
                .about("Prints the status of a transaction.")
                .arg(id_arg()),
        )
        .subcommand(
            SubCommand::with_name("get-transaction")
                .about("Gets a transaction from the network.")
                .arg(id_arg()),
        )
        .subcommand(
            SubCommand::with_name("list-status")
                .about("Prints statuses.")
                .arg(glob_arg(true))
                .arg(log_dir_arg_read().long("log-dir"))
                .arg(statuses_arg())
                .arg(max_confirms_arg()),
        )
        .subcommand(
            SubCommand::with_name("pending").about("Prints count of pending network transactions."),
        )
        .subcommand(
            SubCommand::with_name("status-report")
                .about("Prints a summary of statuses.")
                .arg(glob_arg(true))
                .arg(log_dir_arg_read().long("log-dir")),
        )
        .subcommand(
            SubCommand::with_name("update-nft-status")
                .about("Updates statuses from NFT upload.")
                .arg(status_log_dir_arg())
                .arg(buffer_arg("10")),
        )
        .subcommand(
            SubCommand::with_name("update-status")
                .about("Updates statuses and prints them.")
                .arg(log_dir_arg_read())
                .arg(glob_arg(false).long("glob"))
                .arg(no_bundle_arg().requires("glob"))
                .arg(buffer_arg("10"))
                .after_help(
                    "EXAMPLES:\nTo update bundle statuses written to some/directory/status:\n\n\tarloader update-status some/directory/status \
                    \n\nTo update individual transaction statuses for files with an extension of *.png written to some/directory/status:\n\n\tarloader update-status some/directory/status --glob \"*.png\" --no-bundle \
                    \n\nNOTES:\n- Make sure to include quotes around <GLOB>.\n- Make sure <GLOB> matches the files you uploaded, not the json status files.
                    " ,
                ),
        )
        .subcommand(
            SubCommand::with_name("update-metadata")
                .about("Updates NFT metadata files with links to uploaded asset files.")
                .arg(glob_arg(true))
                .arg(manifest_path_arg())
                .arg(link_file_arg()),
        )
        .subcommand(
            SubCommand::with_name("upload")
                .about("Uploads files.")
                .arg(glob_arg(true))
                .arg(log_dir_arg_write().long("log-dir"))
                .arg(tags_arg())
                .arg(reward_multiplier_arg())
                .arg(ar_keypair_path_arg().required_unless("with_sol"))
                .arg(ar_default_keypair())
                .arg(with_sol_arg().requires("sol_keypair_path"))
                .arg(sol_keypair_path_arg())
                .arg(no_bundle_arg())
                .arg(buffer_arg("5"))
                .arg(bundle_size_arg())
                .group(
                    ArgGroup::with_name("ar_keypair")
                        .args(&["ar_keypair_path", "ar_default_keypair"])
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("re-upload")
                .about("Re-uploads files. Not currently implemented for bundles.")
                .arg(glob_arg(true))
                .arg(log_dir_arg_write().long("log-dir").required(true))
                .arg(reward_multiplier_arg())
                .arg(statuses_arg())
                .arg(max_confirms_arg())
                .arg(ar_keypair_path_arg().required(true)),
        )
        .subcommand(
            SubCommand::with_name("upload-manifest")
                .about("Uploads a manifest for uploaded files. Only currently implemented bundles.")
                .arg(log_dir_arg_read())
                .arg(reward_multiplier_arg())
                .arg(ar_keypair_path_arg().required_unless("with_sol"))
                .arg(ar_default_keypair())
                .arg(with_sol_arg().requires("sol_keypair_path"))
                .arg(sol_keypair_path_arg())
                .group(
                    ArgGroup::with_name("ar_keypair")
                        .args(&["ar_keypair_path", "ar_default_keypair"])
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("upload-nfts")
                .about("Uploads a directory with pairs of asset and metadata files.")
                .arg(glob_arg(true))
                .arg(reward_multiplier_arg())
                .arg(ar_keypair_path_arg().required_unless("with_sol"))
                .arg(ar_default_keypair())
                .arg(with_sol_arg().requires("sol_keypair_path"))
                .arg(sol_keypair_path_arg())
                .arg(buffer_arg("5"))
                .arg(bundle_size_arg())
                .arg(link_file_arg())
                .group(
                    ArgGroup::with_name("ar_keypair")
                        .args(&["ar_keypair_path", "ar_default_keypair"])
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("write-metaplex-items")
                .about("Writes metaplex items to file.")
                .arg(glob_arg(true))
                .arg(manifest_path_arg())
                .arg(link_file_arg()),
        );
    app_matches
}

// ====================
// Arguments
// ====================

fn ar_default_keypair<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("ar_default_keypair")
        .long("ar-default-keypair")
        .value_name("AR_DEFAULT_KEYPAIR")
        .takes_value(false)
        .requires("with_sol")
        .help("Uses the default AR keypair to sign data items if funding with SOL.")
}

fn ar_keypair_path_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("ar_keypair_path")
        .long("ar-keypair-path")
        .value_name("AR_KEYPAIR_PATH")
        .validator(is_valid_file_path)
        .env("AR_KEYPAIR_PATH")
        .help("Specify path of keypair file to use for funding transactions.")
}

fn buffer_arg<'a, 'b>(default: &'a str) -> Arg<'a, 'b> {
    Arg::with_name("buffer")
        .long("buffer")
        .value_name("BUFFER")
        .takes_value(true)
        .validator(is_parsable::<usize>)
        .default_value(default)
        .help("Specify the maximum number of concurrent network requests.")
}

fn bundle_size_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("bundle_size")
        .long("bundle-size")
        .value_name("BUNDLE_SIZE")
        .takes_value(true)
        .validator(is_valid_bundle_size)
        .default_value("10")
        .help("Specify the bundle size in megabytes.")
}

fn glob_arg<'a, 'b>(required: bool) -> Arg<'a, 'b> {
    Arg::with_name("glob")
        .value_name("GLOB")
        .takes_value(true)
        .required(required)
        .help(
            "Specify pattern to match files against. \
            MUST BE IN QUOTES TO AVOID SHELL EXPANSION.",
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

fn link_file_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("link_file")
        .long("link-file")
        .value_name("LINK_FILE")
        .required(false)
        .takes_value(false)
        .help("Uses file based link instead of id based link.")
}

fn log_dir_arg_write<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("log_dir")
        .value_name("LOG_DIR")
        .validator(is_valid_dir)
        .takes_value(true)
        .help("Specify a directory to write status updates to.")
}

fn log_dir_arg_read<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("log_dir")
        .required(true)
        .value_name("LOG_DIR")
        .validator(is_valid_dir)
        .takes_value(true)
        .help("Specify the directory that statuses have been written to.")
}

fn manifest_path_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("manifest_path")
        .long("manifest-path")
        .value_name("MANIFEST_PATH")
        .required(true)
        .validator(is_parsable::<PathBuf>)
        .help("Path of manifest file from which to update NFT metadata files.")
}

fn max_confirms_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("max_confirms")
        .long("max-confirms")
        .value_name("MAX_CONFIRM")
        .takes_value(true)
        .help("Specify maximum number of confirmations to filter statuses by.")
}

fn no_bundle_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("no_bundle")
        .long("no-bundle")
        .short("nb")
        .value_name("NO_BUNDLE")
        .required(false)
        .takes_value(false)
        .help("Individual transaction for each file without bundling.")
}

fn reward_multiplier_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("reward_multiplier")
        .long("reward-multiplier")
        .short("rx")
        .value_name("REWARD_MULT")
        .default_value("1.0")
        .validator(is_valid_reward_multiplier)
        .help("Specify a float between 0.0 and 10.0 to multiply the reward by.")
}

fn sol_keypair_path_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("sol_keypair_path")
        .long("sol-keypair-path")
        .value_name("SOL_KEYPAIR_PATH")
        .validator(is_valid_file_path)
        .env("SOL_KEYPAIR_PATH")
        .help("Specify path of keypair file to use for funding transactions.")
}

fn statuses_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("statuses")
        .long("statuses")
        .value_name("STATUSES")
        .takes_value(true)
        .multiple(true)
        .possible_values(&["Submitted", "Pending", "Confirmed", "NotFound"])
        .help("Specify the status codes to filter by.")
}

fn status_log_dir_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("log_dir")
        .value_name("LOG_DIR")
        .takes_value(true)
        .takes_value(true)
        .validator(is_valid_dir)
        .help("Parent status directory that contains `assets/` and `metadata/` sub-folders.")
}

fn tags_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("tags")
        .long("tags")
        .value_name("TAGS")
        .multiple(true)
        .takes_value(true)
        .validator(is_valid_tag)
        .help(
            "Specify additional tags for uploaded files as \
        <NAME>:<VALUE>, separated by spaces. Content-Type tag \
        is inferred automatically so not necessary to \
        specify. Applied to each uploaded file.",
        )
}

fn with_sol_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("with_sol")
        .long("with-sol")
        .value_name("WITH_SOL")
        .required(false)
        .takes_value(false)
        .help("Funds transactions with with SOL.")
}

// ====================
// Validators
// ====================

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

fn is_valid_reward_multiplier(reward_mult: String) -> Result<(), String> {
    match reward_mult.parse::<f32>() {
        Ok(n) => {
            if n > 0. && n <= 10. {
                Ok(())
            } else {
                Err(format!("Multiplier must be a float between 0 and 10."))
            }
        }
        Err(_) => Err(format!("Not a valid multiplier.")),
    }
}

fn is_valid_bundle_size(bundle_size: String) -> Result<(), String> {
    match bundle_size.parse::<u64>() {
        Ok(n) => {
            if n > 0 && n <= 200 {
                Ok(())
            } else {
                Err(format!(
                    "Bundle size must be at least 1 MB and not greater than 200 MB."
                ))
            }
        }
        Err(_) => Err(format!("Not a valid bundle size.")),
    }
}

fn is_valid_dir(dir_str: String) -> Result<(), String> {
    match dir_str.parse::<PathBuf>() {
        Ok(p) => {
            if p.exists() {
                if p.is_dir() {
                    Ok(())
                } else {
                    Err(format!("Path is not a directory."))
                }
            } else {
                Err(format!("Path does not exist."))
            }
        }
        Err(_) => Err(format!("Not a valid path.")),
    }
}

fn is_valid_file_path(path_str: String) -> Result<(), String> {
    match path_str.parse::<PathBuf>() {
        Ok(p) => {
            if p.exists() {
                if p.is_file() {
                    Ok(())
                } else {
                    Err(format!("Path is not file."))
                }
            } else {
                Err(format!("Path does not exist."))
            }
        }
        Err(_) => Err(format!("Not a valid path.")),
    }
}

// ====================
// Helpers
// ====================

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

fn get_status_code(output: &str) -> StatusCode {
    match output {
        "Submitted" => StatusCode::Submitted,
        "Pending" => StatusCode::Pending,
        "Confirmed" => StatusCode::Confirmed,
        "NotFound" => StatusCode::NotFound,
        _ => StatusCode::NotFound,
    }
}

pub trait CleanPaths {
    fn expand_tilde(&self) -> String;
    fn add_trailing_slash(&self) -> String;
}

// This gets applied to all directories to both expand the tilde for the home directory
// and to make sure that there is a trailing slash.
impl CleanPaths for &str {
    fn expand_tilde(&self) -> String {
        if self.chars().next().unwrap() == '~' {
            self.replace("~", &dirs_next::home_dir().unwrap().display().to_string())
        } else {
            self.to_string()
        }
    }
    fn add_trailing_slash(&self) -> String {
        let last_char = self.chars().last().unwrap();
        if !(last_char == '/' || last_char == '\\') {
            format!("{}/", self)
        } else {
            self.to_string()
        }
    }
}

impl CleanPaths for String {
    fn expand_tilde(&self) -> String {
        if self.chars().next().unwrap() == '~' {
            self.replace("~", &dirs_next::home_dir().unwrap().display().to_string())
        } else {
            self.to_string()
        }
    }
    fn add_trailing_slash(&self) -> String {
        let last_char = self.chars().last().unwrap();
        if !(last_char == '/' || last_char == '\\') {
            format!("{}/", self)
        } else {
            self.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::get_app;
    use crate::CleanPaths;
    use arloader::error::Error;
    use clap::{value_t, ErrorKind};

    #[test]
    fn estimate() {
        // passes without any wallet
        let m = get_app().get_matches_from(vec!["arloader", "estimate", "tests/fixtures/*.png"]);
        let sub_m = m.subcommand_matches("estimate").unwrap();
        assert_eq!(sub_m.value_of("glob").unwrap(), "tests/fixtures/*.png");
        assert_eq!(
            value_t!(sub_m.value_of("reward_multiplier"), f32).unwrap(),
            1f32
        );

        // passes without any wallet with `--with-sol"
        let m = get_app().get_matches_from(vec![
            "arloader",
            "estimate",
            "tests/fixtures/*.png",
            "--with-sol",
        ]);
        let sub_m = m.subcommand_matches("estimate").unwrap();
        assert_eq!(sub_m.value_of("glob").unwrap(), "tests/fixtures/*.png");
        assert_eq!(
            value_t!(sub_m.value_of("reward_multiplier"), f32).unwrap(),
            1f32
        );
    }

    #[test]
    fn upload() -> Result<(), Error> {
        std::env::remove_var("AR_KEYPAIR_PATH");
        std::env::remove_var("SOL_KEYPAIR_PATH");
        // fails without --ar-keypair-path
        let resp = get_app().get_matches_from_safe(vec![
            "arloader",
            "upload",
            "tests/fixtures/*.png",
            "--ar-default-keypair",
        ]);
        assert_eq!(resp.unwrap_err().kind, ErrorKind::MissingRequiredArgument);

        let m = get_app().get_matches_from(vec![
            "arloader",
            "upload",
            "tests/fixtures/*.png",
            "--ar-keypair-path",
            "tests/fixtures/arweave-keyfile-MlV6DeOtRmakDOf6vgOBlif795tcWimgyPsYYNQ8q1Y.json",
        ]);
        let sub_m = m.subcommand_matches("upload").unwrap();
        assert_eq!(sub_m.value_of("glob").unwrap(), "tests/fixtures/*.png");
        assert_eq!(
            value_t!(sub_m.value_of("reward_multiplier"), f32).unwrap(),
            1f32
        );

        // fails without SOL_KEYPAIR_PATH and AR_KEYPAIR_PATH
        let resp = get_app().get_matches_from_safe(vec![
            "arloader",
            "upload",
            "tests/fixtures/*.png",
            "--with-sol",
        ]);
        assert_eq!(resp.unwrap_err().kind, ErrorKind::MissingRequiredArgument);

        // fails without AR_KEYPAIR_PATH
        let resp = get_app().get_matches_from_safe(vec![
            "arloader",
            "upload",
            "tests/fixtures/*.png",
            "--with-sol",
            "--sol-keypair-path",
            "tests/fixtures/solana_test.json",
        ]);
        assert_eq!(resp.unwrap_err().kind, ErrorKind::MissingRequiredArgument);

        // passes with --ar-default-keypair
        let m = get_app().get_matches_from(vec![
            "arloader",
            "upload",
            "tests/fixtures/*.png",
            "--with-sol",
            "--sol-keypair-path",
            "tests/fixtures/solana_test.json",
            "--ar-default-keypair",
        ]);
        let sub_m = m.subcommand_matches("upload").unwrap();
        assert_eq!(
            sub_m.value_of("sol_keypair_path").unwrap(),
            "tests/fixtures/solana_test.json"
        );
        assert!(sub_m.is_present("with_sol"));

        // passes with --ar-keypair-pathr
        let m = get_app().get_matches_from(vec![
            "arloader",
            "upload",
            "tests/fixtures/*.png",
            "--with-sol",
            "--sol-keypair-path",
            "tests/fixtures/solana_test.json",
            "--ar-keypair-path",
            "tests/fixtures/arweave-keyfile-MlV6DeOtRmakDOf6vgOBlif795tcWimgyPsYYNQ8q1Y.json",
        ]);
        let sub_m = m.subcommand_matches("upload").unwrap();
        assert_eq!(
            sub_m.value_of("sol_keypair_path").unwrap(),
            "tests/fixtures/solana_test.json"
        );

        Ok(())
    }

    #[test]
    fn upload_manifest() -> Result<(), Error> {
        std::env::remove_var("AR_KEYPAIR_PATH");
        std::env::remove_var("SOL_KEYPAIR_PATH");
        // fails without --ar-keypair-path
        let resp = get_app().get_matches_from_safe(vec![
            "arloader",
            "upload-manifest",
            "tests/fixtures/",
            "--ar-default-keypair",
        ]);
        assert_eq!(resp.unwrap_err().kind, ErrorKind::MissingRequiredArgument);

        let m = get_app().get_matches_from(vec![
            "arloader",
            "upload-manifest",
            "tests/fixtures/",
            "--ar-keypair-path",
            "tests/fixtures/arweave-keyfile-MlV6DeOtRmakDOf6vgOBlif795tcWimgyPsYYNQ8q1Y.json",
        ]);
        let sub_m = m.subcommand_matches("upload-manifest").unwrap();
        assert_eq!(sub_m.value_of("log_dir").unwrap(), "tests/fixtures/");
        assert_eq!(
            value_t!(sub_m.value_of("reward_multiplier"), f32).unwrap(),
            1f32
        );

        // fails without SOL_KEYPAIR_PATH and AR_KEYPAIR_PATH
        let resp = get_app().get_matches_from_safe(vec![
            "arloader",
            "upload-manifest",
            "tests/fixtures/",
            "--with-sol",
        ]);
        assert_eq!(resp.unwrap_err().kind, ErrorKind::MissingRequiredArgument);

        // fails without AR_KEYPAIR_PATH
        let resp = get_app().get_matches_from_safe(vec![
            "arloader",
            "upload-manifest",
            "tests/fixtures/",
            "--with-sol",
            "--sol-keypair-path",
            "tests/fixtures/solana_test.json",
        ]);
        assert_eq!(resp.unwrap_err().kind, ErrorKind::MissingRequiredArgument);

        // passes with --ar-default-keypair
        let m = get_app().get_matches_from(vec![
            "arloader",
            "upload-manifest",
            "tests/fixtures/",
            "--with-sol",
            "--sol-keypair-path",
            "tests/fixtures/solana_test.json",
            "--ar-default-keypair",
        ]);
        let sub_m = m.subcommand_matches("upload-manifest").unwrap();
        assert_eq!(
            sub_m.value_of("sol_keypair_path").unwrap(),
            "tests/fixtures/solana_test.json"
        );
        assert!(sub_m.is_present("with_sol"));

        // passes with --ar-keypair-pathr
        let m = get_app().get_matches_from(vec![
            "arloader",
            "upload-manifest",
            "tests/fixtures/",
            "--with-sol",
            "--sol-keypair-path",
            "tests/fixtures/solana_test.json",
            "--ar-keypair-path",
            "tests/fixtures/arweave-keyfile-MlV6DeOtRmakDOf6vgOBlif795tcWimgyPsYYNQ8q1Y.json",
        ]);
        let sub_m = m.subcommand_matches("upload-manifest").unwrap();
        assert_eq!(
            sub_m.value_of("sol_keypair_path").unwrap(),
            "tests/fixtures/solana_test.json"
        );

        Ok(())
    }

    #[test]
    fn base_url() -> Result<(), Error> {
        let resp =
            get_app().get_matches_from_safe(vec!["arloader", "--base-url", "notaurl", "pending"]);
        assert_eq!(resp.unwrap_err().kind, ErrorKind::ValueValidation);

        let m = get_app().get_matches_from(vec![
            "arloader",
            "--base-url",
            "https://valid_url.com",
            "pending",
        ]);
        let value = m.value_of("base_url");
        let new_value = value.unwrap().add_trailing_slash();
        assert_eq!(value.unwrap(), "https://valid_url.com");
        assert_eq!(new_value, "https://valid_url.com/");
        assert_eq!(
            ("https://valid_url.com/").add_trailing_slash(),
            "https://valid_url.com/"
        );

        Ok(())
    }

    #[test]
    fn update_status() {
        let m = get_app().get_matches_from(vec!["arloader", "update-status", "tests/"]);

        let sub_m = m.subcommand_matches("update-status").unwrap();
        assert_eq!(sub_m.value_of("log_dir").unwrap(), "tests/");
    }

    #[test]
    fn tilde_expansion() {
        assert_eq!(
            dirs_next::home_dir().unwrap().join("tests/"),
            std::path::PathBuf::from("~/tests/".expand_tilde())
        );
    }
}
