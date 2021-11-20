[![crates.io](https://img.shields.io/crates/v/arloader.svg)](https://crates.io/crates/arloader)
[![build status](https://github.com/CalebEverett/arloader/actions/workflows/build.yml/badge.svg)](https://github.com/CalebEverett/arloader/actions/workflows/build.yml)
[![docs.rs](https://img.shields.io/docsrs/arloader)](https://docs.rs/arloader)
![Crates.io](https://img.shields.io/crates/l/arloader)

# arloader

Command line application and library for effortlessly uploading files to [Arweave](https://www.arweave.org/). Arweave enables you to store documents and applications forever.

## Installation

1. If you're on Linux, you can install the binary from the [releases on github](https://github.com/CalebEverett/arloader/releases). Otherwise, of if you prefer you can install from [crates.io] once you have [rust installed](https://www.rust-lang.org/tools/install).

```
cargo install arloader
```

2. Get an Arweave wallet json file [here](https://faucet.arweave.net/).

3. If you're going to use AR to pay for transactions, get AR tokens. I've been using gate.io despite the high withdrawal fees and KYC delays.

4. If you're going to use SOL, get a [Solana wallet](https://docs.solana.com/wallet-guide/cli) json file and transfer some SOL to it.

## Usage

If you're uploading more than one file, you should pretty much always be using bundles. Bundles take multiple files and package them together in a single transaction. This is better than uploading multiple individual files because you only have to wait for one transaction to be confirmed. Once the bundle transaction is confirmed, all of your files will be available. Larger transactions with larger rewards are more attractive to miners, which means a larger bundled transaction is more likely to get written quickly than a bunch of smaller individual ones.

Arloader accepts file glob patterns and defaults to creating a bundle for your files.

Arweave gateways only index bundles up to 250MB, so if tjhe aggregate size of all your files is greater than that, you should create multiple bundle transacions each less than 250MB.

1. To get an estimate of the cost of uploading your files run

```
arloader estimate "<GLOB>"
```

Make sure to include quotes around your glob patterns, otherwise your shell will expand them into a list of files. Arloader expects a glob pattern, not a list of files.

2. To upload your files run

```
arloader upload "<GLOB>" --log-dir "<LOG_DIR>"
```

A json status object gets written to `LOG_DIR` with a file name of `txid_<TXID>.json` that has the transaction id, reward and creation time in it. A manifest file will also be written with a file name of `manifest_<TXID>.json`. This has all of the ids of the files that were included in your bundle it. This manifest was automtically included in your bundle, which means that in addition to being available individually at `https://arweave.net/<BUNDLE_ITEM_ID>`, they will also be available at `https://arweave.net<MANIFEST_ID>/<FILE_PATH>` where `MANIFEST_ID` is the id of the manifest item included in the bundle and `FILE_PATH` is the relative file path of the file included in the `GLOB` pattern you specified. The manifest file is named with the bundle transaction id so you can match them up. `MANIFEST_ID` gets printed out following the upload command and can also be found in the manifest json file in `LOG_DIR` at the `id` key.

```
 manifest                                     id                                           status     confirms
--------------------------------------------------------------------------------------------------------------
 aHYRHIQg2BRqzQcyfYdG7vvsbZZZLkmsUJ8SsDQPsmE  a3USnDu6Goq2O6ndbhtornjDPM3nk9v61E-Oklzgle8  Submitted         0

Uploaded 10 files in 1 bundle transaction. Run `arloader raw-status a3USnDu6Goq2O6ndbhtornjDPM3nk9v61E-Oklzgle8` to confirm status.

Files will be available at https://arweave.net/<bundle_item_id> once the bundle transaction has been confirmed.

They will also be available at https://arweave.net/aHYRHIQg2BRqzQcyfYdG7vvsbZZZLkmsUJ8SsDQPsmE/<file_path>.
```

## Usage with SOL

You can use SOL to pay for your tranactions without going through the hassle of procuring AR tokens.

Arloader usage is pretty much exactly the same as above, with the addtion of the `--with-sol` flag.

1. To get an estimate of the cost of uploading your files run

```
arloader estimate "<GLOB>" --with-sol
```

2. To upload your files run

```
arloader upload "<GLOB>" --log-dir "<LOG_DIR>" --with sol
```

This will create the same bundle that gets created without using SOL and then goes out to an api to get your transaction signed. Once the SOL payment transaction has gone through, the signature comes back from the api and gets added to your bundle transaction and then your transaction. Then it gets uploaded directly to the [arweave.net] gateway from your computer.

## Reward Multiplier

Arweave is limited to approximately 1,000 transactions every two minutes so if you happen to submit your transaction at a time when there are a lot of pending transactions, it may take longer to get written, or if there are enough more attractive transaction, i.e, with higher rewards, it may not get written at all. To check the current number of pending transactions, run 

```
arloader pending
```
and that will print the number of pending transctions every second for one minute.

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

Given that Arloader bundles by default, your transaction is hopefully relatively attractive and you don't need to increase the reward to get it written in a timely fashion. However, if you see that there are a lot of transaction pending and you want to be sure your transaction goes through quickly, you can adjust the reward with `--reward-multipler` followed by something tha can be parsed as a float between `1.0` and `10.0`. The reward included in your transaction will then be multiplied by this factor when it gets submitted. Similar to the `--with-sol` flag, you can add `--reward-multipler` to both `estimate` and `upload` commands.

## Usage without Bundles

You can add the `--no-bundle` flag if for some reason you want to create individual transactions. This works with both `estimate` and `upload` commands. In that case individual status objects are written to `LOG_DIR` and you can run `update` status to update them from the network and `status-report` for a count of transactions by status.