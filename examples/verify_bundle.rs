use arloader::transaction::stringify;
use reqwest;
use serde::Deserialize;

#[tokio::main]
async fn main() {
    let txid = "690t_L2ALtdT8mFvfKmO_u5zGel_x3EtKcKTyo2x6JY";

    #[derive(Deserialize, Debug)]
    struct Offset {
        #[serde(with = "stringify")]
        size: usize,
        #[serde(with = "stringify")]
        offset: usize,
    }

    let offset = reqwest::get(format!("https://arweave.net/tx/{}/offset", txid))
        .await
        .unwrap()
        .json::<Offset>()
        .await
        .unwrap();

    println!("{:?}", offset)
}
