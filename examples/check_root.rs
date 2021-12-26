use arloader::{commands::CommandResult, transaction::Base64, Arweave};
use reqwest;
use std::str::FromStr;

#[tokio::main]
async fn main() -> CommandResult {
    let arweave = Arweave::default();
    let id = Base64::from_str("1ZZ36S7S4WsxhQnub5mKo6N_lNHPwumHNqgkz4dNqFk")?;
    let downloaded_transaction = arweave.get_transaction(&id).await?;
    println!(
        "Downloaded:\ntxid: {}\ndata_root: {}",
        downloaded_transaction.id, downloaded_transaction.data_root
    );

    let data = reqwest::get(format!("https://arweave.net/{}", id))
        .await?
        .bytes()
        .await?;

    let calculated_transaction = arweave.merklize(data.to_vec())?;

    println!(
        "Calculated:\ntxid: {}\ndata_root: {}",
        calculated_transaction.id, calculated_transaction.data_root
    );
    Ok(())
}
