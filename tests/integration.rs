use arloader::{
    crypto::Provider,
    error::Error,
    solana::SOL_AR_BASE_URL,
    status::{OutputFormat, Status, StatusCode},
    transaction::{Base64, Tag},
    upload_files_stream,
    utils::TempDir,
    Arweave,
};
use futures::{future::try_join_all, StreamExt};
use glob::glob;
use solana_sdk::signer::keypair;
use std::{iter, path::PathBuf, str::FromStr, time::Duration};
use tokio::time::sleep;
use url::Url;

async fn get_arweave() -> Result<Arweave, Error> {
    let keypair_path =
        "tests/fixtures/arweave-keyfile-MlV6DeOtRmakDOf6vgOBlif795tcWimgyPsYYNQ8q1Y.json";
    let base_url = Url::from_str("http://localhost:1984/")?;
    let arweave = Arweave::from_keypair_path(PathBuf::from(keypair_path), base_url).await?;
    Ok(arweave)
}

async fn mine(arweave: &Arweave) -> Result<(), Error> {
    let url = arweave.base_url.join("mine")?;
    let resp = reqwest::get(url).await?.text().await?;
    // Give the node server a chance
    sleep(Duration::from_secs(2)).await;
    println!("mine resp: {}", resp);
    Ok(())
}

async fn airdrop(arweave: &Arweave) -> Result<(), Error> {
    let url = arweave.base_url.join(&format!(
        "mint/{}/100000000000000",
        arweave.crypto.wallet_address().unwrap().to_string()
    ))?;
    let resp = reqwest::get(url).await?.text().await?;
    println!("mine resp: {}", resp);
    Ok(())
}

#[tokio::test]
async fn test_post_transaction() -> Result<(), Error> {
    let arweave = get_arweave().await?;
    // Don't run if test server is not running.
    if let Err(_) = reqwest::get(arweave.base_url.join("info")?).await {
        println!("Test server not running.");
        return Ok(());
    } else {
    }

    airdrop(&arweave).await?;
    let file_path = PathBuf::from("tests/fixtures/0.png");
    let transaction = arweave
        .create_transaction_from_file_path(file_path, None, None, (0, 0), true)
        .await?;

    let signed_transaction = arweave.sign_transaction(transaction)?;
    println!("signed_transaction: {:?}", &signed_transaction);
    arweave.post_transaction(&signed_transaction).await?;

    let url = arweave.base_url.join("mine")?;
    let resp = reqwest::get(url).await?.text().await?;
    println!("mine: {}", resp);

    let status = arweave.get_status(&signed_transaction.id).await?;
    println!("{:?}", status);
    Ok(())
}

#[tokio::test]
async fn test_upload_file_from_path() -> Result<(), Error> {
    let arweave = get_arweave().await?;
    // Don't run if test server is not running.
    if let Err(_) = reqwest::get(arweave.base_url.join("info")?).await {
        println!("Test server not running.");
        return Ok(());
    }

    airdrop(&arweave).await?;
    let file_path = PathBuf::from("tests/fixtures/0.png");
    let temp_log_dir = TempDir::from_str("./tests/").await?;
    let log_dir = temp_log_dir.0.clone();

    let status = arweave
        .upload_file_from_path(file_path.clone(), Some(log_dir.clone()), None, None, (0, 0))
        .await?;

    let read_status = arweave.read_status(file_path, log_dir.clone()).await?;
    println!("{:?}", &read_status);
    assert_eq!(status, read_status);
    Ok(())
}

#[tokio::test]
async fn test_update_status() -> Result<(), Error> {
    let arweave = get_arweave().await?;
    // Don't run if test server is not running.
    if let Err(_) = reqwest::get(arweave.base_url.join("info")?).await {
        println!("Test server not running.");
        return Ok(());
    }

    airdrop(&arweave).await?;
    let file_path = PathBuf::from("tests/fixtures/0.png");
    let temp_log_dir = TempDir::from_str("./tests/").await?;
    let log_dir = temp_log_dir.0.clone();

    let _ = arweave
        .upload_file_from_path(file_path.clone(), Some(log_dir.clone()), None, None, (0, 0))
        .await?;

    let read_status = arweave
        .read_status(file_path.clone(), log_dir.clone())
        .await?;
    assert_eq!(read_status.status, StatusCode::Submitted);

    let url = arweave.base_url.join("mine")?;
    let resp = reqwest::get(url).await?.text().await?;
    println!("mine resp: {}", resp);

    let updated_status = arweave.update_status(file_path, log_dir.clone()).await?;
    println!("{:?}", &updated_status);
    assert_eq!(updated_status.status, StatusCode::Confirmed);
    assert!(updated_status.last_modified > read_status.last_modified);
    Ok(())
}

#[tokio::test]
async fn test_upload_files_from_paths_without_tags() -> Result<(), Error> {
    let arweave = get_arweave().await?;
    // Don't run if test server is not running.
    if let Err(_) = reqwest::get(arweave.base_url.join("info")?).await {
        println!("Test server not running.");
        return Ok(());
    }

    airdrop(&arweave).await?;
    let paths_iter = glob("tests/fixtures/*.png")?.filter_map(Result::ok);
    let temp_log_dir = TempDir::from_str("./tests/").await?;
    let log_dir = temp_log_dir.0.clone();

    #[allow(unused_assignments)]
    let mut tags_iter = Some(iter::repeat(Some(Vec::<Tag<Base64>>::new())));
    tags_iter = None;

    let statuses = arweave
        .upload_files_from_paths(paths_iter, Some(log_dir.clone()), tags_iter, None, (0, 0))
        .await?;

    let paths_iter = glob("tests/fixtures/*.png")?.filter_map(Result::ok);
    let read_statuses = arweave.read_statuses(paths_iter, log_dir.clone()).await?;
    assert_eq!(statuses, read_statuses);
    Ok(())
}

#[tokio::test]
async fn test_update_statuses() -> Result<(), Error> {
    let arweave = get_arweave().await?;
    // Don't run if test server is not running.
    if let Err(_) = reqwest::get(arweave.base_url.join("info")?).await {
        println!("Test server not running.");
        return Ok(());
    }

    airdrop(&arweave).await?;
    let paths_iter = glob("tests/fixtures/*.png")?.filter_map(Result::ok);
    let temp_log_dir = TempDir::from_str("./tests/").await?;
    let log_dir = temp_log_dir.0.clone();

    #[allow(unused_assignments)]
    let mut tags_iter = Some(iter::repeat(Some(Vec::<Tag<Base64>>::new())));
    tags_iter = None;

    let statuses = arweave
        .upload_files_from_paths(paths_iter, Some(log_dir.clone()), tags_iter, None, (0, 0))
        .await?;

    println!("{:?}", statuses);
    let url = arweave.base_url.join("mine")?;
    let resp = reqwest::get(url).await?.text().await?;
    println!("mine resp: {}", resp);

    let paths_iter = glob("tests/fixtures/*.png")?.filter_map(Result::ok);

    let update_statuses = arweave.update_statuses(paths_iter, log_dir.clone()).await?;

    println!("{:?}", update_statuses);

    let all_confirmed = update_statuses
        .iter()
        .all(|s| s.status == StatusCode::Confirmed);
    assert!(all_confirmed);
    Ok(())
}

#[tokio::test]
async fn test_filter_statuses() -> Result<(), Error> {
    let arweave = get_arweave().await?;
    // Don't run if test server is not running.
    if let Err(_) = reqwest::get(arweave.base_url.join("info")?).await {
        println!("Test server not running.");
        return Ok(());
    }

    airdrop(&arweave).await?;
    let _ = mine(&arweave).await?;
    let paths_iter = glob("tests/fixtures/[0-4]*.png")?.filter_map(Result::ok);

    let temp_log_dir = TempDir::from_str("./tests/").await?;
    let log_dir = temp_log_dir.0.clone();

    #[allow(unused_assignments)]
    let mut tags_iter = Some(iter::repeat(Some(Vec::<Tag<Base64>>::new())));
    tags_iter = None;

    // Upload the first five files.
    let _statuses = arweave
        .upload_files_from_paths(
            paths_iter,
            Some(log_dir.clone()),
            tags_iter.clone(),
            None,
            (0, 0),
        )
        .await?;

    // Update statuses.
    let paths_iter = glob("tests/fixtures/[0-4]*.png")?.filter_map(Result::ok);
    let update_statuses = arweave.update_statuses(paths_iter, log_dir.clone()).await?;

    println!("{:?}", update_statuses);
    assert_eq!(update_statuses.len(), 5);

    // There should be 5 StatusCode::Pending.
    let paths_iter = glob("tests/fixtures/[0-4].png")?.filter_map(Result::ok);
    let pending = arweave
        .filter_statuses(
            paths_iter,
            log_dir.clone(),
            // Some(vec![StatusCode::Pending]),
            None,
            None,
        )
        .await?;
    println!("{:?}", pending);
    assert_eq!(pending.len(), 5);

    // Then mine
    let _ = mine(&arweave).await?;

    // Now when we update statuses we should get five confirmed.
    let paths_iter = glob("tests/fixtures/[0-4]*.png")?.filter_map(Result::ok);
    let _updated_statuses = arweave.update_statuses(paths_iter, log_dir.clone()).await?;
    let paths_iter = glob("tests/fixtures/[0-4].png")?.filter_map(Result::ok);
    let confirmed = arweave
        .filter_statuses(
            paths_iter,
            log_dir.clone(),
            Some(vec![StatusCode::Confirmed]),
            None,
        )
        .await?;
    assert_eq!(confirmed.len(), 5);
    println!("{:?}", confirmed);

    // Now write statuses to the log_dir without uploading them so that we get not found when we try
    // to fetch their raw statuses from the server.
    let paths_iter = glob("tests/fixtures/[5-9]*.png")?.filter_map(Result::ok);
    let transactions = try_join_all(
        paths_iter.map(|p| arweave.create_transaction_from_file_path(p, None, None, (0, 0), true)),
    )
    .await?;
    let _ = try_join_all(
        transactions
            .into_iter()
            .map(|t| arweave.sign_transaction(t))
            .filter_map(Result::ok)
            .zip(glob("tests/fixtures/[5-9]*.png")?.filter_map(Result::ok))
            .map(|(s, p)| {
                arweave.write_status(
                    Status {
                        id: s.id.clone(),
                        reward: s.reward,
                        file_path: Some(p),
                        ..Default::default()
                    },
                    log_dir.clone(),
                    None,
                )
            }),
    )
    .await?;

    // We should now have ten statuses
    let paths_iter = glob("tests/fixtures/[0-9]*.png")?.filter_map(Result::ok);
    let updated_statuses = arweave.update_statuses(paths_iter, log_dir.clone()).await?;
    assert_eq!(updated_statuses.len(), 10);

    // With five not found
    let paths_iter = glob("tests/fixtures/[0-9].png")?.filter_map(Result::ok);
    let not_found = arweave
        .filter_statuses(
            paths_iter,
            log_dir.clone(),
            Some(vec![StatusCode::NotFound]),
            None,
        )
        .await?;
    assert_eq!(not_found.len(), 5);

    // Now if we upload transactions for the not found statuses and mine we should have ten confirmed transactions.
    let paths_iter = glob("tests/fixtures/[5-9]*.png")?.filter_map(Result::ok);
    let _statuses = arweave
        .upload_files_from_paths(paths_iter, Some(log_dir.clone()), tags_iter, None, (0, 0))
        .await?;

    let _ = mine(&arweave).await?;

    let paths_iter = glob("tests/fixtures/[0-9]*.png")?.filter_map(Result::ok);
    let updated_statuses = arweave.update_statuses(paths_iter, log_dir.clone()).await?;
    assert_eq!(updated_statuses.len(), 10);

    let paths_iter = glob("tests/fixtures/[0-9].png")?.filter_map(Result::ok);
    let confirmed = arweave
        .filter_statuses(
            paths_iter,
            log_dir.clone(),
            Some(vec![StatusCode::Confirmed]),
            None,
        )
        .await?;
    assert_eq!(confirmed.len(), 10);
    Ok(())
}

#[tokio::test]
async fn test_upload_files_stream() -> Result<(), Error> {
    let arweave = get_arweave().await?;
    // Don't run if test server is not running.
    if let Err(_) = reqwest::get(arweave.base_url.join("info")?).await {
        println!("Test server not running.");
        return Ok(());
    }

    airdrop(&arweave).await?;
    mine(&arweave).await?;
    let paths_iter = glob("tests/fixtures/[0-9]*.png")?.filter_map(Result::ok);

    let temp_log_dir = TempDir::from_str("./tests/").await?;
    let _log_dir = temp_log_dir.0.clone();

    let mut _tags_iter = Some(iter::repeat(Some(Vec::<Tag<Base64>>::new())));
    _tags_iter = None;

    let mut stream = upload_files_stream(&arweave, paths_iter, None, None, (0, 0), 3);

    let output_format = OutputFormat::JsonCompact;

    let mut counter = 0;
    while let Some(Ok(status)) = stream.next().await {
        if counter == 0 {
            println!("{}", status.header_string(&output_format));
        }
        print!("{}", output_format.formatted_string(&status));
        counter += 1;
    }
    Ok(())
}

#[tokio::test]
async fn test_upload_file_from_path_with_sol() -> Result<(), Error> {
    let solana_url = "https://api.devnet.solana.com/".parse::<Url>()?;
    let sol_ar_url = SOL_AR_BASE_URL.parse::<Url>()?.join("dev")?;
    let from_keypair = keypair::read_keypair_file("tests/fixtures/solana_test.json")?;
    let arweave = get_arweave().await?;
    let ar_sol_dev_wallet_address =
        Provider::from_keypair_path(PathBuf::from("tests/fixtures/arweave_dev.json"))
            .await?
            .wallet_address()
            .unwrap()
            .to_string();

    // Don't run if test server is not running.
    if let Err(_) = reqwest::get(arweave.base_url.join("info")?).await {
        println!("Test server not running.");
        return Ok(());
    }

    airdrop(&arweave).await?;
    let url = arweave.base_url.join(&format!(
        "mint/{}/100000000000000",
        ar_sol_dev_wallet_address
    ))?;
    let _ = reqwest::get(url).await?.text().await?;

    // Don't run if sol-ar server is not running.
    if let Err(_) = reqwest::get(SOL_AR_BASE_URL).await {
        println!("sol-ar server not running.");
        return Ok(());
    }

    let file_path = PathBuf::from("tests/fixtures/0.png");
    let temp_log_dir = TempDir::from_str("./tests/").await?;
    let log_dir = temp_log_dir.0.clone();

    let status = arweave
        .upload_file_from_path_with_sol(
            file_path.clone(),
            Some(log_dir.clone()),
            None,
            None,
            (0, 0),
            solana_url,
            sol_ar_url,
            &from_keypair,
        )
        .await?;

    println!("{:?}", status);

    let read_status = arweave.read_status(file_path, log_dir.clone()).await?;
    println!("{:?}", &read_status);
    assert_eq!(status, read_status);
    Ok(())
}

#[tokio::test]
async fn test_upload_bundle_from_file_paths() -> Result<(), Error> {
    let arweave = get_arweave().await?;

    // Don't run if test server is not running.
    if let Err(_) = reqwest::get(arweave.base_url.join("info")?).await {
        println!("Test server not running.");
        return Ok(());
    }

    airdrop(&arweave).await?;
    let paths_iter = glob("tests/fixtures/*.png")?.filter_map(Result::ok);
    let paths_chunks = arweave.chunk_file_paths(paths_iter, 2000000)?;
    println!("{:?}", paths_chunks);
    let status = arweave
        .post_bundle_transaction_from_file_paths(paths_chunks[0].clone(), Vec::new(), (0, 0))
        .await?;

    println!("{:?}", status);
    Ok(())
}
