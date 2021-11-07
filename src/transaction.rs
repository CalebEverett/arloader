//! Data structures for serializing and deserializing [`Transaction`]s and [`Tag`]s.

use crate::{
    error::Error,
    merkle::{Node, Proof},
};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;

/// Transaction data structure per [Arweave spec](https://docs.arweave.org/developers/server/http-api#transaction-format).
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Transaction {
    pub format: u8,
    pub id: Base64,
    pub last_tx: Base64,
    pub owner: Base64,
    pub tags: Vec<Tag>,
    pub target: Base64,
    #[serde(with = "stringify")]
    pub quantity: u64,
    pub data_root: Base64,
    pub data: Base64,
    #[serde(with = "stringify")]
    pub data_size: u64,
    #[serde(with = "stringify")]
    pub reward: u64,
    pub signature: Base64,
    #[serde(skip)]
    pub chunks: Vec<Node>,
    #[serde(skip)]
    pub proofs: Vec<Proof>,
}

/// Serializes and deserializes numbers represented as Strings. Used for `quantity`, `data_size`
/// and `reward` [`Transaction`] fields so that they can be represented as numbers but be serialized
/// to Strings as required by the Arweave spec.
pub mod stringify {
    use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: std::str::FromStr,
        <T as std::str::FromStr>::Err: std::fmt::Display,
    {
        String::deserialize(deserializer)?
            .parse::<T>()
            .map_err(|e| D::Error::custom(format!("{}", e)))
    }

    pub fn serialize<S, T>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: std::fmt::Display,
    {
        format!("{}", value).serialize(serializer)
    }
}

/// Implemented on [`Transaction`] to create root [`DeepHashItem`]s used by
/// [`crate::crypto::Methods::deep_hash`] in the creation of a transaction
/// signatures.
pub trait ToItems<'a, T> {
    fn to_deep_hash_item(&'a self) -> Result<DeepHashItem, Error>;
}

impl<'a> ToItems<'a, Transaction> for Transaction {
    fn to_deep_hash_item(&'a self) -> Result<DeepHashItem, Error> {
        match &self.format {
            1 => {
                let mut children: Vec<DeepHashItem> = vec![
                    &self.owner.0[..],
                    &self.target.0,
                    &self.data.0,
                    self.quantity.to_string().as_bytes(),
                    self.reward.to_string().as_bytes(),
                    &self.last_tx.0,
                ]
                .into_iter()
                .map(DeepHashItem::from_item)
                .collect();
                children.push(self.tags.to_deep_hash_item()?);

                Ok(DeepHashItem::from_children(children))
            }
            2 => {
                let mut children: Vec<DeepHashItem> = vec![
                    self.format.to_string().as_bytes(),
                    &self.owner.0,
                    &self.target.0,
                    self.quantity.to_string().as_bytes(),
                    self.reward.to_string().as_bytes(),
                    &self.last_tx.0,
                ]
                .into_iter()
                .map(DeepHashItem::from_item)
                .collect();
                children.push(self.tags.to_deep_hash_item()?);
                children.push(DeepHashItem::from_item(
                    self.data_size.to_string().as_bytes(),
                ));
                children.push(DeepHashItem::from_item(&self.data_root.0));

                Ok(DeepHashItem::from_children(children))
            }
            _ => unreachable!(),
        }
    }
}

/// Transaction tag.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tag {
    pub name: Base64,
    pub value: Base64,
}

/// Implemented as a convenience to create [`Tag`]s from name, value pairs of utf8 strings.
pub trait FromStrs<T> {
    fn from_utf8_strs(name: &str, value: &str) -> Result<T, Error>;
}

impl FromStrs<Tag> for Tag {
    fn from_utf8_strs(name: &str, value: &str) -> Result<Self, Error> {
        let b64_name = Base64::from_utf8_str(name)?;
        let b64_value = Base64::from_utf8_str(value)?;

        Ok(Self {
            name: b64_name,
            value: b64_value,
        })
    }
}

impl<'a> ToItems<'a, Vec<Tag>> for Vec<Tag> {
    fn to_deep_hash_item(&'a self) -> Result<DeepHashItem, Error> {
        if self.len() > 0 {
            Ok(DeepHashItem::List(
                self.iter()
                    .map(|t| t.to_deep_hash_item().unwrap())
                    .collect(),
            ))
        } else {
            Ok(DeepHashItem::Blob(Vec::<u8>::new()))
        }
    }
}

impl<'a> ToItems<'a, Tag> for Tag {
    fn to_deep_hash_item(&'a self) -> Result<DeepHashItem, Error> {
        Ok(DeepHashItem::List(vec![
            DeepHashItem::Blob(self.name.0.to_vec()),
            DeepHashItem::Blob(self.value.0.to_vec()),
        ]))
    }
}

/// A struct of [`Vec<u8>`] used for all data and address fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Base64(pub Vec<u8>);

impl Default for Base64 {
    fn default() -> Self {
        Base64(vec![])
    }
}

impl std::fmt::Display for Base64 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let string = &base64::display::Base64Display::with_config(&self.0, base64::URL_SAFE_NO_PAD);
        write!(f, "{}", string)
    }
}

/// Converts a base64url encoded string to a Base64 struct.
impl FromStr for Base64 {
    type Err = base64::DecodeError;
    fn from_str(str: &str) -> Result<Self, Self::Err> {
        let result = base64::decode_config(str, base64::URL_SAFE_NO_PAD)?;
        Ok(Self(result))
    }
}

/// Implemented on [`Base64`] to encode and decode utf8 strings.
pub trait ConvertUtf8<T> {
    fn from_utf8_str(str: &str) -> Result<T, Error>;
    fn to_utf8_string(&self) -> Result<String, Error>;
}

impl ConvertUtf8<Base64> for Base64 {
    fn from_utf8_str(str: &str) -> Result<Self, Error> {
        let enc_string = base64::encode_config(str.as_bytes(), base64::URL_SAFE_NO_PAD);
        let dec_bytes = base64::decode_config(enc_string, base64::URL_SAFE_NO_PAD)?;
        Ok(Self(dec_bytes))
    }
    fn to_utf8_string(&self) -> Result<String, Error> {
        let enc_string = base64::encode_config(&self.0, base64::URL_SAFE_NO_PAD);
        let dec_bytes = base64::decode_config(enc_string, base64::URL_SAFE_NO_PAD)?;
        Ok(String::from_utf8(dec_bytes)?)
    }
}

impl Serialize for Base64 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&format!("{}", &self))
    }
}

impl<'de> Deserialize<'de> for Base64 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Vis;
        impl serde::de::Visitor<'_> for Vis {
            type Value = Base64;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a base64 string")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                base64::decode_config(v, base64::URL_SAFE_NO_PAD)
                    .map(Base64)
                    .map_err(|_| de::Error::custom("failed to decode base64 string"))
            }
        }
        deserializer.deserialize_str(Vis)
    }
}

/// Recursive data structure that facilitates [`crate::crypto::Methods::deep_hash`] accepting nested
/// arrays of arbitrary depth as an argument with a single type.
#[derive(Debug, Clone, PartialEq)]
pub enum DeepHashItem {
    Blob(Vec<u8>),
    List(Vec<DeepHashItem>),
}

/// Implemented as a convenience to create [`DeepHashItem`]s.
pub trait FromItemOrChild<T> {
    fn from_item(item: &[u8]) -> Self;
    fn from_children(children: Vec<T>) -> Self;
}

impl FromItemOrChild<DeepHashItem> for DeepHashItem {
    fn from_item(item: &[u8]) -> DeepHashItem {
        Self::Blob(item.to_vec())
    }
    fn from_children(children: Vec<DeepHashItem>) -> DeepHashItem {
        Self::List(children)
    }
}

#[cfg(test)]
mod tests {
    use crate::transaction::FromStrs;

    use super::{Base64, ConvertUtf8, DeepHashItem, Error, Tag, ToItems};
    use serde_json;
    use std::str::FromStr;

    #[test]
    fn test_deserialize_base64() -> Result<(), Error> {
        let base_64 = Base64(vec![44; 7]);
        assert_eq!(base_64.0, vec![44; 7]);
        assert_eq!(format!("{}", base_64), "LCwsLCwsLA");

        let base_64: Base64 = serde_json::from_str("\"LCwsLCwsLA\"")?;
        assert_eq!(base_64.0, vec![44; 7]);
        assert_eq!(format!("{}", base_64), "LCwsLCwsLA");
        Ok(())
    }

    #[test]
    fn test_base64_convert_utf8() -> Result<(), Error> {
        let foo_b64 = Base64::from_utf8_str("foo")?;
        assert_eq!(foo_b64.0, vec![102, 111, 111]);

        let foo_b64 = Base64(vec![102, 111, 111]);
        assert_eq!(foo_b64.to_utf8_string()?, "foo".to_string());
        Ok(())
    }

    #[test]
    fn test_base64_convert_string() -> Result<(), Error> {
        let foo_b64 = Base64::from_str("LCwsLCwsLA")?;
        assert_eq!(foo_b64.0, vec![44; 7]);

        let foo_b64 = Base64(vec![44; 7]);
        assert_eq!(foo_b64.to_string(), "LCwsLCwsLA".to_string());
        Ok(())
    }

    #[test]
    fn test_tags_deep_hash_item2() -> Result<(), Error> {
        let tags = vec![
            Tag::from_utf8_strs("Content-Type", "text/html")?,
            Tag::from_utf8_strs("key2", "value2")?,
        ];

        assert_eq!("Content-Type".to_string(), tags[0].name.to_utf8_string()?);
        assert_eq!("Q29udGVudC1UeXBl".to_string(), tags[0].name.to_string());

        let deep_hash_item = tags.to_deep_hash_item()?;

        let deep_hash_item_actual = DeepHashItem::List(vec![
            DeepHashItem::List(vec![
                DeepHashItem::Blob(vec![
                    67, 111, 110, 116, 101, 110, 116, 45, 84, 121, 112, 101,
                ]),
                DeepHashItem::Blob(vec![116, 101, 120, 116, 47, 104, 116, 109, 108]),
            ]),
            DeepHashItem::List(vec![
                DeepHashItem::Blob(vec![107, 101, 121, 50]),
                DeepHashItem::Blob(vec![118, 97, 108, 117, 101, 50]),
            ]),
        ]);

        assert_eq!(deep_hash_item, deep_hash_item_actual);
        Ok(())
    }
}
