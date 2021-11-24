[![crates.io](https://img.shields.io/crates/v/arloader.svg)](https://crates.io/crates/arloader)
[![build status](https://github.com/CalebEverett/arloader/actions/workflows/build.yml/badge.svg)](https://github.com/CalebEverett/arloader/actions/workflows/build.yml)
[![docs.rs](https://img.shields.io/docsrs/arloader)](https://docs.rs/arloader)
![Crates.io](https://img.shields.io/crates/l/arloader)

# arloader

Command line application and library for effortlessly uploading files to [Arweave](https://www.arweave.org/). Arweave enables you to store documents and applications forever.

Upload gigabytes of files with one command specifying a glob pattern to match files against. Files are read and posted to [arweave.net](https://arweave.net) asynchronously and computationally intensive bundle preparation is performed in parallel across multiple threads.

## Installation

1. If you're on Linux, you can install the binary from the [releases on github](https://github.com/CalebEverett/arloader/releases). Otherwise, of if you prefer, you can install from [crates.io](https://crates.io) once you have [rust installed](https://www.rust-lang.org/tools/install).

```
cargo install arloader
```

2. Get an Arweave wallet json file [here](https://faucet.arweave.net/).

3. If you're going to use AR to pay for transactions, get AR tokens. I've been using [gate.io](https://gate.io) despite the high withdrawal fees and KYC delays.

4. If you're going to use SOL, get a [Solana wallet](https://docs.solana.com/wallet-guide/cli) json file and transfer some SOL to it.

## Usage

If you're uploading more than one file, you should pretty much always be using bundles. Bundles take multiple files and packages them together in a single transaction. This is better than uploading multiple individual files because you only have to wait for one transaction to be confirmed. Once the bundle transaction is confirmed, all of your files will be available. Larger transactions with larger rewards are more attractive to miners, which means a larger bundled transaction is more likely to get written quickly than a bunch of smaller individual ones.

Arloader accepts file glob patterns and defaults to creating a bundle for your files.

Arloader will create as many bundles as necessary to upload all of your files. Your files are read asynchronously, bundled in parallel across multiple threads and then posted to [arweave.net](https://arweave.net). Currently Arloader support bundle sizes up to 10 MB in order to be able to post to the `/tx` endpoint with full bundles size payloads (avoiding posting in 256 KB chunks to the `/chunks` endpoint. This should work fine for file sizes less than 10 MB. Future versions will support larger file sizes. 

1. To get an estimate of the cost of uploading your files run

```
arloader estimate "<GLOB>"
```

Make sure to include quotes around your glob patterns, otherwise your shell will expand them into a list of files. Arloader expects a glob pattern, not a list of files.

2. To upload your files run

```
arloader upload "<GLOB>" --log-dir "<LOG_DIR>"
```

This kicks off the process of uploading a stream of bundles created from your files. The default bundle size is 10 MB. The example output below had a bundle size of 5000 bytes.

```
bundle txid                                   items      KB  status       confirms
------------------------------------------------------------------------------------
 QGPFcZq91lQgmmz2l7rQHkSQpgfJi-Vhv47oTqIYLm4       2       3  Submitted           0
 _-bhdsi4irDEWz8R9wXT-1c06WVQVSMAmQxVF9OkW94       2       3  Submitted           0
 -OAWdFiGS4NKOZXVJG3yZ0yN4xydGOhfQGX2FCdlG88       2       3  Submitted           0
 UBWGFKyTrUVaCa7wi_181FjAd545vdoHmBQEdlaVdA4       2       3  Submitted           0
 qzQlASZrQXNF9HYIOTPjEZL9uy1U9Ou086kCkQWqld0       2       3  Submitted           0
 ```

A json status object gets written to `LOG_DIR` for each uploaded bundle with a file name of `<TXID>.json`. It has the transaction id, reward, creation time and ids and paths of the files included in the bundle.

```json
{
    "id": "_-bhdsi4irDEWz8R9wXT-1c06WVQVSMAmQxVF9OkW94",
    "status": "Submitted",
    "file_paths": {
        "tests/fixtures/8.png": {
            "id": "0jd-NTQUZhmnKRY-kMt2vEcmSqgzKOLX_P3QYw6CaNE"
        },
        "tests/fixtures/9.png": {
            "id": "1XdiLkoZ5POHsNx7eLyRgisjnxTLzW8SxGsRcb22j84"
        }
    },
    "number_of_files": 2,
    "data_size": 3546,
    "created_at": "2021-11-23T05:47:41.948103600Z",
    "last_modified": "2021-11-23T05:47:41.948107100Z",
    "reward": 50947968
}
```

3. After uploading your files, you'll want to check on their status to make sure the have been uploaded successfully and that they ultimately are confirmed at least 25 times before you can be absolutely certain they have been permanently uploaded.

```
arloader update-status --log-dir "<LOG_DIR>"
```

This will read the files in `<LOG_DIR>`, looking for a valid transaction id as a file stem, and then go out to the network to update the status of each. The example below contained two sets of bundles, one still pending and one with 45 confirmations.

```
bundle txid                                   items      KB  status       confirms
------------------------------------------------------------------------------------
 -OAWdFiGS4NKOZXVJG3yZ0yN4xydGOhfQGX2FCdlG88       2       3  Pending             0
 _-bhdsi4irDEWz8R9wXT-1c06WVQVSMAmQxVF9OkW94       2       3  Pending             0
 qzQlASZrQXNF9HYIOTPjEZL9uy1U9Ou086kCkQWqld0       2       3  Pending             0
 QGPFcZq91lQgmmz2l7rQHkSQpgfJi-Vhv47oTqIYLm4       2       3  Pending             0
 UBWGFKyTrUVaCa7wi_181FjAd545vdoHmBQEdlaVdA4       2       3  Pending             0
 KuuEZpbfCbw6izMeN3knWlpzmaFhnrDL9dUKCW2LQHw       2       3  Confirmed          45
 IRToYYvsftCiR71sfW5qt8XCzBFotwoDFBoEMEtrMrU       2       3  Confirmed          45
 M2QZYxUqw3ZJ2KXzU4pfw9fFIkVOSrJbSpE7NAvHLvo       2       3  Confirmed          45
 qvci4i6Mfr-5_NHI1bL-Omv16QEUw3iiirzv4fXefnM       2       3  Confirmed          45
 NAP2vTKQdMG_eKyKBYz3876T4yBFl4oYFYqwwwnHbFA       2       3  Confirmed          45
 ```

4. Once you have a sufficient number of confirmations of your files, you may want to create a manifest file, which is used by the Arweave gateways to provide relative paths to your files. In order to do that, you run

```
arloader upload-manifest --log-dir "<LOG_DIR>"
```
where `<LOG_DIR>` is the directory containing your bundle status json files. This will go through and consolidate the paths from each of the bundles, create a consolidated manifest, upload it to Arweave and then write a file named `manifest_<TXID>.json`to `<LOG_DIR>`. Once the transaction uploading the manifest has been confirmed, you will be able to access your files and both `https://arweave.net/<BUNDLE_ITEM_ID>` and `https://arweave.net<MANIFEST_ID>/<FILE_PATH>`  where `MANIFEST_ID` is the id of the manifest transaction and `FILE_PATH` is the relative path of the file included in the `GLOB` pattern you specified with the `upload` command.

```json
{
    "id_paths": [
        "https://arweave.net/aCdWUSXMoDWGzjc55dtGV1H-cVVwWsCWQK02JNkIPVE",
        "https://arweave.net/j8CkOBNAXDhYW2Tsw5r3JhQXUaFcxWqiemQVlBRB3Xc",
        "https://arweave.net/hG4UvTIN_xxcg1gv_k2HwEO5RHv67iUp70LqVjJe6QQ",
        "https://arweave.net/297e1JxgnSv6MABn8XEeZOgXW_zDKv-C5mhHAS6NaAY",
        "https://arweave.net/DLI1O46CSAu-iVClAuRt7bTw0Kp71hMMnQowBT2i_gI",
        "https://arweave.net/T-03eMzyk_ribRHHsoNEmhoWWpMj7xCICh0A-p5yUOc",
        "https://arweave.net/xBolTrDkS-2-zDP0efRaq81Mc9rQg1LjgPsp1V3GJks",
        "https://arweave.net/doXvNsNq3bEX-aVzl-068xa3sPvmOmMAxLrDKvRnYcM",
        "https://arweave.net/oGa3YtiFUObfxAEM8SNp0Pij9oKuX_N6zJ7R9cjQ6h8",
        "https://arweave.net/qK8OOc6r9K4mFqnnSEfqSEct97V7Bgsvobes4gaad14"
    ],
    "relative_paths": [
        "https://arweave.net/n3WrgRsplTDvCe_TIMPXWfhXIT-OsGQ9y78Gz11-jKI/tests/fixtures/0.png",
        "https://arweave.net/n3WrgRsplTDvCe_TIMPXWfhXIT-OsGQ9y78Gz11-jKI/tests/fixtures/1.png",
        "https://arweave.net/n3WrgRsplTDvCe_TIMPXWfhXIT-OsGQ9y78Gz11-jKI/tests/fixtures/2.png",
        "https://arweave.net/n3WrgRsplTDvCe_TIMPXWfhXIT-OsGQ9y78Gz11-jKI/tests/fixtures/3.png",
        "https://arweave.net/n3WrgRsplTDvCe_TIMPXWfhXIT-OsGQ9y78Gz11-jKI/tests/fixtures/4.png",
        "https://arweave.net/n3WrgRsplTDvCe_TIMPXWfhXIT-OsGQ9y78Gz11-jKI/tests/fixtures/5.png",
        "https://arweave.net/n3WrgRsplTDvCe_TIMPXWfhXIT-OsGQ9y78Gz11-jKI/tests/fixtures/6.png",
        "https://arweave.net/n3WrgRsplTDvCe_TIMPXWfhXIT-OsGQ9y78Gz11-jKI/tests/fixtures/7.png",
        "https://arweave.net/n3WrgRsplTDvCe_TIMPXWfhXIT-OsGQ9y78Gz11-jKI/tests/fixtures/8.png",
        "https://arweave.net/n3WrgRsplTDvCe_TIMPXWfhXIT-OsGQ9y78Gz11-jKI/tests/fixtures/9.png"
    ]
}
```

You can run the following command to get an update on the status of your manifest transaction.
```
arloader get-status `<MANIFEST_ID>`
```

## Usage with SOL

You can use SOL to pay for your transactions without going through the hassle of procuring AR tokens.

Arloader usage is pretty much exactly the same as above, with the addition of the `--with-sol` flag.

1. To get an estimate of the cost of uploading your files run

```
arloader estimate "<GLOB>" --with-sol
```

2. To upload your files run

```
arloader upload "<GLOB>" --log-dir "<LOG_DIR>" --with sol
```

This will create the same stream of bundles that gets created without using SOL and then goes out to an api to get your transactions signed. Once the SOL payment transaction has gone through, the signature comes back from the api and gets added to your bundle transaction. Then the transaction gets uploaded directly to the [arweave.net](https:://arweave.net) gateway from your computer.

## Reward Multiplier

Arweave is limited to approximately 1,000 transactions every two minutes so if you happen to submit your transaction at a time when there are a lot of pending transactions, it may take longer to get written, or if there are enough more attractive transaction, i.e, with higher rewards, it may not get written at all. To check the current number of pending transactions, run 

```
arloader pending
```
and that will print the number of pending transactions every second for one minute.

```
 pending tx
-------------------------------------------------------------------------------------------------
  118 | ▥▥▥
  123 | ▥▥▥
  124 | ▥▥▥
  224 | ▥▥▥▥▥
  125 | ▥▥▥
  326 | ▥▥▥▥▥▥▥
  128 | ▥▥▥
  ```

Given that Arloader bundles by default, your transaction is hopefully relatively attractive and you don't need to increase the reward to get it written in a timely fashion. However, if you see that there are a lot of transactions pending and you want to be sure your transaction goes through quickly, you can adjust the reward with `--reward-multiplier` followed by something tha can be parsed as a float between `0.0` and `10.0`. The reward included in your transaction will then be multiplied by this factor when it gets submitted. Similar to the `--with-sol` flag, you can add `--reward-multiplier` to both `estimate` and `upload` commands.

## Usage without Bundles

You can add the `--no-bundle` flag if for some reason you want to create individual transactions. This works with both `estimate` and `upload` commands. In that case individual status objects are written to `LOG_DIR` and you can run `update-status` to update them from the network and `status-report` for a count of transactions by status.