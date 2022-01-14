## Multiple Asset Files

There isn't a single command you can run if you have multiple media files per NFT, i.e., and mp4 and a png file. But you can run separate commands for each to get all of your files uploaded with your metadata files updated to include links to all of your media files.

Here is an example from a an upload of a series of glb and png files

1. `arloader upload *.glb --sol-keypair-path ~/.config/solana/wallet.json --with-sol --ar-default-keypair --bundle-size 100 --reward-multiplier 2`
2. `arloader upload-manifest  arloader_EsBWe5NTZ8E --sol-keypair-path ~/.config/solana/wallet.json --with-sol --ar-default-keypair --reward-multiplier 2`
3. `arloader update-metadata *.glb --manifest-path arloader_EsBWe5NTZ8E/manifest_2LpuzxUYEmKl5huvPChgoZ3YcAKNnpUoh-Jyq1EoB-9.json`
4. `arloader upload *.png --sol-keypair-path ~/.config/solana/wallet.json --with-sol --ar-default-keypair --reward-multiplier 2`
5. `arloader upload-manifest arloader_l00ydMAOy7E --sol-keypair-path ~/.config/solana/wallet.json --with-sol --ar-default-keypair --reward-multiplier 2`
6. `arloader update-metadata *.png --manifest-path arloader_l00ydMAOy7E/manifest_rJiNPRYtu-tsMYP5wyDU4L-4Qu8HFaL2QMHn7Oq1BGn.json  --update-image`
7. `arloader upload *.json --sol-keypair-path ~/.config/solana/wallet.json --with-sol --ar-default-keypair --reward-multiplier 2`
8. `arloader upload-manifest arloader_XPlkYp5zadw --sol-keypair-path ~/.config/solana/wallet.json --with-sol --ar-default-keypair --reward-multiplier 2`
9. `arloader write-metaplex-items *.json --manifest-path arloader_XPlkYp5zadw/manifest_lHEHx4S-m21EVHXkLYJZjyGqwAXk4Xmn1HR5Mjk55zd.json`
