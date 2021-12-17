use arloader::OraclePrice;
use futures::future::try_join;
use num_bigint::BigUint;
use num_traits::cast::ToPrimitive;
use reqwest;

const BUNDLE_SIZE: usize = usize::pow(1024, 2) * 256;

#[tokio::main]
async fn main() {
    let prices = reqwest::get(
        "https://api.coingecko.com/api/v3/simple/price?ids=arweave,solana&vs_currencies=usd",
    )
    .await
    .unwrap()
    .json::<OraclePrice>()
    .await
    .unwrap();

    let usd_per_ar: BigUint = BigUint::from((prices.arweave.usd * 100.0).floor() as u32);

    println!(
        "Price in USD to upload {} MB of files of various sizes in KB (${:.2} USD per AR):\n",
        BUNDLE_SIZE / usize::pow(1024, 2),
        usd_per_ar.to_f32().unwrap() / 100.0
    );
    println!(
        "{} | {} | {} | {} | {} | {} | {} \n{:-<89}",
        "file size",
        "num files",
        "arweave",
        "bundlr",
        "arweave total",
        "bundlr total",
        "arweave bundle",
        ""
    );

    let arweave_bundle_price = reqwest::get(format!("https://arweave.net/price/{}", BUNDLE_SIZE))
        .await
        .unwrap()
        .json::<usize>()
        .await
        .unwrap();

    for file_size in [10, 15, 18, 20, 22, 24]
        .into_iter()
        .map(|s| usize::pow(2, s))
    {
        let (arweave_file_price, bundlr_file_price) = try_join(
            reqwest::get(format!("https://arweave.net/price/{}", file_size))
                .await
                .unwrap()
                .json::<usize>(),
            reqwest::get(format!("https://node1.bundlr.network/price/{}", file_size))
                .await
                .unwrap()
                .json::<usize>(),
        )
        .await
        .unwrap();

        let num_files = usize::pow(1024, 2) * 256 / file_size;
        let arweave_file_price_usd =
            (arweave_file_price * &usd_per_ar).to_f32().unwrap() / 1e14_f32;
        let bundlr_file_price_usd = (bundlr_file_price * &usd_per_ar).to_f32().unwrap() / 1e14_f32;

        println!(
            "{:>9} {:>11} {:>9.4} {:>8.4} {:>15.4} {:>14.4} {:>16.4}",
            file_size / 1024,
            num_files,
            arweave_file_price_usd,
            bundlr_file_price_usd,
            arweave_file_price_usd * num_files as f32,
            bundlr_file_price_usd * num_files as f32,
            (arweave_bundle_price * &usd_per_ar).to_f32().unwrap() / 1e14_f32
        );
    }
}
