[![crates.io](https://img.shields.io/crates/v/arloader.svg)](https://crates.io/crates/arloader)
[![build status](https://github.com/CalebEverett/arloader/actions/workflows/build.yml/badge.svg)](https://github.com/CalebEverett/arloader/actions/workflows/build.yml)
[![docs.rs](https://img.shields.io/docsrs/arloader)](https://docs.rs/arloader)
![Crates.io](https://img.shields.io/crates/l/arloader)

# arloader

Command line application and client for uploading files to [Arweave](https://www.arweave.org/). Arweave stores documents and applications forever.

Upload gigabytes of files with one command. Files are read and posted to [arweave.net](https://arweave.net) asynchronously with computationally intensive bundle preparation performed in parallel on multiple threads.

## Potential Issue with Transactions Uploaded Prior to Version 1.51
The way arloader was formatting transactions for upload was not entirely compatible with the Arweave protocol prior to version 1.51. For transactions bigger than 256 KB it is possible that even though your transactions are visible and are showing more than 25 confirmations that they were not written to the Arweave blockchain. If you would like assistance determining whether your transactions were impacted, please open an issue and I will be happy to help, including paying for any necessary re-uploading.

## Contents
* [Installation](#installation)
* [NFT Usage](#nft-usage)
* [General Usage](#general-usage)
* [Usage with SOL](#usage-with-sol)
* [Reward Multiplier](#reward-multiplier)
* [Usage without Bundles](#usage-without-bundles)
* [Benchmarks](#benchmarks)
* [Pricing Comparison](#pricing-comparison)
* [Roadmap](#roadmap)

## Discounted Usage with SOL
 Usage with SOL is currently essentially free. The cost per transaction is 10,000 lamports (~$0.002) and includes the Solana network fee of 5,000 lamports.

## Installation

1. The easiest way to use arloader is to download the binary for your system (Linux or Mac) from the [releases on github](https://github.com/CalebEverett/arloader/releases).

You can also install from [crates.io](https://crates.io) once you have [rust installed](https://www.rust-lang.org/tools/install) with the nightly toolchain.

```
rustup default nightly
cargo install arloader
```

2. Get an Arweave wallet json file [here](https://faucet.arweave.net/).

3. If you're going to use AR to pay for transactions, [get AR tokens](https://arweave.news/how-to-buy-arweave-token/).

4. If you're going to use SOL, get a [Solana wallet](https://docs.solana.com/wallet-guide/cli) json file and transfer some SOL to it.

## NFT Usage

### Create Upload Folder
 Put your assets and associated metadata files with `.json` extension in a folder by themselves. You can use any kind of file you want. Arloader automatically adds a content type tag to your upload so that browsers will handle it correctly when accessed from Arweave.
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

### Upload Assets
If you want to fund transactions with SOL, run the command below where `<FILE_PATHS>` matches your asset files.
```
arloader upload-nfts <FILE_PATHS> --with-sol --sol-keypair-path <SOL_KEYPAIR_PATH> --ar-default-keypair
```

For example, if you were uploading mp4 files as your assets and they were in a folder called `path/to/my/assets` and the path to your SOL keypair was `path/to/my/solkeypair.json`, you would enter:

```
arloader upload-nfts path/to/my/assets/*.mp4 --with-sol --sol-keypair-path path/to/my/solkeypair.json --ar-default-keypair
```

To fund transactions with AR, instead run:
```
arloader upload-nfts <FILE_PATHS> --ar-keypair-path <AR_KEYPAIR_PATH>
```

This will first upload your assets, logging statuses to a newly created directory named `arloader_<RANDOM_CHARS>` in the folder where the assets are located.

Then a manifest file will be created from the logged statuses and uploaded. A manifest is a special file that Arweave uses to access your files by their names, relative to the id of the manifest transaction: `https://arweave.net/<MANIFEST_ID>/<FILE_PATH>`. You'll still be able to access your files by their id at `https://arweave.net/<BUNDLE_ITEM_ID>`, but creating and uploading a manifest gives you the option of using either. Once uploaded, the manifest file itself can be accessed online at `https://arweave.net/tx/<MANIFEST_ID>/data.json`.

#### Update Metadata and Upload 
Next your metadata files will be updated with links to the uploaded assets. Arloader adds or replaces the `image` and `files` keys in your metadata `.json` files with the newly created links. It defaults to using the id link, `https://arweave.net/<BUNDLE_ITEM_ID>`, for the `image` key and updates the `files` key to include both links. If you prefer to use the file path based link, `https://arweave.net/<MANIFEST_ID>/<FILE_PATH>`, for the `image` key, you can pass the `--link-file` flag to the `upload-nfts` command.

After your metadata files have been updated, they will be uploaded, followed by the creation and upload of a manifest file for your metadata  files.

### Get Links to Uploaded Metadata

Once everything has been uploaded, the links to your uploaded metadata files, to be included in your on chain token metadata, can be found in `arloader_<RAND_CHAR>/metadata/manifest_<TXID>.json`.

```json
{
    "0.json": {
        "id": "ScU9mEuKBbPX5o5nv8DZkDnZuJbzf84lyLk-uLVDqNk",
        "files": [
            {
                "uri": "https://arweave.net/ScU9mEuKBbPX5o5nv8DZkDnZuJbzf84lyLk-uLVDqNk",
                "type": "application/json"
            },
            {
                "uri": "https://arweave.net/fo9P3OOq78REajk48vFWbKfIhw6mDzgjANQIh3L7Njs/0.json",
                "type": "application/json"
            }
        ]
    },
    "1.json": {
        "id": "8APeQ5lW0-csTcBaGdPBDLAL2ci2AT9pTn2tppGPU_8",
        "files": [
            {
                "uri": "https://arweave.net/8APeQ5lW0-csTcBaGdPBDLAL2ci2AT9pTn2tppGPU_8",
                "type": "application/json"
            },
            {
                "uri": "https://arweave.net/fo9P3OOq78REajk48vFWbKfIhw6mDzgjANQIh3L7Njs/1.json",
                "type": "application/json"
            }
        ]
    },
```

If you are creating your NFTs with the [Metaplex Candy Machine](https://docs.metaplex.com/create-candy/introduction), you can create a json file with links it that you can copy and paste into your candy machine config by running the command below. `<FILE_PATHS>` can match your either your asset or metadata files.

```
arloader write-metaplex-items <FILE_PATHS> --manifest-path <MANIFEST_PATH>
```

This will write a file named `metaplex_items_<MANIFIEST_ID>.json` to the same directory as `<MANIFEST_PATH>`. As with updating metadata files, Arloader defaults to using the id based link, `https://arweave.net/<BUNDLE_ITEM_ID>`, but 
you can use the file based link, `https://arweave.net/<MANIFEST_ID>/<FILE_PATH>`, by passing the `--link-file` flag.

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
### Confirm All Transactions

Before you create your tokens, make sure that all of your transactions have been confirmed at least 25 times. Run the command below where `<LOG_DIR>` refers to the automatically created directory in your assets folder that begins with `arloader_`.

The primary reason for transactions having a status of `NotFound` is that they were rejected by miners for not having a big enough reward. See [Reward Multiplier](#reward-multiplier) for instructions on increasing the reward by passing the optional `--reward-multiplier` argument.

```
arloader update-nft-status <LOG_DIR>
```

```
Updating asset bundle statuses...

 bundle txid                                   items      KB  status       confirms
------------------------------------------------------------------------------------
 kmgLCgV-dB-DGML8cvFwuP3a-ZKedz7nyDuEsqYPTis      10     980  Confirmed          60
Updated 1 statuses.


Updating metadata bundle statuses...

 bundle txid                                   items      KB  status       confirms
------------------------------------------------------------------------------------
 kmgLCgV-dB-DGML8cvFwuP3a-ZKedz7nyDuEsqYPTis      10     980  Confirmed          60
Updated 1 statuses.


Updating asset manifest status...

 id                                           status     confirms
------------------------------------------------------------------
 URwQtoqrbYlc5183STNy3ZPwSCRY4o8goaF7MJay3xY  Confirmed        60


Updating metadata manifest status...

 id                                           status     confirms
------------------------------------------------------------------
 fo9P3OOq78REajk48vFWbKfIhw6mDzgjANQIh3L7Njs  Confirmed        57
 ```

## General Usage

If you're uploading more than one file, you should pretty much always be using bundles. Bundles take multiple files and packages them together in a single transaction. This is better than uploading multiple individual files because you only have to wait for one transaction to be confirmed. Once the bundle transaction is confirmed, all of your files will be available. Larger transactions with larger rewards are more attractive to miners, which means a larger bundled transaction is more likely to get written quickly than a bunch of smaller individual ones.

Arloader will create as many bundles as necessary to upload all of your files. Your files are read asynchronously, bundled in parallel across multiple threads and then posted to [arweave.net](https://arweave.net). Arloader supports bundle sizes up to 200 MB, but the default bundle size is 10 MB, which makes it possible to post full bundle size payloads to the `/tx` endpoint instead of in 256 KB chunks to the `/chunk` endpoint. This should work fine for individual files up to 10 MB. If your files sizes are bigger than 10 MB (but smaller than 200 MB), you can specify a larger bundle size with the `--bundles-size` argument - `--bundle-size 100` to specify a size of 100 MB, for example.

### Estimate Cost
To get an estimate of the cost of uploading your files run

```
arloader estimate <FILE_PATHS>
```

`<FILE_PATHS>` can be a glob, like `path/to/my/files/*.png`, or one or more files separated by spacees, like `path/to/my/files/2.mp4 path/to/my/files/0.mp path/to/my/files/2.mp`.

### Upload
To upload your files run

```
arloader upload <FILE_PATHS>
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

A status object gets written to a json file named `<TXID>.json` in a newly created sub directory in the parent folder of the first file in `<FILE_PATHS>`. The folder will be named `arloader_<RAND_CHAR>`. You can specify an existing folder to write statuses to by passing the `--log-dir` argument.

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

### Check Status
After uploading your files, you'll want to check on their status to make sure the have been uploaded successfully and that they ultimately are confirmed at least 25 times before you can be absolutely certain they have been permanently uploaded.

```
arloader update-status <LOG_DIR>
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

### Re-Upload
If you find that not all of your transactions have a status of `Confirmed` or that the number of confirmations is below 25 after some period of time, you will want to re-upload your transactions with the following command:

```
arloader repupload <FILE_PATHS> --log-dir <LOG_DIR> --statuses <STATUSES> --max-confirmations <MAX_CONFIRMATIONS>
```
This will first check to make sure that all of the files in `<FILE_PATHS>` are included in the status objects in `<LOG_DIR>`, adding them to the list of files to be reuploaded. Then it will filter the paths in the status objects based on `<STATUSES>` and `<MAX_CONFIRM>`. You can provide multiple statuses and max confirmations will only re-upload transactions with fewer than `<MAX_CONFIRM>` confirmations.

For example, if had uploaded a bunch of jpegs that were in `my/images` and statuses had been logged to `my/images/arloader_hehQJu-RJpo`, if you wanted to re-upload transactions with either a status of `NotFound` or `Pending`, you would run:

```
arloader reupload my/images/*.jpeg --log-dir my/images/arloader_hehQJu-RJpo --statuses NotFound Pending
```

If you wanted to reupload anything with less than 25 confirmations, you would run:

```
arloader reupload my/images/*.jpeg --log-dir my/images/arloader_hehQJu-RJpo --max-confirms 25
```


### Create Manifest
Once you have a sufficient number of confirmations of your files, you may want to create a manifest file, which is used by the Arweave gateways to provide relative paths to your files. In order to do that, you run

```
arloader upload-manifest <LOG_DIR>
```
where `<LOG_DIR>` is the directory containing your bundle status json files. This will go through and consolidate the paths from each of the bundles, create a consolidated manifest, upload it to Arweave and then write a file named `manifest_<TXID>.json`to `<LOG_DIR>`. Once the transaction uploading the manifest has been confirmed, you will be able to access your files and both `https://arweave.net/<BUNDLE_ITEM_ID>` and `https://arweave.net/<MANIFEST_ID>/<FILE_PATH>`  where `MANIFEST_ID` is the id of the manifest transaction and `FILE_PATH` is the relative path of the file included with the `upload` command.

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
arloader get-status <MANIFEST_ID>
```

## Usage with SOL

You can use SOL to pay for your transactions without going through the hassle of procuring AR tokens.

Arloader usage is pretty much exactly the same as above, with the addition of the `--with-sol` flag.

1. To get an estimate of the cost of uploading your files run

```
arloader estimate <FILE_PATHS> --with-sol
```

2. To upload your files run

```
arloader upload <FILE_PATHS> --with sol --ar-default-keypair
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

You can add the `--no-bundle` flag if for some reason you want to create individual transactions. This works with both `estimate` and `upload` commands. In that case individual status objects are written to `<LOG_DIR>` and you can run `update-status` to update them from the network and `status-report` for a count of transactions by status.

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

## Pricing Comparison

The table below compares the cost to upload 256 MB of files to Arweave as individual files versus in a single bundle and includes a comparison to the bundlr network as well. This was run on 2021-12-17. You can run `cargo run --example pricing` to print a table with current prices.

```
Price in USD to upload 256 MB of files of various sizes in KB ($48.33 USD per AR):

file size | num files | arweave | bundlr | arweave total | bundlr total | arweave bundle
-----------------------------------------------------------------------------------------
        1      262144    0.0019   0.0000        508.3164         5.1784           1.9616
       32        8192    0.0019   0.0003         15.8849         2.5892           1.9616
      256        1024    0.0019   0.0025          1.9856         2.5892           1.9616
     1024         256    0.0077   0.0101          1.9676         2.5892           1.9616
     4096          64    0.0307   0.0405          1.9631         2.5892           1.9616
    16384          16    0.1226   0.1618          1.9620         2.5892           1.9616
```

## Roadmap

- [x] Bundle size unit in MB
- [x] Handle error on pricing look up
- [x] Buffer chunks post stream
- [x] Add upload nft project example
- [x] Point at folder of assets and json and get back links to uploaded metadata
- [x] Clean up handling of paths
- [x] Re upload bundles
- [ ] Add super simple single upload, return link
- [ ] Progress indicators for longer running processes
- [ ] Output in metaboss format, or include in metaplex cli
- [ ] Implement bundlr
- [ ] Async benchmarking, including reading files from disk
- [ ] Bundlr benchmarking
- [ ] Report on missing files in `list-status` and `update-status` commands
- [ ] Include duration in completion output.


