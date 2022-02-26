use arloader::transaction::{stringify, Base64};
use reqwest;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Offset {
    #[serde(with = "stringify")]
    size: usize,
    #[serde(with = "stringify")]
    offset: usize,
}

#[derive(Deserialize, Debug)]
struct RawChunk {
    tx_path: Base64,
    data_path: Base64,
    chunk: Base64,
}

#[tokio::main]
async fn main() {
    let txid = "690t_L2ALtdT8mFvfKmO_u5zGel_x3EtKcKTyo2x6JY";

    let offset = reqwest::get(format!("https://arweave.net/tx/{}/offset", txid))
        .await
        .unwrap()
        .json::<Offset>()
        .await
        .unwrap();

    println!("{:?}", offset);

    let chunk = reqwest::get(format!("https://arweave.net/chunk/{}", offset.offset))
        .await
        .unwrap()
        .json::<RawChunk>()
        .await
        .unwrap();

    println!("{:?}", chunk);
}
