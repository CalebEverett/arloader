## NFT Usage

The instructions were more relevant before the introduction of the `--upload-nfts` and `update-nft-status` commands, but they may be useful to those looking for more details on how nft files get uploaded to Arweave or to those who may only need to complete one of the steps.

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
arloader write-metaplex-items <GLOB> --manifest-path <MANIFEST_PATH> --log-dir <LOG_DIR>
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