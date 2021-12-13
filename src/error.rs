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
    #[error("error getting arweave price: {0}")]
    ArweaveGetPriceError(reqwest::Error),
    #[error("error posting arweave transaction: {0}")]
    ArweavePostError(reqwest::Error),
    #[error("avro deserialize: {0}")]
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
    #[error("invalid bunlde item binary")]
    InvalidDataItem,
    #[error("hashing failed")]
    InvalidHash,
    #[error("invalid proof")]
    InvalidProof,
    #[error("invalid tags")]
    InvalidTags,
    #[error("insufficient sol funds")]
    InsufficientSolFunds,
    #[error("io: {0}")]
    IOError(#[from] std::io::Error),
    #[error("keypair not provided")]
    KeyPairNotProvided,
    #[error("key rejected: {0}")]
    KeyRejected(#[from] KeyRejected),
    #[error("manifest not found")]
    ManifestNotFound,
    #[error("file path not provided")]
    MissingFilePath,
    #[error("missing trailing slash")]
    MissingTrailingSlash,
    #[error("no bundle statuses found")]
    NoBundleStatusesFound,
    #[error("error getting oracle prices: {0}")]
    OracleGetPriceError(reqwest::Error),
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
    #[error("solana network error")]
    SolanaNetworkError,
    #[error("solana hash parse {0}")]
    TokioJoinError(#[from] tokio::task::JoinError),
    #[error("transaction is not signed")]
    UnsignedTransaction,
    #[error("url parse error: {0}")]
    UrlParse(#[from] ParseError),
}
