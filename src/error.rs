//! Errors propagated by library functions.
use base64::DecodeError;
use glob;
use reqwest;
use ring::error::{KeyRejected, Unspecified};
use serde_json;
use std::string::FromUtf8Error;
use thiserror::Error;
use url::ParseError;

/// Errors propagated by library functions.
#[derive(Error, Debug)]
pub enum Error {
    #[error("avro desserialize: {0}")]
    AvroDeError(#[from] avro_rs::DeError),
    #[error("base64 decode: {0}")]
    Base64Decode(#[from] DecodeError),
    #[error("bincode: {0}")]
    Bincode(#[from] Box<bincode::ErrorKind>),
    #[error("unhandled boxed dyn error {0}")]
    BoxedDynStd(#[from] Box<dyn std::error::Error>),
    #[error("formatting error")]
    FormatError(#[from] std::fmt::Error),
    #[error("from utf8: {0}")]
    FromUtf8(#[from] FromUtf8Error),
    #[error("glob patters: {0}")]
    GlobPattern(#[from] glob::PatternError),
    #[error("hashing failed")]
    InvalidHash,
    #[error("invalid proof")]
    InvalidProof,
    #[error("tags could not be parsed to slices")]
    InvalidTags,
    #[error("io: {0}")]
    IOError(#[from] std::io::Error),
    #[error("key rejected: {0}")]
    KeyRejected(#[from] KeyRejected),
    #[error("file path not provided")]
    MissingFilePath,
    #[error("missing trailing slash")]
    MissingTrailingSlash,
    #[error("reqwest: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("ring unspecified: {0}")]
    RingUnspecified(#[from] Unspecified),
    #[error("serde json: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("status not found")]
    StatusNotFound,
    #[error("solana hash parse {0}")]
    SolanaHashParse(#[from] solana_sdk::hash::ParseHashError),
    #[error("transaction is not signed")]
    UnsignedTransaction,
    #[error("url parse error: {0}")]
    UrlParse(#[from] ParseError),
}
