//! Functionality for creating and verifying signatures and hashing.

use crate::{
    error::Error,
    transaction::{Base64, DeepHashItem},
};
use jsonwebkey::JsonWebKey;
use log::debug;
use ring::{
    digest::{Context, SHA256, SHA384},
    rand::{self, SecureRandom},
    signature::{self, KeyPair, RsaKeyPair},
};
use std::fs as fsSync;
use std::path::PathBuf;
use tokio::fs;

/// Struct for for crypto methods.
pub struct Provider {
    pub keypair: RsaKeyPair,
    pub sr: rand::SystemRandom,
}

impl Default for Provider {
    fn default() -> Self {
        let jwk_parsed: JsonWebKey = DEFAULT_KEYPAIR.parse().unwrap();
        Self {
            keypair: signature::RsaKeyPair::from_pkcs8(&jwk_parsed.key.as_ref().to_der()).unwrap(),
            sr: rand::SystemRandom::new(),
        }
    }
}

impl Provider {
    /// Reads a [`JsonWebKey`] from a [`PathBuf`] and stores it as a [`signature::RsaKeyPair`] in
    /// the `keypair` property of [`Provider`] for future use in signing and funding transactions.
    pub async fn from_keypair_path(keypair_path: PathBuf) -> Result<Provider, Error> {
        debug!("{:?}", keypair_path);
        let data = fs::read_to_string(keypair_path).await?;

        let jwk_parsed: JsonWebKey = data.parse().unwrap();
        Ok(Self {
            keypair: signature::RsaKeyPair::from_pkcs8(&jwk_parsed.key.as_ref().to_der())?,
            sr: rand::SystemRandom::new(),
        })
    }
    /// Sync version of [`Provider::from_keypair_path`].
    pub fn from_keypair_path_sync(keypair_path: PathBuf) -> Result<Provider, Error> {
        let data = fsSync::read_to_string(keypair_path)?;

        let jwk_parsed: JsonWebKey = data.parse().unwrap();
        Ok(Self {
            keypair: signature::RsaKeyPair::from_pkcs8(&jwk_parsed.key.as_ref().to_der())?,
            sr: rand::SystemRandom::new(),
        })
    }

    /// Returns the full modulus of the stored keypair. Encoded as a Base64Url String,
    /// represents the associated network address. Also used in the calculation of transaction
    /// signatures.
    pub fn keypair_modulus(&self) -> Result<Base64, Error> {
        let modulus = self
            .keypair
            .public_key()
            .modulus()
            .big_endian_without_leading_zero();
        Ok(Base64(modulus.to_vec()))
    }
    /// Calculates the wallet address of the provided keypair according to [addressing](https://docs.arweave.org/developers/server/http-api#addressing)
    /// in documentation.
    ///```
    /// # use arloader::Arweave;
    /// # use ring::{signature, rand};
    /// # use std::{fmt::Display, path::PathBuf, str::FromStr};
    /// # use url::Url;
    /// #
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let arweave = Arweave::from_keypair_path(
    ///     PathBuf::from("tests/fixtures/arweave-key-7eV1qae4qVNqsNChg3Scdi-DpOLJPCogct4ixoq1WNg.json"),
    ///     Url::from_str("http://url.com").unwrap()
    /// ).await?;
    /// let calc = arweave.crypto.wallet_address()?;
    /// let actual = String::from("7eV1qae4qVNqsNChg3Scdi-DpOLJPCogct4ixoq1WNg");
    /// assert_eq!(&calc.to_string(), &actual);
    /// # Ok(())
    /// # }
    /// ```
    pub fn wallet_address(&self) -> Result<Base64, Error> {
        let mut context = Context::new(&SHA256);
        context.update(&self.keypair_modulus()?.0[..]);
        let wallet_address = Base64(context.finish().as_ref().to_vec());
        Ok(wallet_address)
    }

    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, Error> {
        let rng = rand::SystemRandom::new();
        let mut signature = vec![0; self.keypair.public_modulus_len()];
        self.keypair
            .sign(&signature::RSA_PSS_SHA256, &rng, message, &mut signature)?;
        Ok(signature)
    }

    /// Verifies that a message was signed by the public key of the Provider.key keypair.
    ///```
    /// # use ring::{signature, rand};
    /// # use arloader::crypto::Provider;
    /// # use std::path::PathBuf;
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let crypto = Provider::from_keypair_path(PathBuf::from("tests/fixtures/arweave-key-7eV1qae4qVNqsNChg3Scdi-DpOLJPCogct4ixoq1WNg.json")).await?;
    /// let message = String::from("hello, world");
    /// let rng = rand::SystemRandom::new();
    /// let signature = crypto.sign(&message.as_bytes())?;
    ///
    /// assert_eq!((), crypto.verify(&signature.as_ref(), &message.as_bytes())?);
    /// # Ok(())
    /// # }
    /// ```
    pub fn verify(&self, signature: &[u8], message: &[u8]) -> Result<(), Error> {
        let public_key = signature::UnparsedPublicKey::new(
            &signature::RSA_PSS_2048_8192_SHA256,
            self.keypair.public_key().as_ref(),
        );
        public_key.verify(message, signature)?;
        Ok(())
    }

    pub fn hash_sha256(&self, message: &[u8]) -> Result<[u8; 32], Error> {
        let mut context = Context::new(&SHA256);
        context.update(message);
        let mut result: [u8; 32] = [0; 32];
        result.copy_from_slice(context.finish().as_ref());
        Ok(result)
    }

    fn hash_sha384(&self, message: &[u8]) -> Result<[u8; 48], Error> {
        let mut context = Context::new(&SHA384);
        context.update(message);
        let mut result: [u8; 48] = [0; 48];
        result.copy_from_slice(context.finish().as_ref());
        Ok(result)
    }

    /// Returns a SHA256 hash of the the concatenated SHA256 hashes of a vector of messages.
    pub fn hash_all_sha256(&self, messages: Vec<&[u8]>) -> Result<[u8; 32], Error> {
        let hash: Vec<u8> = messages
            .into_iter()
            .map(|m| self.hash_sha256(m).unwrap())
            .into_iter()
            .flatten()
            .collect();
        let hash = self.hash_sha256(&hash)?;
        Ok(hash)
    }

    /// Returns a SHA384 hash of the the concatenated SHA384 hashes of a vector messages.
    fn hash_all_sha384(&self, messages: Vec<&[u8]>) -> Result<[u8; 48], Error> {
        let hash: Vec<u8> = messages
            .into_iter()
            .map(|m| self.hash_sha384(m).unwrap())
            .into_iter()
            .flatten()
            .collect();
        let hash = self.hash_sha384(&hash)?;
        Ok(hash)
    }

    /// Concatenates two `[u8; 48]` arrays, returning a `[u8; 96]` array.
    fn concat_u8_48(&self, left: [u8; 48], right: [u8; 48]) -> Result<[u8; 96], Error> {
        let mut iter = left.into_iter().chain(right);
        let result = [(); 96].map(|_| iter.next().unwrap());
        Ok(result)
    }

    /// Calculates data root of transaction in accordance with implementation in [arweave-js](https://github.com/ArweaveTeam/arweave-js/blob/master/src/common/lib/deepHash.ts).
    /// [`DeepHashItem`] is a recursive Enum that allows the function to be applied to
    /// nested [`Vec<u8>`] of arbitrary depth.
    pub fn deep_hash(&self, deep_hash_item: DeepHashItem) -> Result<[u8; 48], Error> {
        let hash = match deep_hash_item {
            DeepHashItem::Blob(blob) => {
                let blob_tag = format!("blob{}", blob.len());
                self.hash_all_sha384(vec![blob_tag.as_bytes(), &blob])?
            }
            DeepHashItem::List(list) => {
                let list_tag = format!("list{}", list.len());
                let mut hash = self.hash_sha384(list_tag.as_bytes())?;

                for child in list.into_iter() {
                    let child_hash = self.deep_hash(child)?;
                    hash = self.hash_sha384(&self.concat_u8_48(hash, child_hash)?)?;
                }
                hash
            }
        };
        Ok(hash)
    }

    pub fn fill_rand(&self, dest: &mut [u8]) -> Result<(), Error> {
        let rand_bytes = self.sr.fill(dest)?;
        Ok(rand_bytes)
    }
}

const DEFAULT_KEYPAIR: &str = r##"{
    "kty": "RSA",
    "n": "vUS-Urn9wBomxlKPhzZrjcsLZaGqPawdFRxHuy9sCUEF2zkRwbVLUf4vstz04Tis8tbd8TbGbmGxFxfybTFCEltwbfAMPmgAyvu4NZztkcFTg8XmsmADxPF5wOc0lpmwcSbec-r69_zNx6WXEM7qVng2nrufM_yR3ociBCSrG9_jnuhDaLxLayCkbD4gViNTIPPUJCQPCmy3PuRx-DITj7VFwi8u-KdWWjVN5cJ-pLLNKQjlpo0BOYMSc11S6N1s1Od6EG-LdL_gG1rfDX2hWzEtH2kHolN3UTSv1UU6980kG-e1BLIJHm7tHIBqxpwMR6m8HD6e3bDlcVQm23qxq6D3sIdauz4RNOl4yVFlI1o5tLeH_ot9uyWKkqGcknc4FgJ1CcVMwZsSl6S-BcTgZgns9AgfnJApZzWdyIpcyuqHBTaBOtcViGTupbn-LdY-lf1CwJZOgp5uDBFfU34ZhEcyCTLTEd5dCw9kQmO7TTqAJEO4kbtczxHUaNrAW8SViFNeG7SNlZ9uwqNMy7R1wswX_baarVjRzF3yUGkdSkzBMJfYs0lFLTiPY8gcuRsz03GNISi6AFuk25LhS19llIaz9-uucP8T0fnXzwHJqe85ygVLEOPcL72Z4VlRDvrdJMba4GKqcbwU5D17Q1lA9cPX7DmVtRJ7PCX2M_ezLQ0",
    "e": "AQAB",
    "d": "duxp1hstmPYVpQmdS61jGT4alCpniMbLo0cYv0IF1S65Gk0anidnA0b--5kgeR-edBuUawsq1ZKmrkcKuZd414YC9-EcIF5DGUffMDjBgZMDAcposW3pEGdWRGJCRdqd5gsxPY7JUObU-fxPFm2dCuYQE976IrUxhqxMMGRF64bbRC7WpEmj7dUd2zGSKe2aPxtWEbtig_9ZiLgL8JKufd69zUzOa8jhVl8l6hcychQzGvSPL_5rZZK5FinufYkb6A7mQMuFyb8Cds27V4O3zk_w9UqOVG2zjB_Z19zfN3L7nFkUAbZISooSjJUYAmFsyd6Z5vll4xBSqsngfIn0djkZBmkV2xhz8-DplEEgCyeEZ1FMsCLiwyHLWZb6e9h6sY_I6aBPaiPU1gLxUzrNM6mGQuuLcJP_6JdjWPmdlG39WGLdIfbFEqWKMBvP6QZIivWwiHVvETYJIGOnPNXTS2tQkC3XO-j8BpGqXLKvm5Tt1lj8RACuzCM0PreVtDyxyfb-DrHL0vb5MwYDSLFBTiy0IctSDDCio3mn0g5zffvc_RCzFcCiqf6x5S6WK2AYqZxRSPiyquCC3eQqDnnp896qkGSxdO1BhR78GvtUOv3qXjKuTsf2x_b4p0X8boZFqln3GjqgXimOT9AD9Cn0RrWGAu7DOKZuJCLJdYum6KE",
    "p": "5aA2SwF9tYQSEZP1ZTWnDqT16uFXEtAncaAyOpm7RR62wA8nQ9ecTgTvlXsFk8oJ8XTiaBn5GZNGnVybwpYwkpruHa36RSjjOkevETEOJtDFLQI7kk2lijG7ad22Sma4njhsPqE6sGJ4syTNlPNsLfUmma5nIJFe5C-mIcV36VJGxboNQnV9mlozQrS_pZWUPojjphzTH7eTrZ1KOsVQyC_SROVsNZLXWex_tJxQxfF2IKFQCMDFZC0BSvPboz_zXbCvf7jQsrt0WzYJDJNJgvWzQ85swB4tYQxiiOicFgwiALf_o63WNjdCJL40q-puKpbzurTav8xDK0KqhRUdlQ",
    "q": "0wHjX3cl6GUSv70oWrFaF-XLU_o3mONAmynidhFQd15--wOh7km4BW-4fFUr2FWM3I7-Ve9FFXcVVy8jSaDHpDWh1cr4-niwBGuO7sbqC7z1sjJ4AqhDt5XKTiYedEiWPeqT8XtLCj7I6HrS92yPNuUmnXjJnG_o8fEQzmAhkVuSaoKBqwSMwNNSRxWryDDvVayz84jwoVcFSwJvrxoPKyou35jtOLvGV1EV2DXM6ZDorWcADiovCrRVQShi0qrrEUMY-uI9Cw9o4AS2llsSe4NxdUKRSejeNm19bhRO2DuH-gGOC9DkfugAHey7iOLrLhKFvei7rTA6Wu7eG1zDmQ",
    "dp": "S30M_EGEOy0s53x1uw0VW3odolbsUjH-FZuth5hMeV-sgp04slPqfbefr8uevMQ52pgraj_HpYHGQCtWxXSsiTXHvBga46uab-lrA0LWPSp69935CZLfLfxFeXs612DHprQz2a8VZTEqLvKVZzdTRBSI2RL9sjY4NNn5SrbpQdobjBsrCsMnRJwMqAxVyLDQ6HIGLPDi81VdhkDkS0fc08Ls5Ftr5HzesSBPp2eQIlLMG9QMRKRjABjPiP18IkH-1rkkKN_wNCHuEaJE_U5aZ2Qwx8TP-aSyFGqG5i1aSuE4OHZE42Fdv7sQ0pV5KV9LUlMH00RreYxENK-Y8WFMtQ",
    "dq": "l1PeVkPkCuQZ6yrkuw5AV601AlgL8Xjhh6YlRJmsRL-ff7QeOP_jmvqBq6GFnVPVfwSKQOUlfXx28JzcyNwm8YyJMQOtRiyxx6m_y10a0ypEZvUs_nLghdRGT3-lDa5VGbiXO3M54PIgMiKMFGhl2W_EHuFWbfwQaxuA-xEUYePzgLFx_013CH9FnbdcCGmX67C9KeZG9N6s7BumL0UYJdPN5AwP7UU1vL9pVDNZbxS-2kVpU79LF3k3P1CQdxefGDUvwBXqw3jctPSMYg6UlcIx52_DNOduHkit0Pl9hjRDk7fzwGOiy6TlGJED-esL0XH1OrqjhlR1NWvkHGmN2Q",
    "qi": "QRTwMaZiz-IbZXr0bJCic6iHK1R4y2Yw-RVYCvFolVhyJORBVkqvu9XhJr1sRQlsqONSXa3T7hZwLi_vYhz2v5lKTdIy7aCW0M7HNc-MmpoEJckPJ5ps0gx5RhriK7dLWb4Jm9nixeyp19KPn-PKbo6pTszaaJGU_fG0r6jf8nAxBAT2nfHkB9SrqbDVko1gswFg8W_rqtesJHngqu-_RYSltkz6yzJ4zJZAyOyFlwwGyEnEPVwxWgy5oxuMPPTU5T0mBGWskDR1o4w78ZS42YLwAKQm48qfZmthTTHBnizW40AFMOJTFwEMZD1dV7YAMm1dQHO8ybQbZk1w7ybiiQ"
}"##;

#[cfg(test)]
mod tests {
    use super::Provider;
    use crate::{
        Arweave, Error,
        {transaction::Transaction, ToItems},
    };
    use std::path::PathBuf;
    use std::str::FromStr;
    use url::Url;

    #[tokio::test]
    async fn test_deep_hash() -> Result<(), Error> {
        let arweave = Arweave::from_keypair_path(
            PathBuf::from(
                "tests/fixtures/arweave-key-7eV1qae4qVNqsNChg3Scdi-DpOLJPCogct4ixoq1WNg.json",
            ),
            Url::from_str("http://url.com").unwrap(),
        )
        .await?;

        let transaction = Transaction {
            format: 2,
            ..Transaction::default()
        };
        let deep_hash = arweave.crypto.deep_hash(transaction.to_deep_hash_item()?)?;

        let correct_hash: [u8; 48] = [
            72, 43, 204, 204, 122, 20, 48, 138, 114, 252, 43, 128, 87, 244, 105, 231, 189, 246, 94,
            44, 150, 163, 165, 136, 133, 204, 158, 192, 28, 46, 222, 95, 55, 159, 23, 15, 3, 169,
            32, 27, 222, 153, 54, 137, 100, 159, 17, 247,
        ];

        assert_eq!(deep_hash, correct_hash);

        Ok(())
    }

    #[test]
    fn test_default_keypair() {
        let provider = Provider::default();
        assert_eq!(
            provider.wallet_address().unwrap().to_string(),
            "jA6UzKJ1cIvL2vUIct7Qf90QhC5b1UttvwknaGGBtjI"
        );
    }
}
