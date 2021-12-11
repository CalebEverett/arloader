# Changelog

All notable changes starting with v0.1.34 to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# v0.1.41 (2021-12-11)
- **changed:** manifest paths now just include file name instead of the full relative paths

# v0.1.40 (2021-12-10)
- **changed:** buffer post requests to `chunk/` endpoint.
- **changed:** `--bundle-size` units from bytes to megabytes.
- **changed:** retry solana transaction.
- **changed:** handle errors from getting Arweave and oracle prices.
- **breaking:** remove `--chunk-files`

# v0.1.39 (2021-12-09)
- **added:** command to chunk files `--chunk-files`
- **added:** `upload_large_bundles` example

# v0.1.38 (2021-12-09)
- **changed:** reduced `--with-sol` cost to 10,000 lamports per transaction.

# v0.1.37 (2021-12-08)
- **added:** [benchmarks](https://calebeverett.github.io/arloader/)
- **changed:** now able to pass `--ar-default-keypair` with `--with-sol` to use a default keypair instead of connecting a blank wallet. This will mean that data items are owned by the default wallet instead of a user wallet, but since data uploaded to Arweave is immutable, the convenience of not having to connect a wallet may outweigh this potential drawback. It is still possible to connect an AR wallet when you pass `--with-sol` to specify the owner of the Arweave transactions funded with SOL.

# v0.1.36 (2021-12-06)
- **changed:** refactored commands to only require wallets for upload transactions
- **added:** expansion of "~" in paths to home user directory

# v0.1.35 (2021-12-04)
- **changed:** more nits on the docs
- **changed:** alphabetized `arloader::commands`

# v0.1.34 (2021-12-04)
- **added:** `command_write_metaplex_items` to write links to json file formatted for use by metaplex candy machine to create NFTs
- **changed:** moved cli command functions from `main` to separate `commands` module