# Changelog

All notable changes starting with v0.1.34 to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# v0.1.49 (2021-12-16)
- **changed:** removed file based link from metadata files property when not passing `--link-file` to `update-metadata`.

# v0.1.48 (2021-12-15)
- **added:** added examples to cli `--help`.
- **removed:** remove Windows build.

# v0.1.46 (2021-12-15)
- **added:** added `--reupload` command for bundles. You specify `--file-paths` and `--log-dir` and the command reuploads any files not included in the statuses in `--log-dir` and any files with a status included in `--statuses` or fewer confirmations than `--max-confirms`.
- **changed:** changed the way file paths are specified from glob strings to file paths. the `--glob` argument has been replaced by `--file-paths`. This new approach has a better user interface and is more robust across operating systems. Instead of avoiding shell expansion by wrapping glob arguments in quotes, the new approach takes advantage of it. It is now possible to specify an individual file path, a list of file paths separated by spaces or a glob pattern that gets expanded by the shell into multiple space separated file paths.
- **changed:** renamed `--upload-filter` subcommand to `--re-upload`.
- **changed:** refactored `--update-status` to include a long name for `<GLOB>`, `--glob`, and to make it required with `--no-bundle`.

# v0.1.45 (2021-12-13)
- **changed:** cleaned up cli text.
- **added:** added message to handle error when Solana network is unavailable.

# v0.1.44 (2021-12-13)
- **fix:** duration on retry sol service too long.

# v0.1.43 (2021-12-12)
- **fix:** bug adding trailing slash to glob strings in addition to directory strings.
- **changed:** made `--log-dir` an optional argument for `upload`, defaulting instead to creating a sub directory in the parent dir of the first file matching `glob` named `arloader_<RAND_CHARS>`.
- **changed:** removed `--log-dir` argument from `write-metaplex-items` in favor of writing to same parent director as `--manifest-path`.
- **changed:** removed requirement to pass `--log-dir` ahead of value in commands where it is the first argument, including `upload-manifest` and `update-status`.

# v0.1.42 (2021-12-11)
- **fixed:** `upload_nfts` example to create `target/examples/upload_nfts` 

# v0.1.41 (2021-12-11)
- **added:** `--upload-nfts` command that automates uploading of pairs of assets and metadata files, including updating metadata files with links to uploaded assets. You can now provide a glob pattern matching your asset files and the complete process will run, returning a manifest file with links to your uploaded metadata files that can be included in your on chain token metadata.
- **added:** `--update-nft-status` command that reports on status for all nft uploads, assets, metadata and manifests.
for both assets and metadata.
- **added:** `examples/upload_nfts`.
- **changed:** manifest paths now just include just file name instead of the full relative path.
- **changed:** sol service now has private rpc.

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