[![build status](https://github.com/CalebEverett/arload/actions/workflows/build.yml/badge.svg)](https://github.com/CalebEverett/arload/actions/workflows/build.yml)

# arload

Command line application and library for uploading files to Arweave.

## Usage

1. Get an Arweave wallet json file and purchase AR tokens.

2. Install
```
git clone git@github.com:CalebEverett/arload.git
cd arload
cargo build
```

3. Get an estimate of how much it is going to cost to store your files:

```
cargo run estimate "tests/fixtures/*.png"
```

```
The price to upload 10 files with 18265 total bytes is 9071040 winstons ($0.00045219137).
```
4. Check your wallet balance
```
cargo run wallet-balance
```
```
Wallet balance is 1549658342531 winstons ($49.82). At the current price of 444274406 winstons ($0.0221) per MB, you can upload 3488 MB of data.
```
5. Upload your files, specifying a `log_dir` to write statuses to so you check them later. Make sure to wrap your paths in quotes to avoid your shell expanding them into lists of files.
```
cargo run upload "tests/fixtures/[1-5]*.png" --log-dir target/tmp/
```

```
 path                            id                                           status     confirms
-------------------------------------------------------------------------------------------------
 tests/fixtures/1.png            s0BdmZ6KDfvjWojSr-BW7RnEcJaC44yNboQsL4V4o2c  Submitted         0
 tests/fixtures/2.png            jLBrbCm5gGpxomIFh0GBCxxYkelF-CPaxbxy8hUW2kE  Submitted         0
 tests/fixtures/3.png            rgudrIf_hVF_VRz3-el9-kVaki8U4OEfxTEYEoZ6eME  Submitted         0
 tests/fixtures/4.png            GK6FieopUSDQ7MLPJ1GvoO9227BhdcY8c0AewPF_ZhY  Submitted         0
 tests/fixtures/5.png            Frj44HRVdfvz98x7zR63sTkVLa7I159HI6IsphLHhQc  Submitted         0
 Uploaded 5 files. Run `update-status "tests/fixtures/[1-5]*.png" --log-dir target/tmp/` to confirm transaction(s).
 ```

Uploads are async and utilize streams, although the default buffer size is 1. To increase the buffer size, pass the flag `--buffer` followed by the size.

6. Your transactions may not be written write away, depending on network traffic and how long it takes miners to add them to the blockchain. To check the status of your transactions run:

```
cargo run update-status "tests/fixtures/[1-5]*.png" --log-dir target/tmp/
```
```
 path                            id                                           status     confirms
-------------------------------------------------------------------------------------------------
 tests/fixtures/1.png            s0BdmZ6KDfvjWojSr-BW7RnEcJaC44yNboQsL4V4o2c  Confirmed         3
 tests/fixtures/2.png            jLBrbCm5gGpxomIFh0GBCxxYkelF-CPaxbxy8hUW2kE  Confirmed         2
 tests/fixtures/3.png            rgudrIf_hVF_VRz3-el9-kVaki8U4OEfxTEYEoZ6eME  Confirmed         1
 tests/fixtures/4.png            GK6FieopUSDQ7MLPJ1GvoO9227BhdcY8c0AewPF_ZhY  Confirmed         1
 tests/fixtures/5.png            Frj44HRVdfvz98x7zR63sTkVLa7I159HI6IsphLHhQc  Confirmed         1
 ```

6. To get a summary report of the status of your uploads, run:
```
cargo run status-report "tests/fixtures/[1-5]*.png" --log-dir target/tmp/
```
```
 status                count
-----------------------------
 Submitted                 0
 Pending                   0
 NotFound                  0
 Confirmed                 5
-----------------------------
 Total                     5
 ```

7. If you wanted to determine whether any of your upload transactions were unsuccessful - you can filter the statuses of your files by status and by number of confirmations.

```
cargo run list-status "tests/fixtures/[1-5]*.png" --log-dir target/tmp/ -max-confirms 1
```

```
 path                            id                                           status     confirms
-------------------------------------------------------------------------------------------------
 tests/fixtures/3.png            rgudrIf_hVF_VRz3-el9-kVaki8U4OEfxTEYEoZ6eME  Confirmed         1
 tests/fixtures/4.png            GK6FieopUSDQ7MLPJ1GvoO9227BhdcY8c0AewPF_ZhY  Confirmed         1
 tests/fixtures/5.png            Frj44HRVdfvz98x7zR63sTkVLa7I159HI6IsphLHhQc  Confirmed         1
Found 3 files matching filter criteria.
```

8. If you then want to re-upload some of your files, you can run 
```
cargo run upload-filter "tests/fixtures/[1-5]*.png" --log-dir target/tmp/ --max-confirms 1 --buffer 3
```
with the same filter criteria you used above.
```
 path                            id                                           status     confirms
-------------------------------------------------------------------------------------------------
 tests/fixtures/4.png            NaB-1fZzzu1ntIe7APxhXZQlmEWz6PDi3oKkfIqYsLg  Submitted         0
 tests/fixtures/3.png            MUaEgj2qzrRfIEgRXyJJFw-ilKk7YvTVqBMnT6K7kaM  Submitted         0
 tests/fixtures/5.png            NLkmVtUAsphbmjNYMWTMMbZ7Kd6l40Fsj4MagKDkgRA  Submitted         0
Uploaded 3 files. Run `update-statuses` to confirm acceptance.

and then if you run 
```
cargo run update-status "tests/fixtures/[1-5]*.png" --log-dir target/tmp/
```
you can see that your files have been re-uploaded.
```
```
 path                            id                                           status     confirms
-------------------------------------------------------------------------------------------------
 tests/fixtures/1.png            s0BdmZ6KDfvjWojSr-BW7RnEcJaC44yNboQsL4V4o2c  Confirmed        14
 tests/fixtures/2.png            jLBrbCm5gGpxomIFh0GBCxxYkelF-CPaxbxy8hUW2kE  Confirmed        14
 tests/fixtures/3.png            cA2ZrJSzEH4dJWQgTIrk7uVd9WSXPJTDBsONWk779No  Pending           0
 tests/fixtures/4.png            OTSp9uxU02ZlTDnSuZT70CzFl1VCN5bveW2-W7hrUZ0  Pending           0
 tests/fixtures/5.png            nWChdcmmIBLYHAjD90IuRVEsd3D3-WzUaknskK9f790  Pending           0
Updated 5 statuses.
```
9. If you want to get the transaction ids for all your uploads, the status objects are written to json files in `log_dir` or you can get json output by passing `json` to the `output flag`.

```
cargo run list-status "tests/fixtures/[1-5]*.png" --log-dir target/tmp/ --output json
```

```
{
  "id": "s0BdmZ6KDfvjWojSr-BW7RnEcJaC44yNboQsL4V4o2c",
  "status": "Confirmed",
  "file_path": "tests/fixtures/1.png",
  "created_at": "2021-10-31T08:22:48.026149300Z",
  "last_modified": "2021-10-31T09:00:56.406929800Z",
  "reward": 2150860,
  "block_height": 801254,
  "block_indep_hash": "QixYntgtTrp83jB97bxyJEKsihZnIACZ7mowGmTxRPbqUXuCHMhM9YywwX5Y6rpO",
  "number_of_confirmations": 14
},
{
  "id": "jLBrbCm5gGpxomIFh0GBCxxYkelF-CPaxbxy8hUW2kE",
  "status": "Confirmed",
  "file_path": "tests/fixtures/2.png",
  "created_at": "2021-10-31T08:22:50.477367700Z",
  "last_modified": "2021-10-31T09:00:57.849300500Z",
  "reward": 2150860,
  "block_height": 801255,
  "block_indep_hash": "4qPA3zuROGZ6HKkjf7YtUJxyEBJaUMnZQojCSknuGg8g5bgs6aZGJUSXEp-maSyT",
  "number_of_confirmations": 14
},
{
  "id": "cA2ZrJSzEH4dJWQgTIrk7uVd9WSXPJTDBsONWk779No",
  "status": "Pending",
  "file_path": "tests/fixtures/3.png",
  "created_at": "2021-10-31T09:00:43.362172200Z",
  "last_modified": "2021-10-31T09:00:59.089684200Z",
  "reward": 2150860
},
```

Keep in mind that the `list-status` command is just reading statuses from disk. You need to run `update-status` to get them updated from the network.