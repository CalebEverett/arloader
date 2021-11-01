//! Async [`TempDir`] for testing.

use crate::error::ArweaveError as Error;
use async_trait::async_trait;
use base64::{self, encode_config};
use ring::rand::{SecureRandom, SystemRandom};
use std::{fs as fsstd, path::PathBuf};
use tokio::fs;

pub struct TempDir(pub PathBuf);

#[async_trait]
pub trait TempFrom<T> {
    async fn from_str(path_str: &str) -> Result<T, Error>;
}

#[async_trait]
impl TempFrom<TempDir> for TempDir {
    async fn from_str(path_str: &str) -> Result<Self, Error> {
        if path_str.chars().last().unwrap() != '/' {
            return Err(Error::MissingTrailingSlash);
        }
        let rng = SystemRandom::new();
        let mut rand_bytes: [u8; 8] = [0; 8];
        let _ = rng.fill(&mut rand_bytes)?;
        let temp_stem = encode_config(rand_bytes, base64::URL_SAFE_NO_PAD);
        let path = PathBuf::from(path_str).join(temp_stem);
        fs::create_dir(&path).await?;
        Ok(Self(path))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        match fsstd::remove_dir_all(&self.0) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("{}", e);
            }
        }
    }
}
