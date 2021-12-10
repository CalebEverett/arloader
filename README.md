[![crates.io](https://img.shields.io/crates/v/arloader.svg)](https://crates.io/crates/arloader)
[![build status](https://github.com/CalebEverett/arloader/actions/workflows/build.yml/badge.svg)](https://github.com/CalebEverett/arloader/actions/workflows/build.yml)
[![docs.rs](https://img.shields.io/docsrs/arloader)](https://docs.rs/arloader)
![Crates.io](https://img.shields.io/crates/l/arloader)

# arloader

Command line application and library for effortlessly uploading files to [Arweave](https://www.arweave.org/). Arweave enables you to store documents and applications forever.

Upload gigabytes of files with one command specifying a glob pattern to match files against. Files are read and posted to [arweave.net](https://arweave.net) asynchronously with computationally intensive bundle preparation performed in parallel across multiple threads.

## Contents
* [Installation](#installation)
* [Benchmarks](#benchmarks)
* [NFT Usage](#nft-usage)
* [General Usage](#general-usage)
* [Usage with SOL](#usage-with-sol)
* [Reward Multiplier](#reward-multiplier)
* [Usage without Bundles](#usage-without-bundles)

## Discounted Usage with SOL
 Usage with SOL is currently essentially free. The cost per transaction is 10,000 lamports (~$0.002), including the Solana network fee of 5,000 lamports.

## Installation

1. The easiest way to use arloader is to download the binary for your system (Linux, Mac or Windows) from the [releases on github](https://github.com/CalebEverett/arloader/releases).

You can also install from [crates.io](https://crates.io) once you have [rust installed](https://www.rust-lang.org/tools/install) with the nightly toolchain.

```
rustup default nightly
cargo install arloader
```

2. Get an Arweave wallet json file [here](https://faucet.arweave.net/).

3. If you're going to use AR to pay for transactions, get AR tokens. I've been using [gate.io](https://gate.io) despite the high withdrawal fees and KYC delays.

4. If you're going to use SOL, get a [Solana wallet](https://docs.solana.com/wallet-guide/cli) json file and transfer some SOL to it.

## Benchmarks

The table below shows the average duration required to create transactions across a range of file sizes and numbers of files. Detailed statistical analyses and charts can be found [here](https://calebeverett.github.io/arloader/) (numbers may vary slightly from those below).

For an NFT project with 10,000 tokens it would take 20 seconds to process the images if they were 256 KB. If the images were 4 MB, it would take approximately two minutes.


| File Size | Num Files | Total Size | Data Item | Data Items | Bundle | Transaction | Total | Per 1,000 |
| --------: | --------: | ---------: | --------: | ---------: | -----: | ----------: | ----: | --------: |
|     32 KB |       500 |         16 |         4 |        430 |     30 |          40 |   0.5 |       1.0 |
|    256 KB |       500 |        128 |         4 |        493 |    179 |         326 |   1.0 |       2.0 |
|      1 MB |       500 |        512 |         5 |        616 |    903 |        1033 |   2.6 |       4.1 |
|      4 MB |       150 |        614 |        11 |        360 |   1050 |        1554 |   3.0 |      10.4 |
|     16 MB |       50  |        819 |        35 |        393 |   1403 |        2058 |   3.9 |      77.1 |


Benchmarks include only processing activity and exclude reading files from disk and uploading them to the network. Benchmarks were performed on an Intel(R) Core(TM) i7-8750H CPU @ 2.20GHz processor with 6 cores.

| Column | Description |
| --- | --- |
| Total Size | File size x number of files in megabytes.|
| Data Item | Time in milliseconds required to create a single data item of the file size. The entails creating a merkle tree data root, generating an id from the deep hash algorithm and signing it.|
| Data Items | Time in milliseconds required to create data items for the number of files. Data items are processed in parallel using all available cores.|
| Bundle | Time in milliseconds required to create a single bundle from the data items. This entails serializing each of the data items and packing them together.|
| Transaction |Time in milliseconds required to create a transaction from the bundle. This entails creating a merkle tree data root, generating an id from the deep hash algorithm and signing it.|
| Total | Sum of the time required to create data items, bundle and transaction in seconds.|
| Per 1,000 | Extrapolation of total to 1,000 files.|

## NFT Usage

NFTs consist of an on-chain token, an asset (image, animation, video, or other digital media) and metadata describing the asset. Since on-chain storage is expensive, the token itself typically only includes a link to a metadata file stored off chain that includes a link to the asset stored off chain as well. Arweave is an excellent choice for storing assets and metadata since you only pay once and your files are stored forever. Neither you nor anyone else who might end up with your NFTs ever has to worry about funding storage in the future. Once uploaded to Arweave, your assets and metadata are stored forever!

In order to create your NFTs, you need your assets uploaded to Arweave, your metadata files to include links to the assets and finally, the updated metadata files to be uploaded to Arweave. Once these steps are completed and your upload transactions have been confirmed, you can use the links returned from uploading your metadata files to create your NFTs.

1. Upload your assets
2. Update your metadata files to include the links to your assets
3. Upload your metadata files
4. Get links to your uploaded metadata files to use in your NFTs

To start with, include both your assets and your metadata files in the same directory and make sure that the stems of your asset files match the stems of your metadata files.
```
├── 0.json
├── 0.png
├── 1.json
├── 1.png
├── 2.json
├── 2.png
├── 3.json
├── 3.png
├── 4.json
├── 4.png
├── 5.json
├── 5.png
```

Also create directories where you can log the statuses of your asset and metadata uploads. arloader will use these to provide updates on confirmation statuses and to write files with links to your uploaded files. The example below assumes that `status/assets/` and `status/metadata/` have been created in advance.

See [Token Metadata Standard](https://docs.metaplex.com/nft-standard) for details on the standard metadata format.

### Upload Assets

```
arloader upload "*.png" --log-dir "status/asset/"
```

At this point, you can also go ahead and create and upload a manifest file. A manifest is a special file that Arweave will use to access your files by their names relative to the id of the manifest transaction: `https://arweave.net/<MANIFEST_ID>/<FILE_PATH>`. You'll still be able to access your files at `https://arweave.net/<BUNDLE_ITEM_ID>`, but creating and uploading a manifest gives you the option of using either link. You'll also be able to use this file to automatically update your metadata files to include links to your uploaded asset files. 

```
arloader upload-manifest --log-dir "status/assets/" --reward-multiplier 2
```

Since this is a small transaction and you want to make sure it goes through, it's a good idea to increase the reward.

A version of the manifest named `manifest_<TXID>.json` will be written in the `status/assets/` directory.

```json
{
    "0.png": {
        "files": [
            {
                "type": "image/png",
                "uri": "https://arweave.net/BSvIAiwthQu_xwQBHn9FcgACaZ8ko4py5mqMNP4r-jM/0.png"
            },
            {
                "type": "image/png",
                "uri": "https://arweave.net/JQbz5py065lqaS_8R7NCtLcK2b-pSkkG6Je0OT8379c"
            }
        ],
        "id": "JQbz5py065lqaS_8R7NCtLcK2b-pSkkG6Je0OT8379c"
    },
    "1.png": {
        "files": [
            {
                "type": "image/png",
                "uri": "https://arweave.net/BSvIAiwthQu_xwQBHn9FcgACaZ8ko4py5mqMNP4r-jM/1.png"
            },
            {
                "type": "image/png",
                "uri": "https://arweave.net/Os-tEyRqdjwwyNo1mpLaPGu8_r3KbV-iNRH-aPtJFOw"
            }
        ],
        "id": "Os-tEyRqdjwwyNo1mpLaPGu8_r3KbV-iNRH-aPtJFOw"
    },
    
```

### Update Metadata

You can proceed with updating your metadata files, but just make sure that you've gotten 25 confirmations on everything - your assets, metadata and manifest files before you create your NFTs. You can check the number of confirmations by running:

```
arloader update-status --log-dir "status/asset/"
```

Also check your manifest confirmations by running:

```
arloader get-status <MANIFEST_ID>
```

If your metadata files have the same stem as your asset files and an extension of `json`, you can update the `image` and `files` keys from the newly created manifest file with the command below.

```
arloader update-metadata "*.png" --manifest-path <MANIFEST_PATH>
```

arloader defaults to using the id link (`https://arweave.net/<BUNDLE_ITEM_ID>`) for the `image` key and updates the `files` key to include both links. If you prefer to use the file path based link for the `image` key, you can pass the `--link-file` flag to the `update-metadata` command.

### Upload Metadata

Now that your metadata files include links to your uploaded assets, you're ready to upload your metadata files.

```
arloader upload "*.json" --log-dir "status/metadata/"
```

Go ahead and create and upload a separate manifest for your metadata files as well.

```
arloader upload-manifest --log-dir "status/metadata/"
```

Same thing as with your asset files, before creating your NFTs, you make sure that each of your metadata upload transactions has been confirmed at least 25 times.

```
arloader update-status --log-dir "status/metadata/"
```

And for your metadata manifest:

```
arloader get-status <MANIFEST_ID>
```

### Get Links to Uploaded Metadata

Once each of your transactions has been confirmed at least 25 times, you are good to go - grab the `manifest_<TXID>.json` file in `status/metadata/` and use the included links to create your NFTs!

If you happen to be creating your NFTs with the [Metaplex Candy Machine](https://docs.metaplex.com/create-candy/introduction), you can create a json file of links you can copy
and paste into your candy machine config by running the command below where `<GLOB>` is a pattern that will match your metadata files (something `*.json`).

```
arloader write-metaplex-items <GLOB> --manifest-path <MANIFEST_PATH> --log-dir <MANIFEST_PATH>
```

This will write a file named `metaplex_items_<MANIFIEST_ID>.json` to `<LOG_DIR>` with the format below that you can copy into the `items` key in your candy machine config. Arloader defaults to using the id based link (`https://arweave.net/<BUNDLE_ITEM_ID>`), but 
you can use the file based link (`https://arweave.net/<MANIFEST_ID>/<FILE_PATH>`), by passing the `--link-file` flag.

```json
{
        "0": {
            "link": "uri link",
            "name": "name",
            "onChain": false
        },
        "1": {
            "link": "uri link",
            "name": "name",
            "onChain": false
        },
```

## General Usage

If you're uploading more than one file, you should pretty much always be using bundles. Bundles take multiple files and packages them together in a single transaction. This is better than uploading multiple individual files because you only have to wait for one transaction to be confirmed. Once the bundle transaction is confirmed, all of your files will be available. Larger transactions with larger rewards are more attractive to miners, which means a larger bundled transaction is more likely to get written quickly than a bunch of smaller individual ones.

Arloader accepts file glob patterns and defaults to creating a bundle for your files.

Arloader will create as many bundles as necessary to upload all of your files. Your files are read asynchronously, bundled in parallel across multiple threads and then posted to [arweave.net](https://arweave.net). Arloader supports bundle sizes up to 200 MB, but the default bundle size is 10 MB, which makes it possible to post full bundle size payloads to the `/tx` endpoint instead of in 256 KB chunks to the `/chunk` endpoint. This should work fine for individual files up to 10 MB. If your files sizes are bigger than 10 MB (but smaller than 200 MB), you can specify a larger bundle size with the `--bundles-size` argument - `--bundle-size 100000000` to specify a size of 100 MB, for example.

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
where `<LOG_DIR>` is the directory containing your bundle status json files. This will go through and consolidate the paths from each of the bundles, create a consolidated manifest, upload it to Arweave and then write a file named `manifest_<TXID>.json`to `<LOG_DIR>`. Once the transaction uploading the manifest has been confirmed, you will be able to access your files and both `https://arweave.net/<BUNDLE_ITEM_ID>` and `https://arweave.net/<MANIFEST_ID>/<FILE_PATH>`  where `MANIFEST_ID` is the id of the manifest transaction and `FILE_PATH` is the relative path of the file included in the `GLOB` pattern you specified with the `upload` command.

```json
{
    "tests/fixtures/0.png": {
        "files": [
            {
                "type": "image/png",
                "uri": "https://arweave.net/BSvIAiwthQu_xwQBHn9FcgACaZ8ko4py5mqMNP4r-jM/tests/fixtures/0.png"
            },
            {
                "type": "image/png",
                "uri": "https://arweave.net/JQbz5py065lqaS_8R7NCtLcK2b-pSkkG6Je0OT8379c"
            }
        ],
        "id": "JQbz5py065lqaS_8R7NCtLcK2b-pSkkG6Je0OT8379c"
    },
    "tests/fixtures/1.png": {
        "files": [
            {
                "type": "image/png",
                "uri": "https://arweave.net/BSvIAiwthQu_xwQBHn9FcgACaZ8ko4py5mqMNP4r-jM/tests/fixtures/1.png"
            },
            {
                "type": "image/png",
                "uri": "https://arweave.net/Os-tEyRqdjwwyNo1mpLaPGu8_r3KbV-iNRH-aPtJFOw"
            }
        ],
        "id": "Os-tEyRqdjwwyNo1mpLaPGu8_r3KbV-iNRH-aPtJFOw"
    },
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

## Roadmap

- [x] Bundle size unit in MB
- [x] Handle error on pricing look up
- [ ] Stream chunks with buffer
- [ ] Re upload bundles
- [ ] Clean up handling of paths
- [ ] Progress indicators for longer running processes
- [ ] Output in metaboss format, or include in metaplex cli
- [ ] Point at folder of assets and json and get back links to uploaded metadata
- [ ] Implement bundlr
- [ ] Async benchmarking, including reading files from disk
- [ ] Bundlr benchmarking


